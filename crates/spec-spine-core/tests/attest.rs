//! Attest (spec 023): determinism (AC-1), recompute match/mismatch (AC-2), the
//! coupling scope and its independently-checkable verdict (AC-3), and
//! version-aware verification (AC-5). The signature round-trip (AC-4) lives in
//! the CLI crate's `seal.rs` unit tests: the core is key-free.

use std::fs;
use std::path::Path;

use spec_spine_core::{AttestOptions, VerifyOutcome, attest, verify_recompute};
use spec_spine_types::Config;

/// Write `specs/<id>/spec.md` under `root` with the given extra frontmatter.
fn write_spec(root: &Path, dir: &str, id: &str, extra: &str) {
    let spec_dir = root.join("specs").join(dir);
    fs::create_dir_all(&spec_dir).unwrap();
    let body = format!(
        "---\nid: \"{id}\"\ntitle: \"Title {id}\"\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: \"s\"\n{extra}---\n# {id}\n"
    );
    fs::write(spec_dir.join("spec.md"), body).unwrap();
}

/// An ownership edge keeps the spec warning-clean (no L-001), so the lint
/// verdict is `ok` under the `--fail-on-warn`-equivalent rule. The claimed file
/// need not exist for the non-coupling cases (compile/lint do not check units).
const OWNED: &str = "establishes:\n  - \"code.txt\"\n";

#[test]
fn ac1_attest_is_byte_identical_on_an_unchanged_corpus() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", OWNED);
    let cfg = Config::default();

    let a = attest(&cfg, tmp.path(), AttestOptions::default()).unwrap();
    let b = attest(&cfg, tmp.path(), AttestOptions::default()).unwrap();

    assert_eq!(a.json, b.json, "attestation must be byte-identical");
    assert_eq!(a.attestation_hash, b.attestation_hash);
    assert!(
        a.attestation.verdicts.couple.is_none(),
        "no coupling block without --with-coupling"
    );
    assert!(a.attestation.verdicts.compile.ok);
    assert!(a.attestation.verdicts.lint.ok);
    assert!(a.json.ends_with("}\n"), "canonical trailing newline");
}

#[test]
fn ac2_recompute_matches_unchanged_then_flags_an_edited_spec() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", OWNED);
    let cfg = Config::default();
    let outcome = attest(&cfg, tmp.path(), AttestOptions::default()).unwrap();

    assert_eq!(
        verify_recompute(&cfg, tmp.path(), &outcome.attestation).unwrap(),
        VerifyOutcome::Match,
        "an unchanged corpus reproduces the attestation"
    );

    // A single spec edit flips it to a named mismatch citing the changed input.
    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        &format!("{OWNED}owner: \"someone\"\n"),
    );
    match verify_recompute(&cfg, tmp.path(), &outcome.attestation).unwrap() {
        VerifyOutcome::ContentMismatch { differences } => assert!(
            differences
                .iter()
                .any(|d| d.contains("inputsManifestHash") || d.contains("registryHash")),
            "mismatch must cite the changed input/registry: {differences:?}"
        ),
        other => panic!("expected ContentMismatch, got {other:?}"),
    }
}

#[test]
fn ac5_a_different_tool_version_is_a_named_version_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", OWNED);
    let cfg = Config::default();
    let mut attestation = attest(&cfg, tmp.path(), AttestOptions::default())
        .unwrap()
        .attestation;

    attestation.tool.version = "0.0.1-other".to_string();
    match verify_recompute(&cfg, tmp.path(), &attestation).unwrap() {
        VerifyOutcome::VersionMismatch { expected, actual } => {
            assert_eq!(expected, "0.0.1-other");
            assert_ne!(actual, "0.0.1-other", "actual is this build's version");
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

#[test]
fn ac3_with_coupling_verdict_is_independently_checkable() {
    let tmp = tempfile::tempdir().unwrap();
    // A spec that claims a code unit via a file establishes edge.
    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        "establishes:\n  - \"code.txt\"\n",
    );
    fs::write(tmp.path().join("code.txt"), "fn main() {}\n").unwrap();
    let cfg = Config::default();

    // In sync: the claimed unit resolves, so the coupling verdict is ok.
    let synced = attest(
        &cfg,
        tmp.path(),
        AttestOptions {
            with_coupling: true,
        },
    )
    .unwrap();
    let couple = synced
        .attestation
        .verdicts
        .couple
        .as_ref()
        .expect("coupling block present under --with-coupling");
    assert!(couple.ok, "a resolving claim is in sync");
    assert!(!couple.index_hash.is_empty());
    assert!(!couple.join_hash.is_empty());

    // Remove the claimed code: the coupling block fails while the
    // registry/lint scope still verifies (the two scopes are independent).
    fs::remove_file(tmp.path().join("code.txt")).unwrap();
    let drifted = attest(
        &cfg,
        tmp.path(),
        AttestOptions {
            with_coupling: true,
        },
    )
    .unwrap();
    let couple = drifted.attestation.verdicts.couple.as_ref().unwrap();
    assert!(!couple.ok, "a missing claimed unit is not in sync");
    assert!(drifted.attestation.verdicts.compile.ok);
    assert!(drifted.attestation.verdicts.lint.ok);
}
