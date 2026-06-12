//! Compile: determinism, the V-code validations, the extra_frontmatter copy,
//! and content-hash sensitivity.

use std::fs;
use std::path::Path;

use spec_spine_core::compile;
use spec_spine_core::compile::MAX_UNDECLARED_EXTRA_FRONTMATTER;
use spec_spine_types::{Config, Severity};

/// Write `specs/<id>/spec.md` under `root` with the given extra frontmatter lines.
fn write_spec(root: &Path, dir: &str, id: &str, extra: &str) {
    let spec_dir = root.join("specs").join(dir);
    fs::create_dir_all(&spec_dir).unwrap();
    let body = format!(
        "---\nid: \"{id}\"\ntitle: \"Title {id}\"\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: \"s\"\n{extra}---\n# {id}\n"
    );
    fs::write(spec_dir.join("spec.md"), body).unwrap();
}

fn codes(outcome: &spec_spine_core::CompileOutcome) -> Vec<String> {
    outcome
        .registry
        .validation
        .violations
        .iter()
        .map(|v| v.code.clone())
        .collect()
}

#[test]
fn compiles_clean_corpus_deterministically() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-alpha", "001-alpha", "");
    write_spec(tmp.path(), "002-beta", "002-beta", "");
    let cfg = Config::default();

    let first = compile(&cfg, tmp.path()).unwrap();
    let second = compile(&cfg, tmp.path()).unwrap();

    assert!(first.validation_passed);
    assert_eq!(first.json, second.json, "compile must be byte-identical");
    assert_eq!(first.registry.specs.len(), 2);
    // Specs are sorted by id.
    assert_eq!(first.registry.specs[0].id, "001-alpha");
    assert_eq!(first.registry.specs[1].id, "002-beta");
    assert_eq!(
        first.registry.spec_version,
        spec_spine_core::REGISTRY_SCHEMA_VERSION
    );
    // Trailing newline + sorted keys (canonical form).
    assert!(first.json.ends_with("}\n"));
}

#[test]
fn content_hash_changes_with_content_but_is_stable_otherwise() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "");
    let cfg = Config::default();
    let h1 = compile(&cfg, tmp.path())
        .unwrap()
        .registry
        .build
        .content_hash;

    // Re-compile unchanged -> same hash.
    let h1b = compile(&cfg, tmp.path())
        .unwrap()
        .registry
        .build
        .content_hash;
    assert_eq!(h1, h1b);

    // Change content -> different hash.
    write_spec(tmp.path(), "001-a", "001-a", "owner: \"someone\"\n");
    let h2 = compile(&cfg, tmp.path())
        .unwrap()
        .registry
        .build
        .content_hash;
    assert_ne!(h1, h2);
    assert_eq!(h2.len(), 64);
}

#[test]
fn extra_frontmatter_is_copied_into_the_registry() {
    // The overlay seam depends on this reaching registry.json (Phase-1 item 1).
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "x_overlay_key: \"carried\"\n");
    let outcome = compile(&Config::default(), tmp.path()).unwrap();

    let spec = &outcome.registry.specs[0];
    assert!(
        spec.extra_frontmatter.contains_key("x_overlay_key"),
        "extra_frontmatter must survive into SpecRecord"
    );
    assert!(
        outcome.json.contains("x_overlay_key"),
        "extra_frontmatter must serialize into registry.json"
    );
}

#[test]
fn v001_directory_must_equal_id() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-folder", "001-different", "");
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-001".to_string()));
    assert!(!outcome.validation_passed);
}

#[test]
fn v003_duplicate_id() {
    // Two directories declaring the same id.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-x", "001-dup", "");
    write_spec(tmp.path(), "002-y", "001-dup", "");
    let c = codes(&compile(&Config::default(), tmp.path()).unwrap());
    assert!(c.contains(&"V-003".to_string()), "duplicate id: {c:?}");
}

#[test]
fn v004_duplicate_prefix() {
    // Two different slugs sharing the numeric prefix.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-alpha", "001-alpha", "");
    write_spec(tmp.path(), "001-beta", "001-beta", "");
    let c = codes(&compile(&Config::default(), tmp.path()).unwrap());
    assert!(c.contains(&"V-004".to_string()), "duplicate prefix: {c:?}");
    assert!(!c.contains(&"V-003".to_string()), "ids are distinct: {c:?}");
}

