//! `spec-spine verify-attestation`: two independent verification modes (spec 023
//! FR-004), either or both selectable in one invocation.
//!
//! `--recompute` re-reads the corpus and checks it reproduces the attestation:
//! no key, no signature, offline, runnable by any third party. It is the load
//! bearing property the run certificate structurally cannot have. `--signature`
//! checks the detached seal against a supplied public key. A mode that cannot
//! run fails visibly (FR-006); skip-as-pass is forbidden.

use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_core::{VerifyOutcome, attestation_hash, verify_recompute};
use spec_spine_types::{Config, CorpusAttestation, Error, LedgerSeal};

use crate::load_repo_config;
use crate::seal;

/// Parsed `verify-attestation` arguments.
pub struct VerifyArgs {
    pub recompute: bool,
    pub signature: bool,
    pub attestation: Option<PathBuf>,
    pub public_key: Option<PathBuf>,
    pub seal: Option<PathBuf>,
}

/// Exit `0` only if every selected mode passes; `1` on any mismatch or version
/// mismatch (a named, non-pass outcome). A missing mode or missing key is a
/// visible config error (exit 3), never a silent pass.
pub fn run(repo: &Path, args: &VerifyArgs) -> Result<u8, Error> {
    if !args.recompute && !args.signature {
        return Err(Error::Config(
            "verify-attestation requires at least one mode: --recompute and/or --signature"
                .to_string(),
        ));
    }

    let cfg = load_repo_config(repo)?;
    let attestation_path = args
        .attestation
        .clone()
        .unwrap_or_else(|| default_attestation_path(repo, &cfg));
    let attestation = load_attestation(&attestation_path)?;

    let mut failed = false;

    if args.recompute {
        match verify_recompute(&cfg, repo, &attestation)? {
            VerifyOutcome::Match => {
                println!("recompute: MATCH (the corpus reproduces this attestation)");
            }
            VerifyOutcome::VersionMismatch { expected, actual } => {
                eprintln!(
                    "recompute: VERSION MISMATCH (attested under {expected}, this tool is {actual}); \
                     recompute under {expected} to verify"
                );
                failed = true;
            }
            VerifyOutcome::ContentMismatch { differences } => {
                eprintln!(
                    "recompute: CONTENT MISMATCH ({} field(s) diverged):",
                    differences.len()
                );
                for d in &differences {
                    eprintln!("  - {d}");
                }
                failed = true;
            }
        }
    }

    if args.signature {
        let pk_path = args.public_key.as_ref().ok_or_else(|| {
            Error::Config(
                "verify-attestation --signature requires --public-key <path> (a 32-byte ed25519 public key)"
                    .to_string(),
            )
        })?;
        let verifying_key = seal::load_verifying_key(pk_path)?;
        let seal_path = args
            .seal
            .clone()
            .unwrap_or_else(|| attestation_path.with_file_name("attestation.sig"));
        let ledger_seal = load_seal(&seal_path)?;
        // Recompute the hash from the loaded payload: a tampered byte changes it
        // and the seal stops verifying.
        let hash = attestation_hash(&attestation)?;
        if seal::verify(&hash, &ledger_seal, &verifying_key)? {
            println!("signature: VALID (sealed by keyId {})", ledger_seal.key_id);
        } else {
            eprintln!(
                "signature: INVALID (the seal does not verify against the supplied public key)"
            );
            failed = true;
        }
    }

    Ok(if failed { 1 } else { 0 })
}

fn default_attestation_path(repo: &Path, cfg: &Config) -> PathBuf {
    repo.join(&cfg.layout.derived_dir)
        .join("attestation")
        .join("attestation.json")
}

fn load_attestation(path: &Path) -> Result<CorpusAttestation, Error> {
    let bytes = fs::read(path).map_err(|e| {
        Error::Io(format!(
            "read attestation {} (run `spec-spine attest` first?): {e}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|e| Error::Parse(format!("invalid attestation {}: {e}", path.display())))
}

fn load_seal(path: &Path) -> Result<LedgerSeal, Error> {
    let bytes = fs::read(path).map_err(|e| {
        Error::Io(format!(
            "read seal {} (run `spec-spine attest --sign` first?): {e}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|e| Error::Parse(format!("invalid seal {}: {e}", path.display())))
}
