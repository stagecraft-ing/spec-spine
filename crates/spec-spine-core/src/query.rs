//! The query capability (spec 002): typed, read-only access over a loaded
//! registry. Because `Registry` is defined in `spec-spine-types`, these are free
//! functions rather than inherent methods (the orphan rule), but the surface is
//! the same: list / show / status_report / relationships, plus `load_registry`.

use serde::Serialize;
use spec_spine_types::{
    CodebaseIndex, Error, INDEX_SCHEMA_VERSION, REGISTRY_SCHEMA_VERSION, Registry, SpecRecord,
    Status, parse_semver,
};

/// Parse `registry.json` bytes into a typed [`Registry`], rejecting an unknown
/// MAJOR schema version (the versioning policy: a build understands its own
/// MAJOR line only).
pub fn load_registry(bytes: &[u8]) -> Result<Registry, Error> {
    let registry: Registry = serde_json::from_slice(bytes)
        .map_err(|e| Error::Parse(format!("invalid registry.json: {e}")))?;
    reject_unknown_major("registry", &registry.spec_version, REGISTRY_SCHEMA_VERSION)?;
    Ok(registry)
}

/// Parse `index.json` bytes into a typed [`CodebaseIndex`], rejecting an unknown
/// MAJOR schema version. The index-side overlay seam.
pub fn load_index(bytes: &[u8]) -> Result<CodebaseIndex, Error> {
    let index: CodebaseIndex = serde_json::from_slice(bytes)
        .map_err(|e| Error::Parse(format!("invalid index.json: {e}")))?;
    reject_unknown_major("index", &index.schema_version, INDEX_SCHEMA_VERSION)?;
    Ok(index)
}

fn reject_unknown_major(what: &str, found: &str, ours: &str) -> Result<(), Error> {
    let (want_major, ..) = parse_semver(ours).expect("our own version constant is semver");
    let (got_major, ..) = parse_semver(found)
        .ok_or_else(|| Error::Schema(format!("{what} schemaVersion '{found}' is not semver")))?;
    if got_major != want_major {
        return Err(Error::Schema(format!(
            "{what} schema MAJOR {got_major} is unsupported (this build understands {want_major}.x)"
        )));
    }
    Ok(())
}

/// Filter for [`list`]. Extend additively as needs grow.
#[derive(Debug, Default, Clone)]
pub struct ListFilter {
    pub status: Option<Status>,
}

/// Specs matching `filter`, in registry (id) order.
pub fn list<'a>(registry: &'a Registry, filter: &ListFilter) -> Vec<&'a SpecRecord> {
    registry
        .specs
        .iter()
        .filter(|s| filter.status.is_none_or(|st| s.status == st))
        .collect()
}

/// The `--ids-only` projection of [`list`] (spec 010 §3.1): the same filter and
/// order, reduced to bare spec ids.
pub fn list_ids<'a>(registry: &'a Registry, filter: &ListFilter) -> Vec<&'a str> {
    list(registry, filter)
        .iter()
        .map(|s| s.id.as_str())
        .collect()
}

/// One spec by id, or [`Error::NotFound`].
pub fn show<'a>(registry: &'a Registry, id: &str) -> Result<&'a SpecRecord, Error> {
    registry
        .specs
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| Error::NotFound(format!("spec '{id}'")))
}

/// Counts of specs by status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub total: usize,
    pub draft: usize,
    pub approved: usize,
    pub superseded: usize,
    pub retired: usize,
}

/// The `--nonzero-only` projection of a [`StatusReport`] (spec 010 §3.2):
/// zero-count statuses are omitted from serialization; `total` always
/// serializes and still reflects the whole corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusReportNonzero {
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retired: Option<usize>,
}

impl StatusReport {
    /// Project to the `--nonzero-only` form.
    pub fn nonzero_only(&self) -> StatusReportNonzero {
        let keep = |n: usize| (n > 0).then_some(n);
        StatusReportNonzero {
            total: self.total,
            draft: keep(self.draft),
            approved: keep(self.approved),
            superseded: keep(self.superseded),
            retired: keep(self.retired),
        }
    }
}

/// Tally specs by status.
pub fn status_report(registry: &Registry) -> StatusReport {
    let mut r = StatusReport {
        total: registry.specs.len(),
        draft: 0,
        approved: 0,
        superseded: 0,
        retired: 0,
    };
    for spec in &registry.specs {
        match spec.status {
            Status::Draft => r.draft += 1,
            Status::Approved => r.approved += 1,
            Status::Superseded => r.superseded += 1,
            Status::Retired => r.retired += 1,
        }
    }
    r
}

/// The relationship neighborhood of a spec: its outgoing id-edges and the
/// incoming edges that target it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipView {
    pub id: String,
    // outgoing
    pub depends_on: Vec<String>,
    pub supersedes: Vec<String>,
    pub amends: Vec<String>,
    // incoming (computed by scanning the corpus)
    pub superseded_by: Vec<String>,
    pub amended_by: Vec<String>,
    pub depended_on_by: Vec<String>,
}

/// Build the relationship view for `id`, or [`Error::NotFound`].
pub fn relationships(registry: &Registry, id: &str) -> Result<RelationshipView, Error> {
    let spec = show(registry, id)?;
    let incoming = |pick: fn(&SpecRecord) -> &Vec<String>| -> Vec<String> {
        registry
            .specs
            .iter()
            .filter(|other| pick(other).iter().any(|t| t == id))
            .map(|other| other.id.clone())
            .collect()
    };
    // `supersedes` carries structured items (spec 019); the relationship view
    // is id-only, so project each item to its predecessor id.
    let superseded_by: Vec<String> = registry
        .specs
        .iter()
        .filter(|other| other.supersedes.iter().any(|x| x.spec() == id))
        .map(|other| other.id.clone())
        .collect();
    Ok(RelationshipView {
        id: spec.id.clone(),
        depends_on: spec.depends_on.clone(),
        supersedes: spec.supersedes.iter().map(|x| x.spec().to_string()).collect(),
        amends: spec.amends.clone(),
        superseded_by,
        amended_by: incoming(|s| &s.amends),
        depended_on_by: incoming(|s| &s.depends_on),
    })
}
