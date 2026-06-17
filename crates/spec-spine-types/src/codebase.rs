//! Codebase-index DTOs: the code-as-source view, emitted by `spec-spine index`
//! as `index.json`. Field names serialize to `camelCase`. Shapes are ported from
//! OAP `codebase-index.schema.json` (3.0.0), pruned to the generic v1 surface and
//! re-versioned to this library's own schema line (currently `0.3.0`; see
//! [`crate::version::INDEX_SCHEMA_VERSION`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::unit::Unit;

/// The compiled codebase index: `index.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebaseIndex {
    /// `MAJOR.MINOR.PATCH`; see [`crate::version::INDEX_SCHEMA_VERSION`].
    pub schema_version: String,
    pub build: IndexBuild,
    /// Layer 1: the discovered compilation units.
    pub packages: Vec<PackageRecord>,
    /// Layer 2: spec ↔ code traceability.
    pub traceability: Traceability,
    pub diagnostics: Diagnostics,
}

/// Deterministic build metadata embedded in `index.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexBuild {
    pub indexer_id: String,
    pub indexer_version: String,
    pub repo_root: String,
    /// SHA-256 over the normalized, path-sorted manifest + spec + extra inputs.
    pub content_hash: String,
    /// Per-slice content hashes (spec 012): one entry per `[index.slices]`
    /// key, same normalization as `content_hash`. Absent when no slices are
    /// configured; loaders tolerate absence (additive MINOR).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slice_hashes: BTreeMap<String, String>,
}

/// The kind of a discovered compilation unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    RustLib,
    RustBin,
    RustLibBin,
    NpmPackage,
    NpmWorkspace,
}

/// A discovered compilation unit (a Rust crate or an npm package).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRecord {
    pub name: String,
    /// Repo-relative POSIX path to the package directory.
    pub path: String,
    pub kind: PackageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// The owning spec id declared in the manifest's metadata namespace, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
}

/// Layer 2: how the corpus maps onto the code, and what is unmapped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Traceability {
    pub mappings: Vec<TraceMapping>,
    /// Specs claiming code that resolves to no location.
    pub orphaned_specs: Vec<String>,
    /// Package paths with no governing spec.
    pub untraced_code: Vec<String>,
}

/// One spec's mapping onto the code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceMapping {
    pub spec_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub amends: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amendment_record: Option<String>,
    /// Flat path ownership (whole-file granularity).
    pub implementing_paths: Vec<ImplementingPath>,
    /// Typed-unit ownership with physical line-spans.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_units: Vec<ResolvedUnit>,
}

/// Where a path-level linkage came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceSource {
    /// A spec's ownership edge (`establishes`/`extends`/…).
    SpecEdge,
    /// A manifest `[package.metadata.<ns>].spec` / `"<ns>".spec` key.
    ManifestMetadata,
    /// A `// Spec: …` file-root comment header.
    CommentHeader,
    /// Two or more sources agree on this path.
    Multiple,
}

/// A path claimed by a spec, with its linkage source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementingPath {
    pub path: String,
    pub source: TraceSource,
}

/// Which edge field a resolved unit came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceField {
    Establishes,
    Extends,
    Refines,
    Supersedes,
    Amends,
    CoAuthority,
    Constrains,
    References,
}

impl SourceField {
    /// Ownership-bearing? `references` is the only non-owning edge.
    pub fn is_ownership(self) -> bool {
        !matches!(self, SourceField::References)
    }
}

/// A typed unit resolved to its physical locations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedUnit {
    pub unit: Unit,
    pub source_field: SourceField,
    /// `false` only for `references` units (the gate ignores them).
    pub ownership: bool,
    /// Resolved locations (empty when resolution failed → a diagnostic).
    pub locations: Vec<ResolvedLocation>,
}

/// A physical location: a file and an optional line-span (absent ⇒ whole file).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLocation {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<LineSpan>,
}

/// An inclusive, 1-based line span, aligned with `git diff -U0` hunk ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineSpan {
    pub start_line: usize,
    pub end_line: usize,
}

impl LineSpan {
    pub fn new(start_line: usize, end_line: usize) -> Self {
        LineSpan {
            start_line,
            end_line,
        }
    }
}

/// Index diagnostics, split by tier. `I-003`..`I-009` (in `errors`) block `check`.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub warnings: Vec<Diagnostic>,
    pub errors: Vec<Diagnostic>,
}

impl Diagnostics {
    /// No diagnostics of either tier (used to omit an empty block from a shard).
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty() && self.errors.is_empty()
    }
}

// ===== sharded committed form (spec 024) =====
//
// The committed index is stored as one file per authority unit so two PRs that
// touch different specs/packages write disjoint files and never conflict
// textually. The aggregate [`CodebaseIndex`] above stays the universal in-memory
// currency: the emitter projects it to shards, and a reader assembles it back
// from the shard set (orphans / untraced code / `build.contentHash` are pure
// functions of the shards, recomputed on read, never committed).

/// One spec's traceability shard: `<derived>/codebase-index/by-spec/<id>.json`.
/// A PR confined to spec X's inputs rewrites only X's shard (spec 024 FR-002).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSpecShard {
    /// `schemaVersion`; see [`crate::version::INDEX_SCHEMA_VERSION`].
    pub schema_version: String,
    /// SHA-256 over this spec's inputs: its `spec.md`, the source files backing
    /// its resolved symbol/section/module spans, and the global-inputs scalar
    /// (config + `extra_hashed_inputs`). Self-describing per-shard staleness.
    pub shard_hash: String,
    /// This spec's mapping onto the code.
    pub mapping: TraceMapping,
    /// Resolver diagnostics scoped to this spec (`I-003`..`I-009` block `check`).
    /// Omitted when empty, so a clean spec's shard carries no diagnostics block.
    #[serde(default, skip_serializing_if = "Diagnostics::is_empty")]
    pub diagnostics: Diagnostics,
}

/// One package's inventory shard: `<derived>/codebase-index/by-package/<slug>.json`.
/// A PR confined to a package's manifest rewrites only that package's shard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexPackageShard {
    /// `schemaVersion`; see [`crate::version::INDEX_SCHEMA_VERSION`].
    pub schema_version: String,
    /// SHA-256 over this package's manifest (governance projection) folded with
    /// the global-inputs scalar.
    pub shard_hash: String,
    /// The discovered compilation unit.
    pub package: PackageRecord,
}

/// A single index diagnostic (`I-###`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
