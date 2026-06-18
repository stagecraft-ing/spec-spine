//! Unit-grammar and edge-grammar tests.

use spec_spine_types::{Frontmatter, Implementation, SupersedeItem, Unit, parse_frontmatter};

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
fn directory_crate_module_units_parse() {
    // Spec 017: the three reserved unit kinds now parse from their tagged form.
    let dir: Unit = serde_yaml::from_str("{ kind: directory, path: \"src/api\" }").unwrap();
    assert_eq!(
        dir,
        Unit::Directory {
            path: "src/api".into()
        }
    );
    let krate: Unit = serde_yaml::from_str("{ kind: crate, id: \"factory-engine\" }").unwrap();
    assert_eq!(
        krate,
        Unit::Crate {
            id: "factory-engine".into()
        }
    );
    let module: Unit = serde_yaml::from_str("{ kind: module, id: \"my_crate::tests\" }").unwrap();
    assert_eq!(
        module,
        Unit::Module {
            id: "my_crate::tests".into()
        }
    );
    // The wrapper form (spec 015) composes with the new kinds.
    let wrapped: Unit =
        serde_yaml::from_str("{ unit: { kind: directory, path: \"crates/x\" } }").unwrap();
    assert_eq!(
        wrapped,
        Unit::Directory {
            path: "crates/x".into()
        }
    );
}

#[test]
fn unit_wrapper_form_unwraps() {
    // Spec 015 §3.1: `{ unit: <unit> }` is a 1:1 wrapper that normalizes to the
    // inner unit. The inner may itself be a bare string or a tagged map.
    let bare: Unit = serde_yaml::from_str("{ unit: \"src/lib.rs\" }").unwrap();
    assert_eq!(
        bare,
        Unit::File {
            path: "src/lib.rs".into()
        }
    );
    let tagged: Unit =
        serde_yaml::from_str("{ unit: { kind: symbol, id: \"crate::f\" } }").unwrap();
    assert_eq!(
        tagged,
        Unit::Symbol {
            id: "crate::f".into()
        }
    );
    // The inner unit inherits its own validation (empty path still rejected).
    assert!(serde_yaml::from_str::<Unit>("{ unit: \"\" }").is_err());
}

