//! The typed relationship edges and the bootstrap `origin` marker.
//!
//! Eight edge types; seven are ownership-bearing and `references` is the only
//! non-owning one (the coupling gate ignores it). `origin` is a bootstrap
//! marker, **not** an edge (see `docs/design/00-architecture.md` §2.1).
//!
//! `establishes` is a bare `Vec<Unit>`; `supersedes` is `Vec<SupersedeItem>` (a
//! bare predecessor id for full supersession, or a structured partial item,
//! spec 019); `amends` is `Vec<String>` of spec ids; the remaining edges are
//! lists of the item structs below. Each
//! item uses `deny_unknown_fields` so a misspelled key produces a clear error
//! rather than silently overflowing. `extends`/`refines` items accept the
//! predecessor dialect's `paths:` list as authoring sugar (spec 014): the
//! parser expands it to N single-`unit` items, so the sugar never reaches
//! `registry.json`.

use serde::{Deserialize, Serialize};

use crate::unit::Unit;

/// `extends: [{ spec, unit? | paths?, nature? }]`: adds surface to a
/// predecessor. `paths:` is parse-time sugar for N file units (spec 014).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendItem {
    /// The predecessor spec id being extended.
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Authoring sugar only: always `None` after parse (spec 014 §3.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    /// Free-text nature hint (e.g. `additive`, `wrapping`); validated by lint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature: Option<String>,
}

/// `refines: [{ aspect, unit? | paths?, refines_specs? }]`: tightens a named
/// aspect. `paths:` is parse-time sugar for N file units (spec 014).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefineItem {
    /// The named aspect being refined.
    pub aspect: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Authoring sugar only: always `None` after parse (spec 014 §3.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refines_specs: Vec<String>,
}

/// Expand the `paths:` sugar on `extends` items (spec 014 §3.2): one item per
/// path, in authored order, every other field copied. `unit` + `paths`
/// together, or an empty `paths` list, is a grammar error (the V-002 class).
pub(crate) fn expand_extend_paths(items: Vec<ExtendItem>) -> Result<Vec<ExtendItem>, String> {
    let mut out = Vec::with_capacity(items.len());
    for mut item in items {
        let Some(paths) = item.paths.take() else {
            out.push(item);
            continue;
        };
        if item.unit.is_some() {
            return Err(format!(
                "extends item for spec '{}' cannot carry both unit: and paths:",
                item.spec
            ));
        }
        if paths.is_empty() {
            return Err(format!(
                "extends item for spec '{}' has an empty paths: list",
                item.spec
            ));
        }
        for path in paths {
            out.push(ExtendItem {
                spec: item.spec.clone(),
                unit: Some(Unit::File { path }),
                paths: None,
                nature: item.nature.clone(),
            });
        }
    }
    Ok(out)
}

/// Expand the `paths:` sugar on `refines` items, symmetric with
/// [`expand_extend_paths`].
pub(crate) fn expand_refine_paths(items: Vec<RefineItem>) -> Result<Vec<RefineItem>, String> {
    let mut out = Vec::with_capacity(items.len());
    for mut item in items {
        let Some(paths) = item.paths.take() else {
            out.push(item);
            continue;
        };
        if item.unit.is_some() {
            return Err(format!(
                "refines item for aspect '{}' cannot carry both unit: and paths:",
                item.aspect
            ));
        }
        if paths.is_empty() {
            return Err(format!(
                "refines item for aspect '{}' has an empty paths: list",
                item.aspect
            ));
        }
        for path in paths {
            out.push(RefineItem {
                aspect: item.aspect.clone(),
                unit: Some(Unit::File { path }),
                paths: None,
                refines_specs: item.refines_specs.clone(),
            });
        }
    }
    Ok(out)
}

/// `co_authority: [{ unit, with_specs? }]`: shares a section with other specs.
///
/// `unit` must be a [`Unit::Section`] (validated by the compiler in Phase 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoAuthorityItem {
    pub unit: Unit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_specs: Vec<String>,
}

/// `constrains: [{ flavor? | kind?, unit?, note?, target_specs? }]`: asserts an
/// invariant others must respect.
///
/// Two shapes coexist (spec 018): a **path-scoped** constraint carries a `unit:`
/// (the canonical `invariant-freeze` over a file/schema); a **spec-scoped**
/// constraint carries `target_specs:` and no unit (a sequencing/ordering plan
/// over other specs). `flavor` and `kind` are interchangeable, documentary
/// discriminators (synonyms; the predecessor dialect uses both spellings);
/// neither is gate-load-bearing. The compiler requires at least one of `unit` or
/// `target_specs` (V-011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstrainItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Documentary constraint classification (e.g. `invariant-freeze`). Synonym
    /// of `kind`; both are accepted and preserved verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flavor: Option<String>,
    /// Documentary constraint classification: the alternative spelling of
    /// `flavor` (e.g. `sequencing-plan`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_specs: Vec<String>,
}

