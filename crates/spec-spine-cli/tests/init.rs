//! `spec-spine init` + the adoption definition-of-done (prompt §8): scaffold a
//! throwaway repo and run the full compile → index → lint → couple loop against
//! it with a **non-default `manifest.metadata_namespace`** and a **custom
//! `domains.allowed`**, with zero source edits to the library.

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

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    bin().arg("--repo").arg(root).args(args).output().unwrap()
}

#[test]
fn init_is_idempotent_and_force_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let first = run(root, &["init"]);
    assert_eq!(code(&first), 0);
    assert!(root.join("specs/000-bootstrap/spec.md").is_file());
    assert!(root.join("standards/spec/constitution.md").is_file());
    assert!(
        root.join(".claude/rules/adversarial-prompt-refusal.md")
            .is_file()
    );

    // Second run skips existing files (idempotent, still exit 0).
    let second = run(root, &["init"]);
    assert_eq!(code(&second), 0);
    assert!(String::from_utf8_lossy(&second.stdout).contains("skip (exists)"));

    // --force overwrites in place.
    let forced = run(root, &["init", "--force"]);
    assert_eq!(code(&forced), 0);
    assert!(String::from_utf8_lossy(&forced.stdout).contains("(--force)"));
}

#[test]
fn adoption_loop_with_non_default_namespace_and_domains() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Non-default namespace + a custom domain allowlist — written before init so
    // the scaffolder and every command read them.
    write(
        root,
        "spec-spine.toml",
        "[manifest]\nmetadata_namespace = \"acme\"\n\n[domains]\nallowed = [\"tooling\"]\n",
    );

    assert_eq!(code(&run(root, &["init"])), 0);

    // A crate linked back via the NON-DEFAULT namespace key.
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"tool-x\"]\n");
    write(
        root,
        "tool-x/Cargo.toml",
        "[package]\nname = \"tool-x\"\nversion = \"0.1.0\"\n\
         [package.metadata.acme]\nspec = \"010-feature\"\n",
    );
    write(root, "tool-x/src/lib.rs", "pub fn run() {}\n");
    // A spec with a VALID domain from the custom allowlist.
    write(
        root,
        "specs/010-feature/spec.md",
        "---\nid: \"010-feature\"\ntitle: \"Feature\"\nstatus: approved\n\
         created: \"2026-06-09\"\nsummary: \"s\"\ndomain: \"tooling\"\n\
         establishes:\n  - \"tool-x/src/lib.rs\"\n---\n# 010\n## body\n",
    );

    // compile → index → lint, all clean.
    assert_eq!(code(&run(root, &["compile"])), 0, "compile clean");
    let idx = run(root, &["index"]);
    assert_eq!(code(&idx), 0, "index clean");
    // The non-default namespace drove the manifest read.
    let index_json = fs::read_to_string(root.join(".derived/codebase-index/index.json")).unwrap();
    assert!(
        index_json.contains("\"specRef\": \"010-feature\""),
        "acme namespace must link tool-x → 010-feature"
    );
    assert_eq!(code(&run(root, &["lint"])), 0, "lint runs");

    // couple: drift when only code changes; cleared when the owning spec is edited.
    write(root, "changed1.txt", "tool-x/src/lib.rs\n");
    let drift = run(
        root,
        &[
            "couple",
            "--paths-from",
            root.join("changed1.txt").to_str().unwrap(),
        ],
    );
    assert_eq!(
        code(&drift),
        1,
        "{}",
        String::from_utf8_lossy(&drift.stderr)
    );

    write(
        root,
        "changed2.txt",
        "tool-x/src/lib.rs\nspecs/010-feature/spec.md\n",
    );
    let cleared = run(
        root,
        &[
            "couple",
            "--paths-from",
            root.join("changed2.txt").to_str().unwrap(),
        ],
    );
    assert_eq!(code(&cleared), 0);
}

#[test]
fn custom_domain_allowlist_is_enforced_at_compile() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "spec-spine.toml",
        "[domains]\nallowed = [\"tooling\"]\n",
    );
    write(root, "Cargo.toml", "[workspace]\nmembers = []\n");
    // A domain OUTSIDE the allowlist → compile validation failure (exit 1).
    write(
        root,
        "specs/010-feature/spec.md",
        "---\nid: \"010-feature\"\ntitle: \"F\"\nstatus: approved\ncreated: \"2026-06-09\"\n\
         summary: \"s\"\ndomain: \"not-allowed\"\n---\n# 010\n",
    );
    let out = run(root, &["compile"]);
    assert_eq!(code(&out), 1, "invalid domain must fail compile");
}
