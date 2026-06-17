//! Registry DTOs: the spec-as-source view, emitted by `compile` as
//! `registry.json`. Field names serialize to `camelCase` (the JSON contract),
//! distinct from the `snake_case` authored [`crate::Frontmatter`] grammar.
//!
//! The compiler (Phase 2) populates these from parsed frontmatter plus computed
//! fields (`spec_path`, `section_headings`, the content hash). Shapes are ported
//! from OAP `registry.schema.json` (`featureRecord`, `build`, `violation`),
//! pruned to the generic v1 surface; overlay fields (compliance, factory,
//! capability/registry/profile) are intentionally absent (see §10.4).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::edges::{
    CoAuthorityItem, ConstrainItem, ExtendItem, Origin, ReferenceItem, RefineItem, SupersedeItem,
};
use crate::frontmatter::{Implementation, Risk, Status};
use crate::unit::Unit;

/// The compiled registry: `registry.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    /// `MAJOR.MINOR.PATCH`; see [`crate::version::REGISTRY_SCHEMA_VERSION`].
    pub spec_version: String,
    pub build: Build,
    pub specs: Vec<SpecRecord>,
    pub validation: ValidationReport,
}

/// Deterministic build metadata embedded in `registry.json` (no timestamps:
/// the wall clock lives in the separate, non-deterministic `build-meta.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub compiler_id: String,
    pub compiler_version: String,
    /// The input root the registry was compiled from, repo-relative (e.g. `.`).
    pub input_root: String,
    /// SHA-256 over the normalized, path-sorted spec inputs (64 lowercase hex).
    pub content_hash: String,
}

/// One spec's entry in the registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecRecord {
    // --- required ---
    pub id: String,
    pub title: String,
    pub status: Status,
    pub created: String,
    pub summary: String,
    /// Repo-relative path: `specs/NNN-slug/spec.md`.
    pub spec_path: String,

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
    /// Markdown headings discovered in the spec body (anchors for sections).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub section_headings: Vec<String>,

    // --- typed edges (8) ---
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub establishes: Vec<Unit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<ExtendItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refines: Vec<RefineItem>,
    /// Full supersession serializes as a bare predecessor id; a partial item
    /// serializes as an object (spec 019).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<SupersedeItem>,
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

    // --- overflow ---
    /// Declared keys carry any JSON value (spec 013); undeclared keys are
    /// scalars or string arrays.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_frontmatter: BTreeMap<String, serde_json::Value>,
}

// ===== sharded committed form (spec 024) =====
//
// The committed registry is stored as one file per spec so two PRs that add or
// edit different specs write disjoint files and never conflict textually on a
// shared content-hash line. The aggregate [`Registry`] above stays the universal
// in-memory currency: the compiler projects it to shards, and a reader assembles
// it back from the shard set. The aggregate `validation` and `build.contentHash`
// are recomputed on read (cross-spec checks like duplicate-id / dangling edges
// are pure functions of the assembled record set), never committed.

/// One spec's registry shard: `<derived>/spec-registry/by-spec/<id>.json`.
/// A PR that adds or edits spec X rewrites only X's shard (spec 024 FR-002).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySpecShard {
    /// `specVersion`; see [`crate::version::REGISTRY_SCHEMA_VERSION`].
    pub spec_version: String,
    /// SHA-256 over this spec's `spec.md` (the registry's only hashed input,
    /// matching the pre-shard `build.contentHash` input set). Self-describing
    /// per-shard staleness.
    pub shard_hash: String,
    /// This spec's compiled record.
    pub record: SpecRecord,
    /// Validation findings that are a pure function of THIS spec (V-001/002/
    /// 005/006/007/011/012/013). Cross-spec findings (duplicate id/prefix,
    /// dangling edges) are recomputed on read from the assembled record set, so
    /// they are never stored here (storing them would make a sibling spec's PR
    /// stale this shard). Omitted when empty: a clean spec carries none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local_violations: Vec<Violation>,
}

/// Non-deterministic build metadata sidecar (`build-meta.json`). The wall-clock
/// `built_at` lives here, never in `registry.json`, and is excluded from every
/// determinism/golden check. The CLI populates `built_at`; the library never
/// reads the clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildMeta {
    pub schema_version: String,
    pub built_at: String,
    pub compiler_id: String,
    pub compiler_version: String,
}

/// Severity tier of a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single validation/lint/coupling diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    /// A stable code such as `V-001`, `L-003`, `I-004`.
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// The registry's validation summary. `passed` is false iff any `error`-tier
/// violation is present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub passed: bool,
    #[serde(default)]
    pub violations: Vec<Violation>,
}

impl ValidationReport {
    /// Build a report from violations, setting `passed` per the error-tier rule.
    pub fn from_violations(violations: Vec<Violation>) -> Self {
        let passed = !violations.iter().any(|v| v.severity == Severity::Error);
        ValidationReport { passed, violations }
    }
}
