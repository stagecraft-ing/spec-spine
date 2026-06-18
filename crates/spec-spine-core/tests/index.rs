//! Index integration tests: determinism, conformance, manifest + npm discovery
//! (the encore fix), file/section/symbol resolution with per-platform span
//! goldens, staleness, and authorities.

use std::fs;
use std::path::Path;

use spec_spine_core::shard::{self, BY_PACKAGE_DIR, BY_SPEC_DIR};
use spec_spine_core::{
    Freshness, IndexOutcome, authorities, check_index_freshness, index, index_dir,
    index_shard_files,
};
use spec_spine_types::{Config, INDEX_SCHEMA, LineSpan, PackageKind, Unit};

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

/// Write an index outcome to disk as the CLI's `spec-spine index` does: the
/// per-spec/per-package shard tree (spec 024), not a monolithic `index.json`.
fn emit_index_shards(cfg: &Config, repo: &Path, outcome: &IndexOutcome) {
    let dir = index_dir(cfg, repo);
    let (by_spec, by_package) = index_shard_files(&outcome.shards).unwrap();
    shard::sync_dir(&dir.join(BY_SPEC_DIR), &by_spec).unwrap();
    shard::sync_dir(&dir.join(BY_PACKAGE_DIR), &by_package).unwrap();
}

fn spec(id: &str, body: &str) -> String {
    format!(
        "---\nid: \"{id}\"\ntitle: \"T\"\nstatus: approved\ncreated: \"2026-06-09\"\nsummary: \"s\"\n{body}---\n# {id}\n"
    )
}

/// Like [`spec`] but with an explicit lifecycle `status` (spec 025 fixtures need
/// `draft` corpora; the default helper hardcodes `approved`).
fn spec_with_status(id: &str, status: &str, body: &str) -> String {
    format!(
        "---\nid: \"{id}\"\ntitle: \"T\"\nstatus: {status}\ncreated: \"2026-06-09\"\nsummary: \"s\"\n{body}---\n# {id}\n"
    )
}

/// A mixed Rust + npm fixture exercising manifest discovery, the encore fix, and
/// symbol resolution in both languages.
fn mixed_fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();

    // Rust workspace + crate.
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"rs-thing\"]\n");
    write(
        r,
        "rs-thing/Cargo.toml",
        "[package]\nname = \"rs-thing\"\nversion = \"0.1.0\"\n[package.metadata.spec-spine]\nspec = \"001-rs\"\n",
    );
    write(
        r,
        "rs-thing/src/lib.rs",
        "pub fn alpha() {}\npub struct Beta {\n    x: u8,\n}\n",
    );

    // npm workspace declared at the ROOT package.json (the encore fix).
    write(
        r,
        "package.json",
        "{\n  \"name\": \"root\",\n  \"workspaces\": [\"pkgs/*\"]\n}\n",
    );
    write(
        r,
        "pkgs/web/package.json",
        "{\n  \"name\": \"web\",\n  \"spec-spine\": { \"spec\": \"002-ts\" }\n}\n",
    );
    write(
        r,
        "pkgs/web/src/util.ts",
        "export function formatDate() {}\nexport class Helper {}\n",
    );

    // Specs declaring symbol units.
    write(
        r,
        "specs/001-rs/spec.md",
        &spec(
            "001-rs",
            "establishes:\n  - { kind: symbol, id: \"rs_thing::alpha\" }\n  - { kind: symbol, id: \"rs_thing::Beta\" }\n  - \"rs-thing/src/lib.rs\"\n",
        ),
    );
    write(
        r,
        "specs/002-ts/spec.md",
        &spec(
            "002-ts",
            "establishes:\n  - { kind: symbol, id: \"web::src::util::formatDate\" }\n",
        ),
    );
    tmp
}

fn mapping<'a>(
    idx: &'a spec_spine_types::CodebaseIndex,
    id: &str,
) -> &'a spec_spine_types::TraceMapping {
    idx.traceability
        .mappings
        .iter()
        .find(|m| m.spec_id == id)
        .expect("mapping present")
}