#[test]
fn v002_malformed_frontmatter_is_recorded_not_fatal() {
    let tmp = tempfile::tempdir().unwrap();
    // Missing required `summary`.
    let spec_dir = tmp.path().join("specs").join("001-bad");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        "---\nid: \"001-bad\"\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\n---\n",
    )
    .unwrap();
    // A valid one alongside it.
    write_spec(tmp.path(), "002-ok", "002-ok", "");

    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-002".to_string()));
    // The valid spec still made it into the registry.
    assert!(outcome.registry.specs.iter().any(|s| s.id == "002-ok"));
    assert!(!outcome.validation_passed);
}

#[test]
fn v007_extra_frontmatter_count_cap_with_exemption() {
    let tmp = tempfile::tempdir().unwrap();
    let mut extra = String::new();
    let n = MAX_UNDECLARED_EXTRA_FRONTMATTER + 1;
    for i in 0..n {
        extra.push_str(&format!("x_key_{i}: {i}\n"));
    }
    write_spec(tmp.path(), "001-a", "001-a", &extra);

    // Undeclared -> V-007.
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-007".to_string()));

    // Declaring the keys in extra_known_keys exempts them from the cap.
    let mut cfg = Config::default();
    cfg.frontmatter.extra_known_keys = (0..n).map(|i| format!("x_key_{i}")).collect();
    let outcome = compile(&cfg, tmp.path()).unwrap();
    assert!(
        !codes(&outcome).contains(&"V-007".to_string()),
        "declared keys are exempt"
    );
}

#[test]
fn declared_nested_extra_roundtrips_deterministically() {
    // Spec 013 §3.5: a compliance-shaped declared key survives compile ->
    // registry byte-identically across two runs.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        "compliance:\n  reviewed: true\n  owasp:\n    - \"A01\"\n    - \"A03\"\n",
    );
    let mut cfg = Config::default();
    cfg.frontmatter.extra_known_keys = vec!["compliance".into()];

    let first = compile(&cfg, tmp.path()).unwrap();
    let second = compile(&cfg, tmp.path()).unwrap();
    assert!(first.validation_passed);
    assert_eq!(first.json, second.json, "byte-identical across two runs");
    assert_eq!(
        first.registry.specs[0].extra_frontmatter.get("compliance"),
        Some(&serde_json::json!({"owasp": ["A01", "A03"], "reviewed": true}))
    );
}

#[test]
fn declared_map_key_order_is_canonicalized() {
    // Spec 013 §3.2/§3.5: two authoring orders, one registry value.
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = Config::default();
    cfg.frontmatter.extra_known_keys = vec!["compliance".into()];

    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        "compliance:\n  zz: 1\n  aa: 2\n",
    );
    let one = compile(&cfg, tmp.path()).unwrap().registry.specs[0]
        .extra_frontmatter
        .clone();
    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        "compliance:\n  aa: 2\n  zz: 1\n",
    );
    let two = compile(&cfg, tmp.path()).unwrap().registry.specs[0]
        .extra_frontmatter
        .clone();
    assert_eq!(one, two);
}

#[test]
fn undeclared_nested_extra_keeps_pre013_guard() {
    // Guard regression (spec 013 §3.5): an UNDECLARED nested map is rejected
    // exactly as pre-013 (V-002, spec skipped).
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "custom_obj:\n  nested: 1\n");
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-002".to_string()));
    assert!(!outcome.validation_passed);
    assert!(outcome.registry.specs.is_empty(), "the spec is skipped");
}

#[test]
fn v013_unrepresentable_declared_value() {
    // A non-string map key under a DECLARED key -> V-013, skip-and-continue.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "compliance:\n  1: \"x\"\n");
    write_spec(tmp.path(), "002-ok", "002-ok", "");
    let mut cfg = Config::default();
    cfg.frontmatter.extra_known_keys = vec!["compliance".into()];
    let outcome = compile(&cfg, tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-013".to_string()));
    assert!(!outcome.validation_passed);
    assert!(outcome.registry.specs.iter().any(|s| s.id == "002-ok"));
    assert!(!outcome.registry.specs.iter().any(|s| s.id == "001-a"));
}

#[test]
fn v007_cap_unchanged_in_presence_of_declared_keys() {
    // Spec 013 §3.5: the undeclared cap is counted and enforced exactly as
    // before, with declared keys present and exempt.
    let tmp = tempfile::tempdir().unwrap();
    let n = MAX_UNDECLARED_EXTRA_FRONTMATTER + 1;
    let mut extra = String::from("compliance:\n  reviewed: true\n");
    for i in 0..n {
        extra.push_str(&format!("x_key_{i}: {i}\n"));
    }
    write_spec(tmp.path(), "001-a", "001-a", &extra);
    let mut cfg = Config::default();
    cfg.frontmatter.extra_known_keys = vec!["compliance".into()];
    let outcome = compile(&cfg, tmp.path()).unwrap();
    assert!(
        codes(&outcome).contains(&"V-007".to_string()),
        "undeclared cap still fires"
    );
}

#[test]
fn v005_domain_allowlist_when_enabled() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "domain: \"galaxy\"\n");

    // Disabled (default) -> no V-005.
    assert!(
        !codes(&compile(&Config::default(), tmp.path()).unwrap()).contains(&"V-005".to_string())
    );

    // Enabled and value not permitted -> V-005.
    let mut cfg = Config::default();
    cfg.domains.allowed = vec!["app".into(), "substrate".into()];
    assert!(codes(&compile(&cfg, tmp.path()).unwrap()).contains(&"V-005".to_string()));
}