/// The scope of a `supersedes` edge (spec 019): a whole-spec transfer or a
/// unit-scoped one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SupersedeScope {
    /// The successor inherits the predecessor's entire authority surface.
    #[default]
    Full,
    /// The successor takes over only the named `unit` (additive: the
    /// predecessor keeps everything else, and keeps the unit too).
    Partial,
}

/// One `supersedes` entry. A bare predecessor id and `{ spec, scope: full }`
/// both mean full supersession; only a `partial` item carries a `unit` (spec
/// 019). The full form normalizes to [`SupersedeItem::Full`] at parse time, so a
/// corpus that uses only full supersession emits a byte-identical bare-string
/// `supersedes` array: the registry wire is unchanged for every existing
/// adopter; only a partial item serializes as an object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SupersedeItem {
    /// Full supersession: the predecessor id alone.
    Full(String),
    /// A structured item (`{ spec, scope?, unit?, note?, rationale? }`).
    Scoped(SupersedeScoped),
}

/// The structured `supersedes` item shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupersedeScoped {
    /// The predecessor spec id being superseded.
    pub spec: String,
    #[serde(default)]
    pub scope: SupersedeScope,
    /// For a `partial` scope: the unit whose authority transfers. A partial item
    /// with no unit is a documentary lifecycle marker; it transfers nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

impl SupersedeItem {
    /// The predecessor spec id, regardless of form.
    pub fn spec(&self) -> &str {
        match self {
            SupersedeItem::Full(id) => id,
            SupersedeItem::Scoped(s) => &s.spec,
        }
    }

    /// True for a full (whole-spec) supersession. (After normalization a
    /// `Scoped` is always partial, but this stays correct pre-normalization.)
    pub fn is_full(&self) -> bool {
        match self {
            SupersedeItem::Full(_) => true,
            SupersedeItem::Scoped(s) => s.scope == SupersedeScope::Full,
        }
    }

    /// The unit a *partial* supersession scopes its transfer to, if any.
    pub fn partial_unit(&self) -> Option<&Unit> {
        match self {
            SupersedeItem::Scoped(s) if s.scope == SupersedeScope::Partial => s.unit.as_ref(),
            _ => None,
        }
    }

    /// Rewrite the predecessor id in place (used by compile-time short-id
    /// resolution; spec 016/019).
    pub fn set_spec(&mut self, id: String) {
        match self {
            SupersedeItem::Full(s) => *s = id,
            SupersedeItem::Scoped(s) => s.spec = id,
        }
    }
}

/// Normalize `supersedes` items (spec 019): a `Scoped` item with full scope
/// carries no information beyond its id, so it collapses to the bare-string
/// [`SupersedeItem::Full`] form, keeping `{ scope: full }` (OAP spec 073) and a
/// bare id byte-identical on the wire. Partial items pass through unchanged.
pub(crate) fn normalize_supersedes(items: Vec<SupersedeItem>) -> Vec<SupersedeItem> {
    items
        .into_iter()
        .map(|item| match item {
            SupersedeItem::Scoped(s) if s.scope == SupersedeScope::Full => {
                SupersedeItem::Full(s.spec)
            }
            other => other,
        })
        .collect()
}

/// `references: [{ unit? | provenance?, role? }]`: the non-owning edge.
///
/// `unit` and `provenance` are mutually exclusive (enforced by the compiler).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    /// Open-vocabulary role hint (e.g. `context`, `evidence`, `precedent`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// A provenance reference carried on a `references` edge:
/// `{ kind, ref, derived_at? }`.
///
/// `kind` keys into `config.provenance.uri_schemes` to validate `ref`'s scheme.
/// `derived_at` is a generic, optional ISO-8601 timestamp recording when the
/// reference was derived (spec 028): additive and preserved verbatim, with no
/// timestamp-format validation in the type (an adopter that wants format
/// enforcement adds a lint). `deny_unknown_fields` is preserved: the field is
/// now known, not a hole in the schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub kind: String,
    /// The provenance URI. (`ref` is a Rust keyword, hence the rename.)
    #[serde(rename = "ref")]
    pub reference: String,
    /// Optional ISO-8601 timestamp recording when this reference was derived
    /// (spec 028). Absent items do not serialize the field, so existing goldens
    /// stay byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_at: Option<String>,
}

/// The `origin` bootstrap marker, NOT a relationship edge.
///
/// `retroactive: true` declares authority held since before the graph existed.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    #[serde(default)]
    pub retroactive: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}