fn symbol_span(m: &spec_spine_types::TraceMapping, sym_id: &str) -> Option<LineSpan> {
    m.resolved_units
        .iter()
        .find(|u| matches!(&u.unit, Unit::Symbol { id } if id == sym_id))
        .and_then(|u| u.locations.first())
        .and_then(|loc| loc.span)
}

#[test]
fn indexes_deterministically() {
    let fx = mixed_fixture();
    let cfg = Config::default();
    let a = index(&cfg, fx.path()).unwrap();
    let b = index(&cfg, fx.path()).unwrap();
    assert_eq!(a.json, b.json, "index must be byte-identical across runs");
    assert!(a.json.ends_with("}\n"));
}

#[test]
fn discovers_rust_and_npm_packages() {
    // The npm package is declared by root package.json#workspaces; the encore
    // failure was that npm packages went undiscovered. They must appear here.
    let fx = mixed_fixture();
    let idx = index(&Config::default(), fx.path()).unwrap().index;

    let names: Vec<&str> = idx.packages.iter().map(|p| p.name.as_str()).collect();
    assert!(
        names.contains(&"rs-thing"),
        "rust crate discovered: {names:?}"
    );
    assert!(
        names.contains(&"web"),
        "npm package discovered (encore fix): {names:?}"
    );

    let web = idx.packages.iter().find(|p| p.name == "web").unwrap();
    assert_eq!(web.kind, PackageKind::NpmPackage);
    assert_eq!(web.spec_ref.as_deref(), Some("002-ts"));
}

#[test]
fn resolves_rust_symbols_with_exact_spans() {
    // Per-platform span golden (watch-item 2): pinned tree-sitter ⇒ identical
    // spans on every triple.
    let fx = mixed_fixture();
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    let m = mapping(&idx, "001-rs");
    assert_eq!(symbol_span(m, "rs_thing::alpha"), Some(LineSpan::new(1, 1)));
    assert_eq!(symbol_span(m, "rs_thing::Beta"), Some(LineSpan::new(2, 4)));
}

#[test]
fn resolves_typescript_symbols_with_exact_spans() {
    let fx = mixed_fixture();
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    let m = mapping(&idx, "002-ts");
    assert_eq!(
        symbol_span(m, "web::src::util::formatDate"),
        Some(LineSpan::new(1, 1))
    );
}

#[test]
fn missing_file_unit_is_blocking_diagnostic_i004() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec("001-x", "establishes:\n  - \"src/does_not_exist.rs\"\n"),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(idx.diagnostics.errors.iter().any(|d| d.code == "I-004"));
}

// ===== spec 025: lifecycle- and edge-aware unresolved-unit severity =====

/// AC-1: an unresolved unit on a non-owning `references` edge is a counted
/// `W-002` warning, never a blocking error, regardless of lifecycle.
#[test]
fn ac1_unresolved_reference_is_w002_warning_not_error() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec(
            "001-x",
            "implementation: complete\nreferences:\n  - { unit: { kind: file, path: \"docs/gone.md\" }, role: context }\n",
        ),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(
        idx.diagnostics.errors.is_empty(),
        "a dangling reference must not block: {:?}",
        idx.diagnostics.errors
    );
    assert_eq!(
        idx.diagnostics
            .warnings
            .iter()
            .filter(|d| d.code == "W-002")
            .count(),
        1,
        "exactly one W-002 for the unresolved reference"
    );
}

/// AC-2: an unresolved owning unit on a `draft` spec is a counted `W-001`
/// warning, never a blocking error (legitimate in-flight work).
#[test]
fn ac2_draft_owning_unit_is_w001_warning_not_error() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec_with_status(
            "001-x",
            "draft",
            "establishes:\n  - \"src/not_built_yet.rs\"\n",
        ),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(
        idx.diagnostics.errors.is_empty(),
        "a draft spec's unbuilt owning unit must not block: {:?}",
        idx.diagnostics.errors
    );
    assert_eq!(
        idx.diagnostics
            .warnings
            .iter()
            .filter(|d| d.code == "W-001")
            .count(),
        1
    );
}

