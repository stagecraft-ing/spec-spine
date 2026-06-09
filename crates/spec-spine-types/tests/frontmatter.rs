//! Frontmatter tests: split, required keys, enums, extra-frontmatter overflow.

use spec_spine_types::{Error, ExtraValue, Status, parse_frontmatter, split_frontmatter};

const MINIMAL: &str = "---\n\
id: \"001-thing\"\n\
title: \"A thing\"\n\
status: draft\n\
created: \"2026-06-08\"\n\
summary: \"Does a thing.\"\n\
---\n\
# Body\n";

#[test]
fn splits_frontmatter_and_body() {
    let (fm, body) = split_frontmatter(MINIMAL).unwrap();
    assert!(fm.contains("id:"));
    assert!(body.contains("# Body"));
    assert!(!body.contains("id:"));
}

#[test]
fn split_handles_bom_and_crlf() {
    let src = "\u{feff}---\r\nid: \"x\"\r\n---\r\nbody\r\n";
    let (fm, body) = split_frontmatter(src).unwrap();
    assert!(fm.contains("id:"));
    assert_eq!(body.trim(), "body");
}

#[test]
fn missing_opening_fence_is_parse_error() {
    let e = split_frontmatter("no fence here\n").unwrap_err();
    assert!(matches!(e, Error::Parse(_)));
}

#[test]
fn unterminated_block_is_parse_error() {
    let e = split_frontmatter("---\nid: x\nno closing fence\n").unwrap_err();
    assert!(matches!(e, Error::Parse(_)));
}

#[test]
fn parses_required_keys() {
    let fm = parse_frontmatter(MINIMAL).unwrap();
    assert_eq!(fm.id, "001-thing");
    assert_eq!(fm.title, "A thing");
    assert_eq!(fm.status, Status::Draft);
    assert_eq!(fm.created, "2026-06-08");
    assert!(fm.extra_frontmatter.is_empty());
}

#[test]
fn missing_required_key_is_parse_error() {
    let src = "---\ntitle: \"no id\"\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: x\n---\n";
    let e = parse_frontmatter(src).unwrap_err();
    assert!(matches!(e, Error::Parse(_)));
    assert_eq!(e.exit_code(), 3);
}

#[test]
fn invalid_status_enum_is_parse_error() {
    let src = "---\nid: x\ntitle: t\nstatus: bogus\ncreated: \"2026-06-08\"\nsummary: s\n---\n";
    assert!(matches!(
        parse_frontmatter(src).unwrap_err(),
        Error::Parse(_)
    ));
}

#[test]
fn extra_frontmatter_overflow_scalars_and_lists() {
    let src = "---\n\
id: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
custom_flag: true\n\
custom_count: 7\n\
custom_tags: [\"a\", \"b\"]\n\
custom_note: \"hello\"\n\
---\n";
    let fm = parse_frontmatter(src).unwrap();
    assert_eq!(
        fm.extra_frontmatter.get("custom_flag"),
        Some(&ExtraValue::Bool(true))
    );
    assert_eq!(
        fm.extra_frontmatter.get("custom_count"),
        Some(&ExtraValue::Int(7))
    );
    assert_eq!(
        fm.extra_frontmatter.get("custom_tags"),
        Some(&ExtraValue::List(vec!["a".into(), "b".into()]))
    );
    assert_eq!(
        fm.extra_frontmatter.get("custom_note"),
        Some(&ExtraValue::Str("hello".into()))
    );
}

#[test]
fn complex_extra_value_is_rejected() {
    // A nested map under an unknown key violates the scalar/string-list cap.
    let src = "---\n\
id: x\ntitle: t\nstatus: draft\ncreated: \"2026-06-08\"\nsummary: s\n\
custom_obj:\n  nested: 1\n\
---\n";
    assert!(matches!(
        parse_frontmatter(src).unwrap_err(),
        Error::Parse(_)
    ));
}
