//! Dogfood: this repo's own bootstrap spec must parse through the types crate.
//!
//! The authored frontmatter of `specs/000` must conform to the grammar this
//! crate defines; a parse failure here means the corpus and the types drifted
//! apart (this guards the grammar independently of the full compile pipeline).

use spec_spine_types::{Status, parse_frontmatter};

fn read_repo_file(rel: &str) -> String {
    let path = format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn bootstrap_spec_000_parses() {
    let src = read_repo_file("specs/000-spec-spine-bootstrap/spec.md");
    let fm = parse_frontmatter(&src).expect("specs/000 frontmatter must parse");

    assert_eq!(fm.id, "000-spec-spine-bootstrap");
    assert_eq!(fm.status, Status::Approved);

    // It is the retroactive bootstrap root.
    let origin = fm.origin.expect("000 must declare origin");
    assert!(origin.retroactive, "000 must be origin.retroactive");

    // Its constitutional freeze surface is present and includes the core anchors.
    for anchor in [
        "markdown-truth-boundary",
        "json-truth-boundary",
        "determinism-requirement",
        "directory-name-equals-id",
        "typed-authority-graph",
        "refusal-rule",
    ] {
        assert!(
            fm.unamendable.iter().any(|a| a == anchor),
            "000 unamendable must include {anchor}"
        );
    }

    // The root declares no relationship edges (it establishes nothing via the graph).
    assert!(fm.establishes.is_empty());
    assert!(fm.supersedes.is_empty());
    assert!(fm.amends.is_empty());

    // No unknown keys leaked into the overflow.
    assert!(
        fm.extra_frontmatter.is_empty(),
        "unexpected extra frontmatter: {:?}",
        fm.extra_frontmatter.keys().collect::<Vec<_>>()
    );
}
