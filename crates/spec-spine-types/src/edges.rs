//! The typed relationship edges and the bootstrap `origin` marker.
//!
//! Eight edge types; seven are ownership-bearing and `references` is the only
//! non-owning one (the coupling gate ignores it). `origin` is a bootstrap
//! marker, **not** an edge (see `docs/design/00-architecture.md` §2.1).
//!
//! `establishes` is a bare `Vec<Unit>`; `supersedes`/`amends` are `Vec<String>`
//! of spec ids; the remaining edges are lists of the item structs below. Each
//! item uses `deny_unknown_fields` so a misspelled key produces a clear error
//! rather than silently overflowing. `extends`/`refines` items accept the
//! predecessor dialect's `paths:` list as authoring sugar (spec 014): the
//! parser expands it to N single-`unit` items, so the sugar never reaches
//! `registry.json`.

use serde::{Deserialize, Serialize};

use crate::unit::Unit;

/// `extends: [{ spec, unit? | paths?, nature? }]` — adds surface to a
/// predecessor. `paths:` is parse-time sugar for N file units (spec 014).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendItem {
    /// The predecessor spec id being extended.
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Authoring sugar only — always `None` after parse (spec 014 §3.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    /// Free-text nature hint (e.g. `additive`, `wrapping`); validated by lint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature: Option<String>,
}

/// `refines: [{ aspect, unit? | paths?, refines_specs? }]` — tightens a named
/// aspect. `paths:` is parse-time sugar for N file units (spec 014).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefineItem {
    /// The named aspect being refined.
    pub aspect: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Authoring sugar only — always `None` after parse (spec 014 §3.2).
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

/// Expand the `paths:` sugar on `refines` items — symmetric with
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

/// `co_authority: [{ unit, with_specs? }]` — shares a section with other specs.
///
/// `unit` must be a [`Unit::Section`] (validated by the compiler in Phase 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoAuthorityItem {
    pub unit: Unit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_specs: Vec<String>,
}

/// `constrains: [{ unit, note?, target_specs? }]` — asserts an invariant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstrainItem {
    pub unit: Unit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_specs: Vec<String>,
}

/// `references: [{ unit? | provenance?, role? }]` — the non-owning edge.
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

/// A provenance reference carried on a `references` edge: `{ kind, ref }`.
///
/// `kind` keys into `config.provenance.uri_schemes` to validate `ref`'s scheme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub kind: String,
    /// The provenance URI. (`ref` is a Rust keyword, hence the rename.)
    #[serde(rename = "ref")]
    pub reference: String,
}

/// The `origin` bootstrap marker — NOT a relationship edge.
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
