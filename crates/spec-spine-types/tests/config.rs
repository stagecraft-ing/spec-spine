//! Config tests: absent (defaults), minimal, full, malformed (clean error).

use spec_spine_types::{Config, Error, load_config};

#[test]
fn default_config_has_expected_values() {
    let c = Config::default();
    assert_eq!(c.manifest.metadata_namespace, "spec-spine");
    assert!(c.domains.allowed.is_empty());
    assert!(c.kind.allowed.is_empty());
    assert_eq!(c.layout.specs_dir, "specs");
    assert_eq!(c.layout.derived_dir, ".derived");
    assert_eq!(c.layout.standards_dir, "standards/spec");
    assert_eq!(c.layout.cargo_workspace, "Cargo.toml");
    // The encore-bug fix: the default reads root package.json#workspaces.
    assert!(
        c.layout
            .npm_workspaces
            .contains(&"package.json".to_string())
    );
    assert!(
        c.layout
            .npm_workspaces
            .contains(&"pnpm-workspace.yaml".to_string())
    );
    assert_eq!(c.branding.compiler_id, "spec-spine");
    assert_eq!(c.coupling.waiver_keyword, "Spec-Drift-Waiver:");
    assert_eq!(
        c.provenance
            .uri_schemes
            .get("code-fingerprint")
            .map(String::as_str),
        Some("fingerprint://")
    );
}

#[test]
fn absent_config_equals_default() {
    // An empty document yields a working default for a conventional repo.
    let c = load_config("").expect("empty config must load");
    assert_eq!(c, Config::default());
}

#[test]
fn minimal_config_overrides_one_knob() {
    let c = load_config("[manifest]\nmetadata_namespace = \"oap\"\n").unwrap();
    assert_eq!(c.manifest.metadata_namespace, "oap");
    // Everything else stays at default.
    assert_eq!(c.layout.specs_dir, "specs");
}

#[test]
fn allowlist_semantics() {
    let c = load_config("[domains]\nallowed = [\"app\", \"substrate\"]\n").unwrap();
    assert!(!c.domains.is_disabled());
    assert!(c.domains.permits("app"));
    assert!(!c.domains.permits("platform"));
    // Disabled allowlist permits anything.
    assert!(Config::default().kind.is_disabled());
    assert!(Config::default().kind.permits("anything-goes"));
}

#[test]
fn full_config_round_trips() {
    // Serialize a non-default config to TOML and load it back.
    let mut original = Config::default();
    original.manifest.metadata_namespace = "myns".to_string();
    original.domains.allowed = vec!["app".to_string(), "tooling".to_string()];
    original.kind.allowed = vec!["feature".to_string()];
    let toml_src = toml::to_string(&original).unwrap();
    let reloaded = load_config(&toml_src).unwrap();
    assert_eq!(original, reloaded);
}

#[test]
fn malformed_config_is_clean_error_not_panic() {
    // Unknown top-level section.
    let e = load_config("[bogus_section]\nx = 1\n").unwrap_err();
    assert!(matches!(e, Error::Config(_)));
    assert_eq!(e.exit_code(), 3);

    // Unknown key in a known section (the typo'd-knob failure class).
    let e = load_config("[manifest]\nmetadata_namspace = \"x\"\n").unwrap_err();
    assert!(matches!(e, Error::Config(_)));

    // Wrong type.
    let e = load_config("[domains]\nallowed = \"not-a-list\"\n").unwrap_err();
    assert!(matches!(e, Error::Config(_)));
}

#[test]
fn this_repos_spec_spine_toml_loads() {
    // Dogfood: the committed config for this very repo must parse under
    // deny_unknown_fields (catches struct/TOML drift).
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../spec-spine.toml");
    let src = std::fs::read_to_string(root).expect("repo spec-spine.toml must exist");
    let c = load_config(&src).expect("repo spec-spine.toml must load");
    assert_eq!(c.manifest.metadata_namespace, "spec-spine");
    assert!(c.domains.is_disabled());
    assert!(c.kind.is_disabled());
}