/// AC-3: `status: approved` but `implementation: pending` is in-flight, so an
/// unresolved owning unit is `W-001` (spec 025 §3.1 arm 2 keys on either signal).
#[test]
fn ac3_pending_owning_unit_is_w001_warning_not_error() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec(
            "001-x",
            "implementation: pending\nestablishes:\n  - \"src/not_built_yet.rs\"\n",
        ),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(
        idx.diagnostics.errors.is_empty(),
        "a pending spec's unbuilt owning unit must not block: {:?}",
        idx.diagnostics.errors
    );
    assert_eq!(
        idx.diagnostics
            .warnings
            .iter()
            .filter(|d| d.code == "W-001")
            .count(),
        1
    );
}

/// AC-4: a settled (`approved` + `complete`) spec's missing owning unit stays a
/// hard `I-004` error, unchanged by spec 025 (the complement of AC-2 / AC-3).
#[test]
fn ac4_settled_owning_unit_still_errors_i004() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec(
            "001-x",
            "implementation: complete\nestablishes:\n  - \"src/gone.rs\"\n",
        ),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(idx.diagnostics.errors.iter().any(|d| d.code == "I-004"));
    assert!(
        idx.diagnostics.warnings.iter().all(|d| d.code != "W-001"),
        "a settled spec is not downgraded"
    );
}

/// AC-5: edge-type precedence. A `references` edge on a `draft` spec is `W-002`
/// (arm 1), not `W-001`: edge authority is evaluated before lifecycle.
#[test]
fn ac5_reference_on_draft_spec_is_w002_edge_type_precedence() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec_with_status(
            "001-x",
            "draft",
            "references:\n  - { unit: { kind: file, path: \"docs/gone.md\" }, role: context }\n",
        ),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(idx.diagnostics.errors.is_empty());
    assert_eq!(
        idx.diagnostics
            .warnings
            .iter()
            .filter(|d| d.code == "W-002")
            .count(),
        1
    );
    assert!(
        idx.diagnostics.warnings.iter().all(|d| d.code != "W-001"),
        "edge-type wins: an unresolved reference is W-002 even on a draft spec"
    );
}

#[test]
fn resolves_section_unit() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "Makefile",
        "build:\n\tcargo build\n\ntest:\n\tcargo test\n",
    );
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec(
            "001-x",
            "establishes:\n  - { kind: section, file: \"Makefile\", anchor: \"build\" }\n",
        ),
    );
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    let m = mapping(&idx, "001-x");
    let loc = m.resolved_units[0]
        .locations
        .first()
        .expect("section resolved");
    assert_eq!(loc.file, "Makefile");
    assert_eq!(loc.span, Some(LineSpan::new(1, 2)));
}

#[test]
fn conforms_to_embedded_schema() {
    let fx = mixed_fixture();
    let outcome = index(&Config::default(), fx.path()).unwrap();
    let schema: serde_json::Value = serde_json::from_str(INDEX_SCHEMA).unwrap();
    let instance: serde_json::Value = serde_json::from_str(&outcome.json).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    if !validator.is_valid(&instance) {
        let errs: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| e.to_string())
            .collect();
        panic!("index.json does not conform:\n{}", errs.join("\n"));
    }
}

#[test]
fn emitted_index_shards_conform_to_embedded_schema() {
    use spec_spine_types::{INDEX_PACKAGE_SHARD_SCHEMA, INDEX_SPEC_SHARD_SCHEMA};
    let fx = mixed_fixture();
    let outcome = index(&Config::default(), fx.path()).unwrap();
    let (by_spec, by_package) = index_shard_files(&outcome.shards).unwrap();
    assert!(!by_spec.is_empty() && !by_package.is_empty());

    let check = |schema_src: &str, files: &[(String, String)]| {
        let schema: serde_json::Value = serde_json::from_str(schema_src).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for (name, content) in files {
            let instance: serde_json::Value = serde_json::from_str(content).unwrap();
            if !validator.is_valid(&instance) {
                let errs: Vec<String> = validator
                    .iter_errors(&instance)
                    .map(|e| e.to_string())
                    .collect();
                panic!("index shard {name} does not conform:\n{}", errs.join("\n"));
            }
        }
    };
    check(INDEX_SPEC_SHARD_SCHEMA, &by_spec);
    check(INDEX_PACKAGE_SHARD_SCHEMA, &by_package);
}