#[test]
fn directory_subtree_detection() {
    assert!(Unit::file("src/").is_directory_subtree());
    assert!(!Unit::file("src/lib.rs").is_directory_subtree());
    // An explicit directory unit is always a subtree (spec 017).
    assert!(Unit::Directory { path: "src".into() }.is_directory_subtree());
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
fn establishes_accepts_unit_wrapper() {
    // Spec 015 §3.1: the predecessor dialect wraps each establishes item in a
    // single-key `unit:` map. Wrapped and bare forms parse to the same units,
    // and may be mixed within one list.
    let fm = fm_with_edges(
        "establishes:\n  - { unit: \"src/whole.rs\" }\n  - \"src/bare.rs\"\n  - { unit: { kind: section, file: \"Makefile\", anchor: \"ci\" } }\n",
    );
    assert_eq!(
        fm.establishes,
        vec![
            Unit::File {
                path: "src/whole.rs".into()
            },
            Unit::File {
                path: "src/bare.rs".into()
            },
            Unit::Section {
                file: "Makefile".into(),
                anchor: "ci".into()
            },
        ]
    );
}

#[test]
fn implementation_na_slash_is_alias_for_na() {
    // Spec 015 §3.2: `n/a` is a deserialize-only alias for the canonical `n-a`,
    // which is the only spelling ever emitted.
    let slash = fm_with_edges("implementation: n/a\n");
    let kebab = fm_with_edges("implementation: n-a\n");
    assert_eq!(slash.implementation, Some(Implementation::Na));
    assert_eq!(slash.implementation, kebab.implementation);
    assert_eq!(
        serde_json::to_string(&Implementation::Na).unwrap(),
        "\"n-a\"",
        "Na must emit canonically as n-a"
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
fn constrains_discriminator_and_optional_unit() {
    // Spec 018: a path-scoped item carries flavor + unit; a spec-scoped item
    // carries kind + target_specs and no unit. Both spellings are accepted.
    let fm = fm_with_edges(
        "constrains:\n  - { flavor: invariant-freeze, unit: \"schema.json\" }\n  - { kind: sequencing-plan, target_specs: [\"001-a\", \"002-b\"] }\n",
    );
    assert_eq!(fm.constrains.len(), 2);
    assert_eq!(fm.constrains[0].flavor.as_deref(), Some("invariant-freeze"));
    assert_eq!(
        fm.constrains[0].unit,
        Some(Unit::File {
            path: "schema.json".into()
        })
    );
    assert_eq!(fm.constrains[1].kind.as_deref(), Some("sequencing-plan"));
    assert!(fm.constrains[1].unit.is_none());
    assert_eq!(fm.constrains[1].target_specs, vec!["001-a", "002-b"]);
}

#[test]
fn supersedes_and_amends_are_id_lists() {
    let fm = fm_with_edges("supersedes: [\"001-old\"]\namends: [\"002-other\"]\n");
    assert_eq!(fm.supersedes.len(), 1);
    assert_eq!(fm.supersedes[0].spec(), "001-old");
    assert!(fm.supersedes[0].is_full());
    assert_eq!(fm.amends, vec!["002-other".to_string()]);
}

#[test]
fn supersedes_full_and_partial_forms_parse() {
    // Spec 019: a bare id and `{ scope: full }` both normalize to the bare-string
    // full form; `{ scope: partial, unit }` and `{ scope: partial, note }` stay
    // structured. (Covers OAP shapes 108 / 073 / 114 / 199 respectively.)
    let fm = fm_with_edges(
        "supersedes:\n  - \"108-old\"\n  - { spec: \"073-x\", scope: full }\n  - { spec: \"113-y\", scope: partial, unit: \"src/clone.ts\" }\n  - { spec: \"140-z\", scope: partial, note: \"retires §2.2\" }\n",
    );
    assert_eq!(fm.supersedes.len(), 4);
    // Bare id → full.
    assert!(matches!(&fm.supersedes[0], SupersedeItem::Full(id) if id == "108-old"));
    // `{ scope: full }` collapses to the bare-string full form (byte-stable wire).
    assert!(matches!(&fm.supersedes[1], SupersedeItem::Full(id) if id == "073-x"));
    // Partial + unit stays structured and exposes its scoping unit.
    assert_eq!(fm.supersedes[2].spec(), "113-y");
    assert!(!fm.supersedes[2].is_full());
    assert_eq!(
        fm.supersedes[2].partial_unit(),
        Some(&Unit::File {
            path: "src/clone.ts".into()
        })
    );
    // Partial without a unit is documentary: no transfer unit.
    assert_eq!(fm.supersedes[3].spec(), "140-z");
    assert!(!fm.supersedes[3].is_full());
    assert_eq!(fm.supersedes[3].partial_unit(), None);
}

#[test]
fn references_provenance_derived_at_round_trips() {
    // Spec 028 AC-1: a `references` provenance `{ kind, ref, derived_at }` parses
    // and the timestamp is preserved on re-emit. (The shape OAP spec 165 emits.)
    let fm = fm_with_edges(
        "references:\n  - { role: decomposition-origin, provenance: { kind: code-fingerprint, ref: \"xray-fingerprint://abc123\", derived_at: \"2026-06-18T00:00:00Z\" } }\n",
    );
    assert_eq!(fm.references.len(), 1);
    assert_eq!(
        fm.references[0].role.as_deref(),
        Some("decomposition-origin")
    );
    let prov = fm.references[0]
        .provenance
        .as_ref()
        .expect("provenance present");
    assert_eq!(prov.kind, "code-fingerprint");
    assert_eq!(prov.reference, "xray-fingerprint://abc123");
    assert_eq!(prov.derived_at.as_deref(), Some("2026-06-18T00:00:00Z"));

    // Re-emit preserves the timestamp under the wire name `derived_at` (and `ref`).
    let json = serde_json::to_string(prov).unwrap();
    assert!(
        json.contains("\"derived_at\":\"2026-06-18T00:00:00Z\""),
        "{json}"
    );
    assert!(
        json.contains("\"ref\":\"xray-fingerprint://abc123\""),
        "{json}"
    );
}

#[test]
fn references_provenance_without_derived_at_omits_the_field() {
    // Spec 028 AC-2: an item without `derived_at` parses unchanged and does NOT
    // serialize the field (`skip_serializing_if`), so existing goldens are
    // byte-identical.
    let fm = fm_with_edges(
        "references:\n  - { provenance: { kind: knowledge, ref: \"knowledge://x\" } }\n",
    );
    let prov = fm.references[0]
        .provenance
        .as_ref()
        .expect("provenance present");
    assert_eq!(prov.derived_at, None);
    let json = serde_json::to_string(prov).unwrap();
    assert!(
        !json.contains("derived_at"),
        "absent field must not serialize: {json}"
    );
}

#[test]
fn references_provenance_unknown_field_is_rejected() {
    // Spec 028 AC-3: a genuinely-unknown provenance field still trips
    // `deny_unknown_fields`; the gate is not weakened by the additive field.
    let src = "---\nid: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
references:\n  - { provenance: { kind: k, ref: \"r\", bogus: \"x\" } }\n---\n";
    assert!(parse_frontmatter(src).is_err());
}
