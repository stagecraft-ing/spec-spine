//! Scaffold tests (spec 006): the generated corpus is well-formed — a scaffolded
//! repo compiles and lints clean, proving the adoption loop works with zero
//! library edits.

use std::fs;

use spec_spine_core::{compile, lint, scaffold_init};
use spec_spine_types::Config;

/// Write a [`Scaffold`] to a temp dir as the CLI would.
fn materialize(cfg: &Config) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let scaffold = scaffold_init(cfg).unwrap();
    for f in &scaffold.files {
        let abs = tmp.path().join(&f.rel_path);
        fs::create_dir_all(abs.parent().unwrap()).unwrap();
        fs::write(&abs, &f.contents).unwrap();
    }
    tmp
}

#[test]
fn scaffolded_corpus_compiles_and_lints_clean() {
    let cfg = Config::default();
    let repo = materialize(&cfg);

    let outcome = compile(&cfg, repo.path()).unwrap();
    assert!(
        outcome.registry.validation.passed,
        "scaffolded corpus must compile clean: {:?}",
        outcome.registry.validation.violations
    );
    assert!(
        outcome
            .registry
            .specs
            .iter()
            .any(|s| s.id == "000-bootstrap"),
        "the bootstrap spec is present"
    );

    // The bootstrap spec is retroactive, so it raises no L-001 (no-edge) warning.
    let report = lint(&cfg, repo.path()).unwrap();
    assert!(
        !report.violations.iter().any(|v| v.code == "L-001"),
        "retroactive bootstrap should not trip L-001: {:?}",
        report.violations
    );
}

#[test]
fn non_default_namespace_scaffolds_coherently() {
    let mut cfg = Config::default();
    cfg.manifest.metadata_namespace = "acme".to_string();
    cfg.layout.specs_dir = "contracts".to_string();
    let repo = materialize(&cfg);

    // The bootstrap spec landed under the configured specs dir and compiles.
    assert!(
        repo.path()
            .join("contracts/000-bootstrap/spec.md")
            .is_file()
    );
    let outcome = compile(&cfg, repo.path()).unwrap();
    assert!(outcome.registry.validation.passed);
}