#[test]
fn staleness_detects_input_change() {
    let fx = mixed_fixture();
    let cfg = Config::default();
    // Write the index to disk as the CLI would.
    let outcome = index(&cfg, fx.path()).unwrap();
    emit_index_shards(&cfg, fx.path(), &outcome);

    assert_eq!(
        check_index_freshness(&cfg, fx.path()).unwrap(),
        Freshness::Fresh
    );

    // Mutate a hashed input (a spec) -> stale.
    write(
        fx.path(),
        "specs/001-rs/spec.md",
        &spec("001-rs", "owner: \"changed\"\n"),
    );
    assert!(matches!(
        check_index_freshness(&cfg, fx.path()).unwrap(),
        Freshness::Stale { .. }
    ));
}

#[test]
fn staleness_detects_symbol_source_line_shift() {
    // The freshness false-negative (spec 004 §3.5): a source-line shift in a file
    // backing a resolved SYMBOL span must report Stale, even though that file is
    // neither a manifest, a spec.md, nor an extra_hashed_input. Before the fix the
    // span-backing source was not hashed, so this read Fresh against stale spans.
    let fx = mixed_fixture();
    let cfg = Config::default();
    let outcome = index(&cfg, fx.path()).unwrap();
    emit_index_shards(&cfg, fx.path(), &outcome);

    // Sanity: the committed index resolved a symbol span into rs-thing/src/lib.rs.
    assert_eq!(
        symbol_span(mapping(&outcome.index, "001-rs"), "rs_thing::Beta"),
        Some(LineSpan::new(2, 4)),
        "fixture must back a symbol span with this source file"
    );
    assert_eq!(
        check_index_freshness(&cfg, fx.path()).unwrap(),
        Freshness::Fresh
    );

    // Prepend a line to the symbol's source file: this shifts every committed
    // span downward but touches no manifest/spec/config. It MUST go Stale.
    write(
        fx.path(),
        "rs-thing/src/lib.rs",
        "// a new leading comment line\npub fn alpha() {}\npub struct Beta {\n    x: u8,\n}\n",
    );
    assert!(
        matches!(
            check_index_freshness(&cfg, fx.path()).unwrap(),
            Freshness::Stale { .. }
        ),
        "a source-line shift behind a resolved symbol span must report Stale"
    );
}

#[test]
fn authorities_resolves_owners() {
    let fx = mixed_fixture();
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    // The file unit established by 001-rs.
    let owners = authorities(&idx, &Unit::file("rs-thing/src/lib.rs"));
    assert!(owners.contains(&"001-rs".to_string()), "owners: {owners:?}");
}

// ===== spec 017: crate / directory / module unit kinds =====

/// The resolved locations for the first resolved unit of `spec_id`.
fn first_unit_locations<'a>(
    idx: &'a spec_spine_types::CodebaseIndex,
    spec_id: &str,
) -> &'a [spec_spine_types::ResolvedLocation] {
    &mapping(idx, spec_id).resolved_units[0].locations
}

/// A Rust crate with an inline `mod tests {}`, a file-module (`helper.rs`), and a
/// nested directory, for exercising the three new unit kinds.
fn module_fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"rs-thing\"]\n");
    write(
        r,
        "rs-thing/Cargo.toml",
        "[package]\nname = \"rs-thing\"\nversion = \"0.1.0\"\n",
    );
    write(
        r,
        "rs-thing/src/lib.rs",
        "pub fn alpha() {}\n\nmod tests {\n    fn t() {}\n}\n",
    );
    write(r, "rs-thing/src/helper.rs", "pub fn help() {}\n");
    tmp
}

