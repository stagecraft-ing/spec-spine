//! The spec frontmatter grammar: the typed `Frontmatter` struct, the value
//! enums, the `extra_frontmatter` overflow, and the parse entry points.
//!
//! Parsing is config-free and pure. The `---`-delimited block is split out
//! ([`split_frontmatter`]), the known keys are deserialized into [`Frontmatter`],
//! and every key not in [`KNOWN_KEYS`] overflows into `extra_frontmatter` as a
//! scalar or string-list (ported from OAP/aide `spec-types`). Whether an extra
//! key is *warned about* is a lint concern driven by
//! `config.frontmatter.extra_known_keys`, not a parse concern — so this module
//! needs no `Config`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::edges::{CoAuthorityItem, ConstrainItem, ExtendItem, Origin, ReferenceItem, RefineItem};
use crate::error::{Error, Result};
use crate::unit::Unit;

/// Lifecycle status of a spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Approved,
    Superseded,
    Retired,
}

/// Risk level (optional descriptive metadata).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    High,
    Critical,
}

/// Implementation progress (optional descriptive metadata).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Implementation {
    Pending,
    InProgress,
    Complete,
    #[serde(rename = "n-a")]
    Na,
    Deferred,
}

/// A scalar or string-list value carried in `extra_frontmatter`.
///
/// The grammar caps extra frontmatter to scalars and string lists; a complex
/// (nested map / mixed list) value under an unknown key is a parse error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtraValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<String>),
}

/// Every frontmatter key modeled as a struct field. Keys outside this set
/// overflow into `extra_frontmatter`.
pub const KNOWN_KEYS: &[&str] = &[
    // required + descriptive
    "id",
    "title",
    "status",
    "created",
    "summary",
    "authors",
    "owner",
    "kind",
    "domain",
    "risk",
    "implementation",
    "depends_on",
    "code_aliases",
    "feature_branch",
    // typed edges (8)
    "establishes",
    "extends",
    "refines",
    "supersedes",
    "amends",
    "co_authority",
    "constrains",
    "references",
    // lifecycle / amendment
    "superseded_by",
    "retirement_rationale",
    "amends_sections",
    "unamendable",
    "amendment_record",
    // bootstrap marker
    "origin",
];

/// The typed, parsed frontmatter of a `spec.md`.
///
/// Field names are `snake_case` to match the authored YAML. Unknown keys are
/// **not** captured by serde (unknown fields are ignored on deserialize); they
/// are collected separately into `extra_frontmatter` by [`parse_frontmatter`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    // --- required ---
    pub id: String,
    pub title: String,
    pub status: Status,
    pub created: String,
    pub summary: String,

    // --- optional descriptive ---
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<Implementation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code_aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_branch: Option<String>,

    // --- typed edges (8) ---
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub establishes: Vec<Unit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<ExtendItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refines: Vec<RefineItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub amends: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub co_authority: Vec<CoAuthorityItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constrains: Vec<ConstrainItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<ReferenceItem>,

    // --- lifecycle / amendment ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retirement_rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub amends_sections: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unamendable: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amendment_record: Option<String>,

    // --- bootstrap marker ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,

    // --- overflow (populated by parse_frontmatter, never by serde) ---
    #[serde(skip)]
    pub extra_frontmatter: BTreeMap<String, ExtraValue>,
}

/// Split a `spec.md` source into its `(frontmatter_yaml, body)` halves.
///
/// Strips a leading UTF-8 BOM, requires the file to open with a `---` fence, and
/// reads to the next line that is exactly `---`. Line-ending agnostic (uses
/// [`str::lines`], which handles both `\n` and `\r\n`). Returns owned strings.
pub fn split_frontmatter(src: &str) -> Result<(String, String)> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut lines = src.lines();

    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => {
            return Err(Error::Parse(
                "spec.md must begin with a YAML frontmatter block delimited by '---'".into(),
            ));
        }
    }

    let mut frontmatter = String::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }
    if !closed {
        return Err(Error::Parse(
            "unterminated frontmatter block (missing closing '---')".into(),
        ));
    }

    let mut body = String::new();
    for line in lines {
        body.push_str(line);
        body.push('\n');
    }

    Ok((frontmatter, body))
}

/// Parse the frontmatter block of a `spec.md` into a typed [`Frontmatter`].
///
/// Pure and config-free. Returns [`Error::Parse`] for a malformed block, a
/// missing required key, an invalid enum value, or a non-scalar value under an
/// unknown (overflow) key.
pub fn parse_frontmatter(src: &str) -> Result<Frontmatter> {
    let (yaml, _body) = split_frontmatter(src)?;

    let value: serde_yaml::Value = serde_yaml::from_str(&yaml)
        .map_err(|e| Error::Parse(format!("invalid YAML frontmatter: {e}")))?;

    let mapping = value
        .as_mapping()
        .ok_or_else(|| Error::Parse("frontmatter must be a YAML mapping".into()))?;

    // Known keys (unknown keys are ignored here; collected below).
    let mut frontmatter: Frontmatter = serde_yaml::from_value(value.clone())
        .map_err(|e| Error::Parse(format!("invalid frontmatter: {e}")))?;

    // Overflow: every key not in KNOWN_KEYS becomes an extra_frontmatter entry.
    for (k, v) in mapping {
        let key = match k.as_str() {
            Some(s) => s,
            None => return Err(Error::Parse("frontmatter keys must be strings".into())),
        };
        if KNOWN_KEYS.contains(&key) {
            continue;
        }
        if let Some(extra) = yaml_to_extra(v)? {
            frontmatter.extra_frontmatter.insert(key.to_string(), extra);
        }
    }

    Ok(frontmatter)
}

/// Convert a YAML scalar / string-list into an [`ExtraValue`]. `Null` yields
/// `None` (the key is dropped); a nested map or mixed list is a parse error.
fn yaml_to_extra(v: &serde_yaml::Value) -> Result<Option<ExtraValue>> {
    use serde_yaml::Value;
    Ok(match v {
        Value::Null => None,
        Value::Bool(b) => Some(ExtraValue::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(ExtraValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                Some(ExtraValue::Float(f))
            } else {
                return Err(Error::Parse(
                    "unsupported numeric extra-frontmatter value".into(),
                ));
            }
        }
        Value::String(s) => Some(ExtraValue::Str(s.clone())),
        Value::Sequence(seq) => {
            let mut list = Vec::with_capacity(seq.len());
            for item in seq {
                match item.as_str() {
                    Some(s) => list.push(s.to_string()),
                    None => {
                        return Err(Error::Parse(
                            "extra-frontmatter lists must contain only strings".into(),
                        ));
                    }
                }
            }
            Some(ExtraValue::List(list))
        }
        Value::Mapping(_) | Value::Tagged(_) => {
            return Err(Error::Parse(
                "extra-frontmatter values must be scalars or string lists, not nested maps".into(),
            ));
        }
    })
}
