//! The typed relationship edges and the bootstrap `origin` marker.
//!
//! Eight edge types; seven are ownership-bearing and `references` is the only
//! non-owning one (the coupling gate ignores it). `origin` is a bootstrap
//! marker, **not** an edge (see `docs/design/00-architecture.md` §2.1).
//!
//! `establishes` is a bare `Vec<Unit>`; `supersedes`/`amends` are `Vec<String>`
//! of spec ids; the remaining edges are lists of the item structs below. Each
//! item uses `deny_unknown_fields` so the legacy `paths:` form (replaced by
//! `unit:`) produces a clear error rather than silently overflowing.

use serde::{Deserialize, Serialize};

use crate::unit::Unit;

/// `extends: [{ spec, unit?, nature? }]` — adds surface to a predecessor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendItem {
    /// The predecessor spec id being extended.
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Free-text nature hint (e.g. `additive`, `wrapping`); validated by lint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature: Option<String>,
}

/// `refines: [{ aspect, unit?, refines_specs? }]` — tightens a named aspect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefineItem {
    /// The named aspect being refined.
    pub aspect: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refines_specs: Vec<String>,
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
