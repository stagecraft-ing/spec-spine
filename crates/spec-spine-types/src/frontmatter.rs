//! The spec frontmatter grammar: the typed `Frontmatter` struct, the value
//! enums, the `extra_frontmatter` overflow, and the parse entry points.
//!
//! Parsing is pure. The `---`-delimited block is split out
//! ([`split_frontmatter`]), the known keys are deserialized into [`Frontmatter`],
//! and every key not in [`KNOWN_KEYS`] overflows into `extra_frontmatter` as a
//! `serde_json::Value`. The value domain splits on declaration (spec 013):
//! a key listed in `config.frontmatter.extra_known_keys` (passed to
//! [`parse_frontmatter_with`]) carries **any JSON-representable YAML value**,
//! transported verbatim under canonical-JSON normalization; an undeclared key
//! keeps the original scalar / string-list restriction (the anti-bulk-YAML
//! guard, ported from OAP/aide `spec-types`). [`parse_frontmatter`] is the
//! declared-nothing form, byte-compatible with pre-013 behavior.

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

/// A failed frontmatter parse, classified for the compiler's V-code mapping
/// (spec 013 §3.3).
#[derive(Clone, Debug)]
pub enum FrontmatterIssue {
    /// Malformed YAML or a grammar violation — the V-002 class.
    Malformed(String),
    /// A DECLARED extra key whose value JSON cannot represent (non-string
    /// mapping key, YAML tag, non-finite number) — the V-013 class.
    UnrepresentableDeclared { key: String, detail: String },
}

impl From<FrontmatterIssue> for Error {
    fn from(issue: FrontmatterIssue) -> Self {
        match issue {
            FrontmatterIssue::Malformed(m) => Error::Parse(m),
            FrontmatterIssue::UnrepresentableDeclared { key, detail } => Error::Parse(format!(
                "declared extra-frontmatter key '{key}' carries an unrepresentable YAML value: {detail}"
            )),
        }
    }
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
    pub extra_frontmatter: BTreeMap<String, serde_json::Value>,
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

/// Parse the frontmatter block of a `spec.md` into a typed [`Frontmatter`],
/// treating every extra key as undeclared (pre-013 behavior, kept for
/// config-free callers).
///
/// Returns [`Error::Parse`] for a malformed block, a missing required key, an
/// invalid enum value, or a non-scalar value under an unknown (overflow) key.
pub fn parse_frontmatter(src: &str) -> Result<Frontmatter> {
    parse_frontmatter_with(src, &[]).map_err(Into::into)
}

/// Parse with declared-key awareness (spec 013): a key listed in `declared`
/// (the adopter's `frontmatter.extra_known_keys`) carries any
/// JSON-representable YAML value, transported verbatim; an undeclared key
/// keeps the scalar / string-list restriction. A top-level `null` value drops
/// the key on either path.
pub fn parse_frontmatter_with(
    src: &str,
    declared: &[String],
) -> std::result::Result<Frontmatter, FrontmatterIssue> {
    let malformed = |m: String| FrontmatterIssue::Malformed(m);
    let (yaml, _body) = split_frontmatter(src).map_err(|e| {
        malformed(match e {
            Error::Parse(m) => m,
            other => other.to_string(),
        })
    })?;

    let value: serde_yaml::Value = serde_yaml::from_str(&yaml)
        .map_err(|e| malformed(format!("invalid YAML frontmatter: {e}")))?;

    let mapping = value
        .as_mapping()
        .ok_or_else(|| malformed("frontmatter must be a YAML mapping".into()))?;

    // Known keys (unknown keys are ignored here; collected below).
    let mut frontmatter: Frontmatter = serde_yaml::from_value(value.clone())
        .map_err(|e| malformed(format!("invalid frontmatter: {e}")))?;

    // Overflow: every key not in KNOWN_KEYS becomes an extra_frontmatter entry.
    for (k, v) in mapping {
        let key = match k.as_str() {
            Some(s) => s,
            None => return Err(malformed("frontmatter keys must be strings".into())),
        };
        if KNOWN_KEYS.contains(&key) {
            continue;
        }
        let json = if declared.iter().any(|d| d == key) {
            yaml_to_json(v).map_err(|detail| FrontmatterIssue::UnrepresentableDeclared {
                key: key.to_string(),
                detail,
            })?
        } else {
            yaml_to_extra(v).map_err(malformed)?
        };
        if json.is_null() {
            continue;
        }
        frontmatter.extra_frontmatter.insert(key.to_string(), json);
    }

    // `paths:` sugar on extends/refines items (spec 014): expanded here, in
    // the shared parse path, so every consumer (compile, index, lint, couple)
    // sees only single-unit edges.
    frontmatter.extends =
        crate::edges::expand_extend_paths(std::mem::take(&mut frontmatter.extends))
            .map_err(malformed)?;
    frontmatter.refines =
        crate::edges::expand_refine_paths(std::mem::take(&mut frontmatter.refines))
            .map_err(malformed)?;

    Ok(frontmatter)
}

/// The UNDECLARED-key path: scalars and string lists only (`Null` drops the
/// key); a nested map, mixed list, or tag is a grammar violation — exactly
/// the pre-013 guard.
fn yaml_to_extra(v: &serde_yaml::Value) -> std::result::Result<serde_json::Value, String> {
    use serde_yaml::Value;
    match v {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_json::Value::from(i))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| "unsupported numeric extra-frontmatter value".to_string())
            } else {
                Err("unsupported numeric extra-frontmatter value".to_string())
            }
        }
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Sequence(seq) => {
            let mut list = Vec::with_capacity(seq.len());
            for item in seq {
                match item.as_str() {
                    Some(s) => list.push(serde_json::Value::String(s.to_string())),
                    None => {
                        return Err("extra-frontmatter lists must contain only strings".to_string());
                    }
                }
            }
            Ok(serde_json::Value::Array(list))
        }
        Value::Mapping(_) | Value::Tagged(_) => Err(
            "extra-frontmatter values must be scalars or string lists, not nested maps".to_string(),
        ),
    }
}

/// The DECLARED-key path (spec 013 §3.2): full YAML → JSON conversion.
/// Mappings require string keys; tags and non-finite numbers are
/// unrepresentable. Map key order is canonicalized by the sorted
/// `serde_json::Map` (authoring order is not preserved — the price of
/// byte-identical registries).
fn yaml_to_json(v: &serde_yaml::Value) -> std::result::Result<serde_json::Value, String> {
    use serde_yaml::Value;
    match v {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_json::Value::from(i))
            } else if let Some(u) = n.as_u64() {
                Ok(serde_json::Value::from(u))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| format!("non-finite number {f} is not JSON-representable"))
            } else {
                Err("unsupported YAML number".to_string())
            }
        }
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Sequence(seq) => seq
            .iter()
            .map(yaml_to_json)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        Value::Mapping(map) => {
            let mut out = serde_json::Map::new();
            for (mk, mv) in map {
                let Some(key) = mk.as_str() else {
                    return Err("non-string mapping key is not JSON-representable".to_string());
                };
                out.insert(key.to_string(), yaml_to_json(mv)?);
            }
            Ok(serde_json::Value::Object(out))
        }
        Value::Tagged(tagged) => Err(format!(
            "YAML tag '{}' is not JSON-representable",
            tagged.tag
        )),
    }
}
