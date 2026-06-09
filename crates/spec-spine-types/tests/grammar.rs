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
fn legacy_paths_field_on_edge_is_rejected() {
    // deny_unknown_fields on edge items: the retired `paths:` form fails loudly.
    let src = "---\nid: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
extends:\n  - { spec: \"000\", paths: [\"a.rs\"] }\n---\n";
    assert!(parse_frontmatter(src).is_err());
}

#[test]
fn supersedes_and_amends_are_id_lists() {
    let fm = fm_with_edges("supersedes: [\"001-old\"]\namends: [\"002-other\"]\n");
    assert_eq!(fm.supersedes, vec!["001-old".to_string()]);
    assert_eq!(fm.amends, vec!["002-other".to_string()]);
}