#[test]
fn v008_superseded_requires_resolvable_superseded_by() {
    let tmp = tempfile::tempdir().unwrap();
    let spec_dir = tmp.path().join("specs").join("001-a");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        "---\nid: \"001-a\"\ntitle: t\nstatus: superseded\ncreated: \"2026-06-08\"\nsummary: s\n---\n",
    )
    .unwrap();
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    let v = outcome
        .registry
        .validation
        .violations
        .iter()
        .find(|v| v.code == "V-008")
        .expect("V-008 expected");
    assert_eq!(v.severity, Severity::Error);
}

#[test]
fn paths_sugar_is_byte_equivalent_to_single_unit_items() {
    // Spec 014 §3.3, the acceptance test: the same corpus authored with
    // `paths: [a, b]` and as N single-`unit` items compiles to identical
    // registries. Only `build.contentHash` may differ (it hashes the authored
    // spec bytes, which differ by construction); every emitted record and the
    // validation report must match byte-for-byte.
    let sugar = tempfile::tempdir().unwrap();
    write_spec(
        sugar.path(),
        "001-a",
        "001-a",
        "extends:\n  - { spec: \"000-x\", paths: [\"a.rs\", \"b.rs\"], nature: additive }\nrefines:\n  - { aspect: \"det\", paths: [\"c.rs\", \"d/\"] }\n",
    );
    let desugared = tempfile::tempdir().unwrap();
    write_spec(
        desugared.path(),
        "001-a",
        "001-a",
        "extends:\n  - { spec: \"000-x\", unit: \"a.rs\", nature: additive }\n  - { spec: \"000-x\", unit: \"b.rs\", nature: additive }\nrefines:\n  - { aspect: \"det\", unit: \"c.rs\" }\n  - { aspect: \"det\", unit: \"d/\" }\n",
    );

    let cfg = Config::default();
    let a = compile(&cfg, sugar.path()).unwrap();
    let b = compile(&cfg, desugared.path()).unwrap();
    assert_eq!(
        serde_json::to_string(&a.registry.specs).unwrap(),
        serde_json::to_string(&b.registry.specs).unwrap(),
        "expanded records must be byte-identical"
    );
    assert_eq!(a.registry.validation, b.registry.validation);
}

#[test]
fn paths_sugar_grammar_violations_are_v002() {
    // unit: + paths: on one item.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        "extends:\n  - { spec: \"000-x\", unit: \"a.rs\", paths: [\"b.rs\"] }\n",
    );
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-002".to_string()));
    assert!(outcome.registry.specs.is_empty(), "the spec is skipped");

    // Empty paths: list.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(
        tmp.path(),
        "001-a",
        "001-a",
        "refines:\n  - { aspect: \"a\", paths: [] }\n",
    );
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(codes(&outcome).contains(&"V-002".to_string()));
}

#[test]
fn oap_dialect_refines_fixture_compiles_clean() {
    // Spec 014 §3.4: a fixture modeled on the real OAP shape -- `refines`
    // with an aspect, refines_specs, and two paths -- compiles clean.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-base", "001-base", "");
    write_spec(
        tmp.path(),
        "002-tighten",
        "002-tighten",
        "refines:\n  - aspect: \"hash-determinism\"\n    refines_specs: [\"001-base\"]\n    paths: [\"src/hash.rs\", \"src/canonical_json.rs\"]\n",
    );
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(outcome.validation_passed, "{:?}", codes(&outcome));
    let spec = outcome
        .registry
        .specs
        .iter()
        .find(|s| s.id == "002-tighten")
        .unwrap();
    assert_eq!(spec.refines.len(), 2, "expanded to one item per path");
    assert!(
        spec.refines
            .iter()
            .all(|r| r.unit.is_some() && r.paths.is_none())
    );
}
