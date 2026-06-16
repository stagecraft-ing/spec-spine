//! The attest capability (spec 023): a pure, reproducible attestation over the
//! spec corpus state.
//!
//! `attest` freezes a verdict spec-spine already computes (`compile`, `lint`,
//! and optionally `couple`) into a [`CorpusAttestation`]: a pure function of
//! `(config, file contents)` with no clock, no env, and **no key**. Re-running
//! it on an unchanged corpus at the same tool version yields a byte-identical
//! payload, which is exactly what makes `verify_recompute` runnable by any third
//! party with no key and no trust in the signer. Signing is a separate, key-only
//! post-pass that lives in the CLI (`seal.rs`); this module never touches a key.

use std::path::Path;

use sha2::{Digest, Sha256};
use spec_spine_types::{
    ATTESTATION_SCHEMA_VERSION, CompileVerdict, CorpusAttestation, CoupleVerdict, Error,
    LintVerdict, Severity, ToolStamp, Verdicts,
};

use crate::canonical_json;
use crate::compile::compile;
use crate::index::index;
use crate::lint::lint;

/// Resolver diagnostics that mark code as out of sync with its claiming spec.
/// A **local mirror** of `index.rs::BLOCKING_CODES` (kept equal by value and
/// this comment, not by linkage, so the attest capability does not take a code
/// dependency on the indexer's private constant): the same set that
/// `check_index_freshness` treats as a hard failure.
const BLOCKING_RESOLVER_CODES: &[&str] = &[
    "I-003", "I-004", "I-005", "I-006", "I-007", "I-008", "I-009",
];

/// Inputs to a [`attest`] run beyond `(config, file contents)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttestOptions {
    /// Record the coupling (specs-and-code-in-sync) verdict as well (FR-002).
    /// Without it the attestation covers spec-corpus state only.
    pub with_coupling: bool,
}

/// The result of an [`attest`] run: the typed attestation, its canonical JSON,
/// and the hash a consumer references and a seal signs.
pub struct AttestOutcome {
    pub attestation: CorpusAttestation,
    /// Canonical `CorpusAttestation` JSON (sorted keys, 2-space, trailing LF).
    pub json: String,
    /// SHA-256 (lowercase hex) over [`AttestOutcome::json`]. Emitted alongside,
    /// never inside, the attestation: it is the chain handle a consumer
    /// references and the message a [`crate::types::LedgerSeal`] signs.
    pub attestation_hash: String,
}

/// The outcome of a `--recompute` verification (FR-004/FR-005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// The recomputed attestation is byte-identical to the supplied one.
    Match,
    /// The attestation was produced by a different tool version; recompute is
    /// not meaningful. A distinct, named outcome: never a false content
    /// mismatch, never a skip-as-pass (FR-005).
    VersionMismatch { expected: String, actual: String },
    /// The corpus recomputes to a different attestation. Each entry names a
    /// field that diverged (FR-004).
    ContentMismatch { differences: Vec<String> },
}

