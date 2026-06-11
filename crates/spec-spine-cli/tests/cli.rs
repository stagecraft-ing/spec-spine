//! End-to-end exit-code contract for the `spec-spine` binary.
//!
//! Exit codes: 0 ok, 1 validation failure / not found, 3 I/O / parse / schema.

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_spec-spine"))
}

fn write_spec(root: &Path, dir: &str, id: &str, status: &str) {
    let spec_dir = root.join("specs").join(dir);
    fs::create_dir_all(&spec_dir).unwrap();
    let body = format!(
        "---\nid: \"{id}\"\ntitle: \"T\"\nstatus: {status}\ncreated: \"2026-06-08\"\nsummary: \"s\"\n---\n# {id}\n"
    );
    fs::write(spec_dir.join("spec.md"), body).unwrap();
}

fn code(out: &std::process::Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

#[test]
fn compile_ok_then_queries() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "approved");
    write_spec(tmp.path(), "002-b", "002-b", "approved");

    let compile = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("compile")
        .output()
        .unwrap();
    assert_eq!(code(&compile), 0, "clean compile exits 0");
    assert!(
        tmp.path()
            .join(".derived/spec-registry/registry.json")
            .is_file()
    );

    let list = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "list"])
        .output()
        .unwrap();
    assert_eq!(code(&list), 0);
    assert!(String::from_utf8_lossy(&list.stdout).contains("001-a"));

    let show_missing = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "show", "999-nope"])
        .output()
        .unwrap();
    assert_eq!(code(&show_missing), 1, "not found exits 1");
}

#[test]
fn registry_list_ids_only_projection() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "approved");
    write_spec(tmp.path(), "002-b", "002-b", "approved");
    write_spec(tmp.path(), "003-c", "003-c", "draft");
    let compiled = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("compile")
        .output()
        .unwrap();
    assert_eq!(code(&compiled), 0);

    // Text form: newline-delimited ids in id order, nothing else.
    let text = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "list", "--ids-only"])
        .output()
        .unwrap();
    assert_eq!(code(&text), 0);
    assert_eq!(
        String::from_utf8_lossy(&text.stdout),
        "001-a\n002-b\n003-c\n"
    );

    // JSON form: an array of id strings, same order.
    let json = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "list", "--ids-only", "--json"])
        .output()
        .unwrap();
    assert_eq!(code(&json), 0);
    let ids: Vec<String> = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(ids, ["001-a", "002-b", "003-c"]);

    // --status filters first, then the projection applies.
    let filtered = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "list", "--ids-only", "--status", "approved"])
        .output()
        .unwrap();
    assert_eq!(code(&filtered), 0);
    assert_eq!(String::from_utf8_lossy(&filtered.stdout), "001-a\n002-b\n");

    let filtered_json = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args([
            "registry",
            "list",
            "--ids-only",
            "--status",
            "retired",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(code(&filtered_json), 0);
    let none: Vec<String> = serde_json::from_slice(&filtered_json.stdout).unwrap();
    assert!(none.is_empty());

    // Empty projection in text mode: empty output (no "(no specs)"), exit 0.
    let empty = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "list", "--ids-only", "--status", "retired"])
        .output()
        .unwrap();
    assert_eq!(code(&empty), 0);
    assert!(empty.stdout.is_empty());
}

#[test]
fn registry_status_report_nonzero_only_projection() {
    let tmp = tempfile::tempdir().unwrap();
    // approved + draft present; superseded + retired are the zero-count rows.
    write_spec(tmp.path(), "001-a", "001-a", "approved");
    write_spec(tmp.path(), "002-b", "002-b", "approved");
    write_spec(tmp.path(), "003-c", "003-c", "draft");
    let compiled = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("compile")
        .output()
        .unwrap();
    assert_eq!(code(&compiled), 0);

    // Without the flag, output is byte-identical to pre-010 behavior.
    let plain = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "status-report"])
        .output()
        .unwrap();
    assert_eq!(code(&plain), 0);
    assert_eq!(
        String::from_utf8_lossy(&plain.stdout),
        "total:      3\ndraft:      1\napproved:   2\nsuperseded: 0\nretired:    0\n"
    );

    // Human form: zero-count rows omitted, total unaffected.
    let human = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "status-report", "--nonzero-only"])
        .output()
        .unwrap();
    assert_eq!(code(&human), 0);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "total:      3\ndraft:      1\napproved:   2\n"
    );

    // JSON form: zero-count keys absent, total present.
    let json = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "status-report", "--nonzero-only", "--json"])
        .output()
        .unwrap();
    assert_eq!(code(&json), 0);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["total"], 3);
    assert_eq!(report["draft"], 1);
    assert_eq!(report["approved"], 2);
    assert!(report.get("superseded").is_none());
    assert!(report.get("retired").is_none());
}

#[test]
fn compile_validation_failure_exits_1() {
    let tmp = tempfile::tempdir().unwrap();
    // Directory name != id -> V-001 (error tier).
    write_spec(tmp.path(), "001-folder", "001-mismatch", "approved");
    let out = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("compile")
        .output()
        .unwrap();
    assert_eq!(code(&out), 1, "validation failure exits 1");
}

