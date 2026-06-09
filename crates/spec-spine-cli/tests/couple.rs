//! `spec-spine couple` end-to-end exit-code contract (spec 005):
//! drift → 1, cleared / waived → 0, stale index → 2. Uses `--paths-from` for the
//! deterministic core, plus one real `git diff` path.

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_spec-spine"))
}

fn code(out: &std::process::Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

/// A minimal governed repo: one crate owned (manifest + file unit) by spec 001-a.
fn setup(root: &Path) {
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"crate-a\"]\n");
    write(
        root,
        "crate-a/Cargo.toml",
        "[package]\nname = \"crate-a\"\nversion = \"0.1.0\"\n\
         [package.metadata.spec-spine]\nspec = \"001-a\"\n",
    );
    write(root, "crate-a/src/lib.rs", "pub fn a() {}\n");
    write(
        root,
        "specs/001-a/spec.md",
        "---\nid: \"001-a\"\ntitle: \"A\"\nstatus: approved\ncreated: \"2026-06-09\"\n\
         summary: \"s\"\nestablishes:\n  - \"crate-a/src/lib.rs\"\n---\n# 001-a\n## body\n",
    );
}

/// Compile + index so `couple`'s freshness guard passes against committed inputs.
fn refresh(root: &Path) {
    assert_eq!(
        code(
            &bin()
                .arg("--repo")
                .arg(root)
                .arg("compile")
                .output()
                .unwrap()
        ),
        0
    );
    assert_eq!(
        code(&bin().arg("--repo").arg(root).arg("index").output().unwrap()),
        0
    );
}

fn couple_paths(root: &Path, paths: &[&str], extra: &[&str]) -> std::process::Output {
    write(root, "changed.txt", &format!("{}\n", paths.join("\n")));
    let mut cmd = bin();
    cmd.arg("--repo")
        .arg(root)
        .arg("couple")
        .arg("--paths-from")
        .arg(root.join("changed.txt"));
    cmd.args(extra);
    cmd.output().unwrap()
}

#[test]
fn drift_then_cleared() {
    let tmp = tempfile::tempdir().unwrap();
    setup(tmp.path());
    refresh(tmp.path());

    // Changed an owned path, did not edit its spec → drift (exit 1).
    let drift = couple_paths(tmp.path(), &["crate-a/src/lib.rs"], &[]);
    assert_eq!(
        code(&drift),
        1,
        "{}",
        String::from_utf8_lossy(&drift.stderr)
    );

    // Same change + the owning spec.md → cleared (exit 0).
    let cleared = couple_paths(
        tmp.path(),
        &["crate-a/src/lib.rs", "specs/001-a/spec.md"],
        &[],
    );
    assert_eq!(code(&cleared), 0);
}

#[test]
fn waiver_clears_exit() {
    let tmp = tempfile::tempdir().unwrap();
    setup(tmp.path());
    refresh(tmp.path());
    write(
        tmp.path(),
        "pr-body.txt",
        "rolling forward\nSpec-Drift-Waiver: hotfix OPS-9\n",
    );
    let out = couple_paths(
        tmp.path(),
        &["crate-a/src/lib.rs"],
        &[
            "--pr-body",
            tmp.path().join("pr-body.txt").to_str().unwrap(),
        ],
    );
    assert_eq!(code(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains("waived"));
}

#[test]
fn stale_index_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    setup(tmp.path());
    refresh(tmp.path());
    // Mutate a hashed input (the spec) without re-indexing → stale.
    write(
        tmp.path(),
        "specs/001-a/spec.md",
        "---\nid: \"001-a\"\ntitle: \"A\"\nstatus: draft\ncreated: \"2026-06-09\"\n\
         summary: \"s\"\nestablishes:\n  - \"crate-a/src/lib.rs\"\n---\n# 001-a\n## body\n",
    );
    let out = couple_paths(tmp.path(), &["crate-a/src/lib.rs"], &[]);
    assert_eq!(code(&out), 2, "stale index must exit 2");
}

#[test]
fn real_git_diff_detects_drift() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    setup(root);

    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };

    git(&["init", "-q"]);
    refresh(root);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "base"]);

    // Change an owned file; refresh + commit at head.
    write(root, "crate-a/src/lib.rs", "pub fn a() {}\npub fn b() {}\n");
    refresh(root);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "head"]);

    let drift = bin()
        .arg("--repo")
        .arg(root)
        .args(["couple", "--base", "HEAD~1", "--head", "HEAD"])
        .output()
        .unwrap();
    assert_eq!(
        code(&drift),
        1,
        "git-diff drift: {}",
        String::from_utf8_lossy(&drift.stderr)
    );
}