/// Build a [`CorpusAttestation`] over the corpus under `repo_root`.
///
/// Pure function of `(config, file contents)`: it runs `compile` and `lint`
/// (and `index` under `--with-coupling`), all themselves pure, and hashes their
/// canonical outputs. No clock, no env, no key.
pub fn attest(
    cfg: &spec_spine_types::Config,
    repo_root: &Path,
    opts: AttestOptions,
) -> Result<AttestOutcome, Error> {
    // compile: the inputs-manifest and registry hashes plus the compile verdict.
    let compiled = compile(cfg, repo_root)?;
    let inputs_manifest_hash = compiled.registry.build.content_hash.clone();
    let registry_hash = sha256_hex(compiled.json.as_bytes());
    let compile_ok = compiled.validation_passed;

    // lint: the verdict and a hash over the canonical findings. `ok` mirrors the
    // repo's own `lint --fail-on-warn` gate (no error and no warning; info-tier
    // is advisory), which is the meaningful "corpus is consistent" claim for an
    // audit attestation. `findings_hash` captures every finding (including info),
    // so any change in the findings set is detectable on recompute.
    let lint_report = lint(cfg, repo_root)?;
    let lint_ok =
        lint_report.count(Severity::Error) == 0 && lint_report.count(Severity::Warning) == 0;
    let findings_hash = sha256_hex(canonical_json::to_string(&lint_report.violations)?.as_bytes());

    // couple (optional, FR-002): specs and code are in sync iff the index built
    // with no blocking resolver diagnostic (every claimed unit resolves). Pure:
    // no git diff is needed, so the core stays git-free.
    let couple = if opts.with_coupling {
        let outcome = index(cfg, repo_root)?;
        let index_hash = outcome.index.build.content_hash.clone();
        let blocking = outcome
            .index
            .diagnostics
            .errors
            .iter()
            .any(|d| BLOCKING_RESOLVER_CODES.contains(&d.code.as_str()));
        let join_hash = sha256_hex(format!("{registry_hash}:{index_hash}").as_bytes());
        Some(CoupleVerdict {
            ok: !blocking,
            index_hash,
            join_hash,
        })
    } else {
        None
    };

    let attestation = CorpusAttestation {
        schema_version: ATTESTATION_SCHEMA_VERSION.to_string(),
        tool: ToolStamp {
            name: cfg.branding.compiler_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        inputs_manifest_hash,
        registry_hash,
        verdicts: Verdicts {
            compile: CompileVerdict { ok: compile_ok },
            lint: LintVerdict {
                ok: lint_ok,
                findings_hash,
            },
            couple,
        },
    };

    let json = canonical_json::to_string(&attestation)?;
    let hash = sha256_hex(json.as_bytes());
    Ok(AttestOutcome {
        attestation,
        json,
        attestation_hash: hash,
    })
}

/// The hash a seal signs and a consumer references: SHA-256 (lowercase hex) over
/// the canonical JSON of `attestation`. Recomputing it from a *loaded* payload
/// is what keeps a signature check tamper-evident: a single changed byte changes
/// the canonical bytes and therefore the hash, so the seal no longer verifies.
pub fn attestation_hash(attestation: &CorpusAttestation) -> Result<String, Error> {
    Ok(sha256_hex(
        canonical_json::to_string(attestation)?.as_bytes(),
    ))
}

/// Re-read the corpus and verify it still recomputes to `attestation`
/// (FR-004 `--recompute`). Version-aware (FR-005): if the attestation's
/// `tool.version` differs from this build's, the result is
/// [`VerifyOutcome::VersionMismatch`], not a content mismatch. The recompute
/// scope mirrors the attestation (coupling is recomputed iff the attestation
/// carries a coupling block).
pub fn verify_recompute(
    cfg: &spec_spine_types::Config,
    repo_root: &Path,
    attestation: &CorpusAttestation,
) -> Result<VerifyOutcome, Error> {
    let actual_version = env!("CARGO_PKG_VERSION");
    if attestation.tool.version != actual_version {
        return Ok(VerifyOutcome::VersionMismatch {
            expected: attestation.tool.version.clone(),
            actual: actual_version.to_string(),
        });
    }

    let recomputed = attest(
        cfg,
        repo_root,
        AttestOptions {
            with_coupling: attestation.verdicts.couple.is_some(),
        },
    )?
    .attestation;

    let mut differences = Vec::new();
    let a = attestation;
    let b = &recomputed;
    if a.tool.name != b.tool.name {
        differences.push(format!("tool.name ({} -> {})", a.tool.name, b.tool.name));
    }
    if a.inputs_manifest_hash != b.inputs_manifest_hash {
        differences.push("inputsManifestHash (corpus inputs changed)".to_string());
    }
    if a.registry_hash != b.registry_hash {
        differences.push("registryHash (compiled registry changed)".to_string());
    }
    if a.verdicts.compile.ok != b.verdicts.compile.ok {
        differences.push(format!(
            "verdicts.compile.ok ({} -> {})",
            a.verdicts.compile.ok, b.verdicts.compile.ok
        ));
    }
    if a.verdicts.lint.ok != b.verdicts.lint.ok {
        differences.push(format!(
            "verdicts.lint.ok ({} -> {})",
            a.verdicts.lint.ok, b.verdicts.lint.ok
        ));
    }
    if a.verdicts.lint.findings_hash != b.verdicts.lint.findings_hash {
        differences.push("verdicts.lint.findingsHash (lint findings changed)".to_string());
    }
    match (&a.verdicts.couple, &b.verdicts.couple) {
        (Some(av), Some(bv)) => {
            if av.ok != bv.ok {
                differences.push(format!("verdicts.couple.ok ({} -> {})", av.ok, bv.ok));
            }
            if av.index_hash != bv.index_hash {
                differences.push("verdicts.couple.indexHash (code index changed)".to_string());
            }
            if av.join_hash != bv.join_hash {
                differences.push("verdicts.couple.joinHash".to_string());
            }
        }
        (None, None) => {}
        // Scope is recomputed from the attestation, so this is unreachable in
        // practice; recorded as a difference rather than panicking.
        _ => differences.push("verdicts.couple (scope presence changed)".to_string()),
    }

    if differences.is_empty() {
        Ok(VerifyOutcome::Match)
    } else {
        Ok(VerifyOutcome::ContentMismatch { differences })
    }
}

/// SHA-256 (lowercase hex) over raw bytes. (Distinct from
/// `hash::content_hash`, which hashes path-keyed, normalized input pieces; here
/// the inputs are already-canonical bytes.)
fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_is_lowercase_64_hex() {
        let h = sha256_hex(b"abc");
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(h.len(), 64);
    }
}