#[test]
fn missing_specs_dir_exits_3() {
    let tmp = tempfile::tempdir().unwrap();
    // No specs/ dir at all -> I/O error.
    let out = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("compile")
        .output()
        .unwrap();
    assert_eq!(code(&out), 3, "I/O error exits 3");
}

#[test]
fn registry_query_before_compile_exits_3() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "approved");
    // No compile yet -> registry.json missing -> I/O error.
    let out = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["registry", "list"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 3);
}

#[test]
fn index_then_check_fresh_then_stale() {
    let tmp = tempfile::tempdir().unwrap();
    write_spec(tmp.path(), "001-a", "001-a", "approved");

    let built = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("index")
        .output()
        .unwrap();
    assert_eq!(code(&built), 0, "index writes -> 0");
    assert!(
        tmp.path()
            .join(".derived/codebase-index/index.json")
            .is_file()
    );

    let fresh = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "check"])
        .output()
        .unwrap();
    assert_eq!(code(&fresh), 0, "fresh -> 0");

    // Mutate a hashed input -> stale.
    write_spec(tmp.path(), "001-a", "001-a", "draft");
    let stale = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "check"])
        .output()
        .unwrap();
    assert_eq!(code(&stale), 2, "stale -> 2");
}

#[test]
fn index_render_and_orphans_projections() {
    let tmp = tempfile::tempdir().unwrap();
    let write_claiming_spec = |id: &str, target: &str| {
        let dir = tmp.path().join("specs").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("spec.md"),
            format!(
                "---\nid: \"{id}\"\ntitle: \"T\"\nstatus: approved\ncreated: \"2026-06-08\"\nsummary: \"s\"\nestablishes:\n  - \"{target}\"\n---\n# {id}\n"
            ),
        )
        .unwrap();
    };
    // 001-a claims a path that resolves -> mapped; 002-b claims a path that
    // resolves nowhere -> orphaned.
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/lib.rs"), "// Spec: 001-a\n").unwrap();
    write_claiming_spec("001-a", "src/lib.rs");
    write_claiming_spec("002-b", "src/missing.rs");

    // Projections before `index` has run: exit 3 (missing artifact).
    let early_render = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "render"])
        .output()
        .unwrap();
    assert_eq!(code(&early_render), 3, "render without index -> 3");
    let early_orphans = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "orphans"])
        .output()
        .unwrap();
    assert_eq!(code(&early_orphans), 3, "orphans without index -> 3");

    let built = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("index")
        .output()
        .unwrap();
    assert_eq!(code(&built), 0);

    // Orphans, text and JSON forms.
    let orphans_text = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "orphans"])
        .output()
        .unwrap();
    assert_eq!(code(&orphans_text), 0, "orphans is a query, not a gate");
    assert_eq!(String::from_utf8_lossy(&orphans_text.stdout), "002-b\n");

    let orphans_json = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "orphans", "--json"])
        .output()
        .unwrap();
    assert_eq!(code(&orphans_json), 0);
    let ids: Vec<String> = serde_json::from_slice(&orphans_json.stdout).unwrap();
    assert_eq!(ids, ["002-b"]);

    // Render: exit 0 even with diagnostics in the artifact; contractual
    // sections present in order.
    let render = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "render"])
        .output()
        .unwrap();
    assert_eq!(code(&render), 0, "diagnostics do not fail a render");
    let md = String::from_utf8_lossy(&render.stdout);
    let positions: Vec<usize> = [
        "# spec-spine codebase index",
        "## Packages",
        "## Traceability",
    ]
    .iter()
    .map(|s| md.find(s).unwrap_or_else(|| panic!("missing section {s}")))
    .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "section order");
    assert!(md.contains("### Orphaned specs"));
    assert!(md.contains("- 002-b"));
    assert!(md.ends_with('\n'));

    // Empty orphans list -> empty output, still exit 0.
    fs::remove_dir_all(tmp.path().join("specs/002-b")).unwrap();
    let rebuilt = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("index")
        .output()
        .unwrap();
    assert_eq!(code(&rebuilt), 0);
    let none = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["index", "orphans"])
        .output()
        .unwrap();
    assert_eq!(code(&none), 0);
    assert!(none.stdout.is_empty());
}

#[test]
fn lint_fail_on_warn_gating() {
    let tmp = tempfile::tempdir().unwrap();
    // An ordinary spec with no ownership edge -> L-001 (warning).
    write_spec(tmp.path(), "001-a", "001-a", "approved");

    let lenient = bin()
        .arg("--repo")
        .arg(tmp.path())
        .arg("lint")
        .output()
        .unwrap();
    assert_eq!(code(&lenient), 0, "warnings alone do not fail");

    let strict = bin()
        .arg("--repo")
        .arg(tmp.path())
        .args(["lint", "--fail-on-warn"])
        .output()
        .unwrap();
    assert_eq!(code(&strict), 1, "--fail-on-warn fails on a warning");
}