#[test]
fn crate_unit_resolves_to_package_subtree() {
    let fx = module_fixture();
    write(
        fx.path(),
        "specs/001-c/spec.md",
        &spec(
            "001-c",
            "establishes:\n  - { kind: crate, id: \"rs-thing\" }\n",
        ),
    );
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    let locs = first_unit_locations(&idx, "001-c");
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "rs-thing");
    // Hyphen/underscore are interchangeable in the crate id.
    assert!(
        authorities(
            &idx,
            &Unit::Crate {
                id: "rs-thing".into()
            }
        )
        .contains(&"001-c".into())
    );
}

#[test]
fn unknown_crate_unit_is_blocking_diagnostic_i003() {
    let fx = module_fixture();
    write(
        fx.path(),
        "specs/001-c/spec.md",
        &spec(
            "001-c",
            "establishes:\n  - { kind: crate, id: \"ghost\" }\n",
        ),
    );
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    assert!(idx.diagnostics.errors.iter().any(|d| d.code == "I-003"));
}

#[test]
fn directory_unit_resolves_to_subtree() {
    let fx = module_fixture();
    write(
        fx.path(),
        "specs/001-d/spec.md",
        &spec(
            "001-d",
            "establishes:\n  - { kind: directory, path: \"rs-thing/src\" }\n",
        ),
    );
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    let locs = first_unit_locations(&idx, "001-d");
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "rs-thing/src");
    assert_eq!(locs[0].span, None);
}

#[test]
fn missing_directory_unit_is_blocking_diagnostic_i007() {
    let fx = module_fixture();
    write(
        fx.path(),
        "specs/001-d/spec.md",
        &spec(
            "001-d",
            "establishes:\n  - { kind: directory, path: \"rs-thing/nope\" }\n",
        ),
    );
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    assert!(idx.diagnostics.errors.iter().any(|d| d.code == "I-007"));
}

#[test]
fn module_unit_resolves_inline_and_file_modules() {
    let fx = module_fixture();
    // Inline `mod tests {}` → a line span; the file-module `helper` → whole file.
    write(
        fx.path(),
        "specs/001-m/spec.md",
        &spec(
            "001-m",
            "establishes:\n  - { kind: module, id: \"rs_thing::tests\" }\n  - { kind: module, id: \"rs_thing::helper\" }\n",
        ),
    );
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    let m = mapping(&idx, "001-m");
    let tests_unit = m
        .resolved_units
        .iter()
        .find(|u| matches!(&u.unit, Unit::Module { id } if id == "rs_thing::tests"))
        .unwrap();
    assert_eq!(tests_unit.locations[0].file, "rs-thing/src/lib.rs");
    assert!(
        tests_unit.locations[0].span.is_some(),
        "inline mod resolves to a block span"
    );
    let helper_unit = m
        .resolved_units
        .iter()
        .find(|u| matches!(&u.unit, Unit::Module { id } if id == "rs_thing::helper"))
        .unwrap();
    assert_eq!(helper_unit.locations[0].file, "rs-thing/src/helper.rs");
    assert_eq!(
        helper_unit.locations[0].span, None,
        "a file-module resolves whole-file"
    );
}

#[test]
fn spec_scoped_constrains_produces_no_resolved_unit() {
    // Spec 018: a constrains item with target_specs and no unit claims no code
    // path, so it contributes no resolved unit to the index.
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        tmp.path(),
        "specs/001-x/spec.md",
        &spec(
            "001-x",
            "constrains:\n  - { kind: sequencing-plan, target_specs: [\"002-y\"] }\n",
        ),
    );
    write(tmp.path(), "specs/002-y/spec.md", &spec("002-y", ""));
    let idx = index(&Config::default(), tmp.path()).unwrap().index;
    assert!(
        mapping(&idx, "001-x").resolved_units.is_empty(),
        "spec-scoped constrains claims no code path"
    );
}

#[test]
fn unresolved_module_unit_is_blocking_diagnostic_i008() {
    let fx = module_fixture();
    write(
        fx.path(),
        "specs/001-m/spec.md",
        &spec(
            "001-m",
            "establishes:\n  - { kind: module, id: \"rs_thing::ghost\" }\n",
        ),
    );
    let idx = index(&Config::default(), fx.path()).unwrap().index;
    assert!(idx.diagnostics.errors.iter().any(|d| d.code == "I-008"));
}
