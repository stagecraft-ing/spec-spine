//! Unit-grammar and edge-grammar tests.

use spec_spine_types::{Frontmatter, Unit, parse_frontmatter};

fn fm_with_edges(edges_yaml: &str) -> Frontmatter {
    let src = format!(
        "---\nid: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n{edges_yaml}---\n"
    );
    parse_frontmatter(&src).unwrap()
}

#[test]
fn bare_string_is_file_unit() {
    let u: Unit = serde_yaml::from_str("\"src/lib.rs\"").unwrap();
    assert_eq!(
        u,
        Unit::File {
            path: "src/lib.rs".into()
        }
    );
}

#[test]
fn tagged_units_parse() {
    let file: Unit = serde_yaml::from_str("{ kind: file, path: \"a.rs\" }").unwrap();
    assert_eq!(
        file,
        Unit::File {
            path: "a.rs".into()
        }
    );

    let section: Unit =
        serde_yaml::from_str("{ kind: section, file: \"Makefile\", anchor: \"build\" }").unwrap();
    assert_eq!(
        section,
        Unit::Section {
            file: "Makefile".into(),
            anchor: "build".into()
        }
    );

    let symbol: Unit = serde_yaml::from_str("{ kind: symbol, id: \"crate::run\" }").unwrap();
    assert_eq!(
        symbol,
        Unit::Symbol {
            id: "crate::run".into()
        }
    );
}

#[test]
fn directory_subtree_detection() {
    assert!(Unit::file("src/").is_directory_subtree());
    assert!(!Unit::file("src/lib.rs").is_directory_subtree());
}

#[test]
fn empty_unit_path_is_rejected() {
    assert!(serde_yaml::from_str::<Unit>("\"\"").is_err());
}

#[test]
fn unknown_unit_kind_is_rejected() {
    assert!(serde_yaml::from_str::<Unit>("{ kind: galaxy, id: x }").is_err());
}

#[test]
fn establishes_accepts_mixed_forms() {
    let fm = fm_with_edges(
        "establishes:\n  - \"src/whole.rs\"\n  - { kind: symbol, id: \"crate::f\" }\n",
    );
    assert_eq!(fm.establishes.len(), 2);
    assert_eq!(
        fm.establishes[0],
        Unit::File {
            path: "src/whole.rs".into()
        }
    );
    assert_eq!(
        fm.establishes[1],
        Unit::Symbol {
            id: "crate::f".into()
        }
    );
}

#[test]
fn extends_item_parses() {
    let fm = fm_with_edges(
        "extends:\n  - { spec: \"000-bootstrap\", unit: { kind: file, path: \"a.rs\" } }\n",
    );
    assert_eq!(fm.extends.len(), 1);
    assert_eq!(fm.extends[0].spec, "000-bootstrap");
}

#[test]
fn extends_paths_sugar_expands_to_file_units() {
    // Spec 014 §3.1/§3.2: the predecessor dialect's `paths:` list is sugar
    // for N single-unit items, expanded at parse time in authored order.
    let fm = fm_with_edges(
        "extends:\n  - { spec: \"000-x\", paths: [\"a.rs\", \"src/api/\"], nature: additive }\n",
    );
    assert_eq!(fm.extends.len(), 2);
    assert_eq!(
        fm.extends[0].unit,
        Some(Unit::File {
            path: "a.rs".into()
        })
    );
    assert_eq!(fm.extends[0].nature.as_deref(), Some("additive"));
    // Directory form (trailing slash) is plain file-unit semantics.
    assert_eq!(
        fm.extends[1].unit,
        Some(Unit::File {
            path: "src/api/".into()
        })
    );
    assert!(
        fm.extends.iter().all(|e| e.paths.is_none()),
        "the sugar never escapes the parser"
    );
}

#[test]
fn refines_paths_sugar_expands_symmetrically() {
    let fm = fm_with_edges(
        "refines:\n  - { aspect: \"determinism\", refines_specs: [\"001-a\"], paths: [\"a.rs\", \"b.rs\"] }\n",
    );
    assert_eq!(fm.refines.len(), 2);
    for r in &fm.refines {
        assert_eq!(r.aspect, "determinism");
        assert_eq!(r.refines_specs, vec!["001-a".to_string()]);
        assert!(r.paths.is_none());
    }
}

#[test]
fn unit_and_paths_together_is_rejected() {
    let src = "---\nid: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
extends:\n  - { spec: \"000-x\", unit: \"a.rs\", paths: [\"b.rs\"] }\n---\n";
    assert!(parse_frontmatter(src).is_err());
}

#[test]
fn empty_paths_list_is_rejected() {
    let src = "---\nid: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
refines:\n  - { aspect: \"a\", paths: [] }\n---\n";
    assert!(parse_frontmatter(src).is_err());
}

#[test]
fn non_string_paths_entry_is_rejected() {
    let src = "---\nid: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
extends:\n  - { spec: \"000-x\", paths: [{ kind: file, path: \"a.rs\" }] }\n---\n";
    assert!(parse_frontmatter(src).is_err());
}

#[test]
fn supersedes_and_amends_are_id_lists() {
    let fm = fm_with_edges("supersedes: [\"001-old\"]\namends: [\"002-other\"]\n");
    assert_eq!(fm.supersedes, vec!["001-old".to_string()]);
    assert_eq!(fm.amends, vec!["002-other".to_string()]);
}
