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
