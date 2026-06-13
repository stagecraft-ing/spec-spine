//! Registry DTO round-trips, version constants, severity/validation logic.

use spec_spine_types::{
    BUILD_META_SCHEMA_VERSION, CONFIG_VERSION, INDEX_SCHEMA_VERSION, REGISTRY_SCHEMA_VERSION,
    Registry, Severity, Status, ValidationReport, Violation, parse_semver,
};

const REGISTRY_JSON: &str = r#"{
  "specVersion": "0.1.0",
  "build": {
    "compilerId": "spec-spine",
    "compilerVersion": "0.1.0",
    "inputRoot": ".",
    "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
  },
  "specs": [
    {
      "id": "000-x",
      "title": "T",
      "status": "approved",
      "created": "2026-06-08",
      "summary": "s",
      "specPath": "specs/000-x/spec.md",
      "coAuthority": [
        { "unit": { "kind": "section", "file": "Cargo.toml", "anchor": "deps" }, "with_specs": ["001-y"] }
      ],
      "origin": { "retroactive": true }
    }
  ],
  "validation": { "passed": true, "violations": [] }
}"#;

#[test]
fn registry_round_trips_camelcase() {
    let reg: Registry = serde_json::from_str(REGISTRY_JSON).unwrap();
    assert_eq!(reg.spec_version, "0.1.0");
    assert_eq!(reg.specs.len(), 1);
    let s = &reg.specs[0];
    assert_eq!(s.spec_path, "specs/000-x/spec.md");
    assert_eq!(s.status, Status::Approved);
    assert_eq!(s.co_authority.len(), 1);
    assert!(reg.validation.passed);

    // Serialize back; top-level keys are camelCase.
    let out = serde_json::to_string(&reg).unwrap();
    assert!(out.contains("\"specPath\""));
    assert!(out.contains("\"coAuthority\""));
    assert!(out.contains("\"contentHash\""));
}

#[test]
fn validation_passed_follows_error_tier() {
    let warn = ValidationReport::from_violations(vec![Violation {
        code: "L-001".into(),
        severity: Severity::Warning,
        message: "w".into(),
        path: None,
    }]);
    assert!(warn.passed, "warnings alone do not fail validation");

    let err = ValidationReport::from_violations(vec![Violation {
        code: "V-001".into(),
        severity: Severity::Error,
        message: "e".into(),
        path: None,
    }]);
    assert!(!err.passed, "any error-tier violation fails validation");
}

#[test]
fn schema_versions_are_pinned() {
    // 0.3.0: structured/partial `supersedes` items (spec 019).
    assert_eq!(REGISTRY_SCHEMA_VERSION, "0.3.0");
    // 0.3.0: additive `directory`/`crate`/`module` resolved-unit kinds (spec 017).
    assert_eq!(INDEX_SCHEMA_VERSION, "0.3.0");
    assert_eq!(BUILD_META_SCHEMA_VERSION, "0.1.0");
    assert_eq!(CONFIG_VERSION, "0.1.0");
}

#[test]
fn parse_semver_works() {
    assert_eq!(parse_semver("0.1.0"), Some((0, 1, 0)));
    assert_eq!(parse_semver("2.13.4"), Some((2, 13, 4)));
    assert_eq!(parse_semver("0.1"), None);
    assert_eq!(parse_semver("0.1.0.0"), None);
    assert_eq!(parse_semver("x.y.z"), None);
}
