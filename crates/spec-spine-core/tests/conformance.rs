//! Schema conformance: emitted registry.json must validate against the embedded
//! JSON Schema. A drift between the DTOs and the schema fails here.

use std::fs;
use std::path::Path;

use spec_spine_core::{compile, registry_shard_files};
use spec_spine_types::{Config, REGISTRY_SCHEMA, REGISTRY_SPEC_SHARD_SCHEMA};

fn write_spec(root: &Path, id: &str, extra: &str) {
    let spec_dir = root.join("specs").join(id);
    fs::create_dir_all(&spec_dir).unwrap();
    let body = format!(
        "---\nid: \"{id}\"\ntitle: \"Title {id}\"\nstatus: approved\ncreated: \"2026-06-08\"\nsummary: \"s\"\n{extra}---\n# {id}\n## Section\n"
    );
    fs::write(spec_dir.join("spec.md"), body).unwrap();
}

#[test]
fn emitted_registry_conforms_to_embedded_schema() {
    let tmp = tempfile::tempdir().unwrap();
    // Exercise edges + extra_frontmatter so the schema sees a rich record.
    write_spec(tmp.path(), "000-root", "origin:\n  retroactive: true\n");
    write_spec(
        tmp.path(),
        "001-child",
        "depends_on: [\"000-root\"]\nestablishes:\n  - \"src/lib.rs\"\nx_extra: \"v\"\nrisk: medium\nimplementation: complete\n",
    );

    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    assert!(outcome.validation_passed, "fixture must compile clean");

    let schema: serde_json::Value =
        serde_json::from_str(REGISTRY_SCHEMA).expect("embedded schema is JSON");
    let instance: serde_json::Value =
        serde_json::from_str(&outcome.json).expect("registry is JSON");

    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    if !validator.is_valid(&instance) {
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| e.to_string())
            .collect();
        panic!(
            "registry.json does not conform to the schema:\n{}",
            errors.join("\n")
        );
    }
}

#[test]
fn emitted_registry_shards_conform_to_embedded_schema() {
    // Spec 024: every committed per-spec registry shard must validate against the
    // embedded shard schema, so a DTO/schema drift fails the build.
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "000-root", "origin:\n  retroactive: true\n");
    write_spec(
        tmp.path(),
        "001-child",
        "depends_on: [\"000-root\"]\nestablishes:\n  - \"src/lib.rs\"\nx_extra: \"v\"\nrisk: medium\nimplementation: complete\n",
    );
    let outcome = compile(&Config::default(), tmp.path()).unwrap();
    let files = registry_shard_files(&outcome.shards).unwrap();
    assert_eq!(files.len(), 2, "one shard per spec");

    let schema: serde_json::Value =
        serde_json::from_str(REGISTRY_SPEC_SHARD_SCHEMA).expect("embedded shard schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    for (name, content) in &files {
        let instance: serde_json::Value = serde_json::from_str(content).expect("shard is JSON");
        if !validator.is_valid(&instance) {
            let errors: Vec<String> = validator
                .iter_errors(&instance)
                .map(|e| e.to_string())
                .collect();
            panic!(
                "registry shard {name} does not conform:\n{}",
                errors.join("\n")
            );
        }
    }
}

#[test]
fn schema_rejects_a_malformed_registry() {
    // A registry missing the required `build` block must be rejected; proves the
    // conformance check has teeth.
    let schema: serde_json::Value = serde_json::from_str(REGISTRY_SCHEMA).unwrap();
    let bad = serde_json::json!({ "specVersion": "0.1.0", "specs": [], "validation": { "passed": true, "violations": [] } });
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(!validator.is_valid(&bad));
}
