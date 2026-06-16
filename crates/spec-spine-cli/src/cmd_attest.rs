//! `spec-spine attest`: emit a reproducible corpus attestation under
//! `<derived_dir>/attestation/`, and optionally seal it (spec 023).
//!
//! The attestation itself is pure (built in `spec-spine-core::attest`); this
//! command is the IO + clock shell: it writes the artifact and, under `--sign`,
//! the wall-clock-dated detached seal.

use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_core::{AttestOptions, attest};
use spec_spine_types::Error;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::load_repo_config;
use crate::seal;

/// Parsed `attest` arguments.
pub struct AttestArgs {
    pub with_coupling: bool,
    pub sign: bool,
    pub key: Option<PathBuf>,
    pub key_id: Option<String>,
}

/// Writes `attestation.json` (always) and `attestation.sig` (under `--sign`).
/// Exit `0` on success; a `--sign` with no `--key` is a visible config error
/// (FR-006: a mode that cannot run fails, never skip-as-pass).
pub fn run(repo: &Path, args: &AttestArgs) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;

    // FR-006 (fail-closed, no side effects on a usage error): when signing,
    // resolve the key BEFORE building or writing the attestation, so a missing
    // or invalid key fails before any artifact lands on disk.
    let signer = if args.sign {
        let key_path = args.key.as_ref().ok_or_else(|| {
            Error::Config(
                "attest --sign requires --key <path> (a 32-byte ed25519 signing key, raw or hex)"
                    .to_string(),
            )
        })?;
        let signing_key = seal::load_signing_key(key_path)?;
        let key_id = args
            .key_id
            .clone()
            .unwrap_or_else(|| seal::default_key_id(&signing_key));
        Some((signing_key, key_id))
    } else {
        None
    };

    let outcome = attest(
        &cfg,
        repo,
        AttestOptions {
            with_coupling: args.with_coupling,
        },
    )?;

    let out_dir = repo.join(&cfg.layout.derived_dir).join("attestation");
    fs::create_dir_all(&out_dir)
        .map_err(|e| Error::Io(format!("create {}: {e}", out_dir.display())))?;
    let attestation_path = out_dir.join("attestation.json");
    fs::write(&attestation_path, &outcome.json)
        .map_err(|e| Error::Io(format!("write {}: {e}", attestation_path.display())))?;

    let scope = if args.with_coupling {
        "specs+code"
    } else {
        "spec-corpus"
    };
    println!("attested {scope} -> {}", attestation_path.display());
    println!("  attestationHash: {}", outcome.attestation_hash);

    if let Some((signing_key, key_id)) = signer {
        let ledger_seal = seal::sign(
            &outcome.attestation_hash,
            &signing_key,
            key_id,
            now_rfc3339(),
        )?;
        let seal_json = serde_json::to_string_pretty(&ledger_seal)
            .map_err(|e| Error::Schema(e.to_string()))?
            + "\n";
        let seal_path = out_dir.join("attestation.sig");
        fs::write(&seal_path, seal_json)
            .map_err(|e| Error::Io(format!("write {}: {e}", seal_path.display())))?;
        println!(
            "sealed -> {} (alg ed25519, keyId {})",
            seal_path.display(),
            ledger_seal.key_id
        );
    }

    Ok(0)
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}
