//! The coupling gate (spec 005): join the spec-as-source registry and the
//! code-as-source index, and refuse drift.
//!
//! `couple_with` is a pure function of `(config, registry, index, diff, waiver)`;
//! `couple` is the freshness-guarded form that loads the committed artifacts.
//! **`git` never runs here** — the CLI parses `git diff --no-color -U0
//! base...head` into a typed [`DiffInput`] and passes it in.
//!
//! The behavioral semantics are ported intact from OAP
//! `tools/spec-spine/spec-code-coupling-check/src/lib.rs`
//! (`legitimate_owners` + the FR-005 strict-expansion guard, `is_bypass_against`,
//! `claim_matches`, `parse_waiver`, `span_overlaps_hunk`, `build_unit_claim_index`)
//! and `main.rs` (`parse_hunk_header`, in `cmd_couple`). Structure is fresh; the
//! algorithm is re-derived, not reinvented.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use spec_spine_types::{CodebaseIndex, Config, Error, LineSpan, Registry, Severity, Violation};

use crate::index::{Freshness, check_index_freshness};
use crate::query::{load_index, load_registry};

/// The hardcoded generic bypass floor (spec 005 §3.5) — the **single built-in
/// source** of bypass entries. The adopter's `config.coupling.bypass_prefixes`
/// (default **empty**) is unioned with this — it is **additive and cannot remove
/// a floor entry** (ported from OAP `BYPASS_PREFIXES`, pruned to the generic
/// subset). Match rules: trailing `/` ⇒ directory prefix; leading `**/` ⇒
/// tail-suffix anywhere; else exact file.
pub const DEFAULT_BYPASS_PREFIXES: &[&str] = &[
    ".github/",
    "docs/",
    "README.md",
    "CHANGELOG.md",
    "LICENSE",
    "CODEOWNERS",
    ".gitignore",
    ".gitattributes",
    "standards/spec/constitution.md",
    ".derived/",
    "**/Cargo.lock",
    "**/package-lock.json",
    "**/pnpm-lock.yaml",
];

/// The parsed diff handed to the gate. The CLI builds it from `git diff -U0`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffInput {
    pub files: Vec<DiffFile>,
}

/// One changed file with its new-side hunk spans. **Empty `hunks`** denotes a
/// whole-file change (a deletion, or `--paths-from` mode) — it overlaps every
/// unit span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    #[serde(default)]
    pub hunks: Vec<LineSpan>,
}

/// A PR-body waiver: the trimmed reason after the configured keyword.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Waiver {
    pub reason: String,
}

/// The coupling outcome. Returned `Ok` for any completed analysis (clean, drift,
/// or waived); the CLI maps it to an exit code. A blocking drift is
/// `!violations.is_empty() && waiver.is_none()`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoupleReport {
    pub violations: Vec<Violation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver: Option<String>,
    /// Non-bypassed diff paths that were examined.
    pub checked_paths: usize,
}

impl CoupleReport {
    /// True when there is drift that no waiver excuses (exit 1).
    pub fn has_blocking_drift(&self) -> bool {
        !self.violations.is_empty() && self.waiver.is_none()
    }

    /// `1` for blocking drift, else `0`. Stale / IO are `Err`, not a report.
    pub fn exit_code(&self) -> u8 {
        if self.has_blocking_drift() { 1 } else { 0 }
    }
}

/// Freshness-guarded coupling. Refuses a stale index (exit 2 — recompute first),
/// then loads the committed `registry.json` + `index.json` from `derived_dir`
/// and delegates to [`couple_with`].
pub fn couple(
    cfg: &Config,
    repo_root: &Path,
    diff: &DiffInput,
    waiver: Option<&Waiver>,
) -> Result<CoupleReport, Error> {
    match check_index_freshness(cfg, repo_root)? {
        Freshness::Stale { expected, actual } => return Err(Error::Stale { expected, actual }),
        Freshness::Fresh => {}
    }
    let registry = load_committed_registry(cfg, repo_root)?;
    let index = load_committed_index(cfg, repo_root)?;
    couple_with(cfg, &registry, &index, diff, waiver)
}

/// Pure coupling over already-loaded artifacts (overlays, tests). No IO.
pub fn couple_with(
    cfg: &Config,
    registry: &Registry,
    index: &CodebaseIndex,
    diff: &DiffInput,
    waiver: Option<&Waiver>,
) -> Result<CoupleReport, Error> {
    let diff_paths: BTreeSet<String> = diff.files.iter().map(|f| f.path.clone()).collect();
    let superseders = build_superseders(registry);

    let mut violations: Vec<Violation> = Vec::new();
    let mut checked_paths = 0usize;

    for file in &diff.files {
        let path = &file.path;
        // Effective bypass = hardcoded floor ∪ adopter list (additive) —
        // UNLESS an explicit, resolved unit claim covers the path, which
        // takes precedence over the entire bypass set (spec 009, amending
        // 005 §3.5). The corpus saying "this surface is governed" beats the
        // blanket scaffolding exemption.
        if !explicitly_claimed(path, index)
            && (is_bypass(path, DEFAULT_BYPASS_PREFIXES)
                || is_bypass(path, &cfg.coupling.bypass_prefixes))
        {
            continue;
        }
        checked_paths += 1;

        let owners = owners_for_path(path, &file.hunks, index, &superseders);
        if owners.is_empty() {
            continue; // unclaimed path — not a coupling concern
        }
        if any_owner_in_diff(&owners, &diff_paths) {
            continue; // primary-owner heuristic: any one owner's spec.md cleared it
        }

        let names: Vec<String> = owners.iter().cloned().collect();
        violations.push(Violation {
            code: "C-001".to_string(),
            severity: Severity::Error,
            message: format!(
                "'{path}' changed without an authoring edit to any owning spec ({})",
                names.join(", ")
            ),
            path: Some(path.clone()),
        });
    }

    violations.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(CoupleReport {
        violations,
        waiver: waiver.map(|w| w.reason.clone()),
        checked_paths,
    })
}

/// The effective bypass verdict for one path: hardcoded floor ∪ adopter list
/// (additive), with explicit unit claims taking precedence (spec 009) —
/// exactly as [`couple_with`] applies it. Public so the CLI's
/// dependency-only auto-waiver pre-filter (spec 005 §3.5) examines the same
/// non-bypassed path set the gate itself will check; the claim-awareness is
/// what keeps a claim-overridden floor path from slipping past that
/// pre-filter into a mechanical waiver.
pub fn is_bypassed_path(cfg: &Config, index: &CodebaseIndex, path: &str) -> bool {
    !explicitly_claimed(path, index)
        && (is_bypass(path, DEFAULT_BYPASS_PREFIXES)
            || is_bypass(path, &cfg.coupling.bypass_prefixes))
}

/// Spec 009 §3.1: true iff at least one **resolved, ownership-bearing unit
/// claim** covers `path` — a location file matching exactly, or by directory
/// prefix for a directory-form file unit (004 §3.3). Implicit path-level
/// ownership (manifest metadata, comment headers → `implementingPaths`)
/// deliberately does NOT count (§3.2): an explicit unit in spec frontmatter
/// is an author saying *this exact surface is governed*; a crate floor is a
/// blanket safety net that keeps deferring to bypass.
fn explicitly_claimed(path: &str, index: &CodebaseIndex) -> bool {
    index.traceability.mappings.iter().any(|m| {
        m.resolved_units.iter().filter(|ru| ru.ownership).any(|ru| {
            ru.locations
                .iter()
                .any(|loc| claim_matches(&loc.file, path))
        })
    })
}

/// Parse a waiver from the PR body using the configured keyword. Returns the
/// first line's trimmed reason (ported from OAP `parse_waiver`).
pub fn parse_waiver(cfg: &Config, body: &str) -> Option<Waiver> {
    let keyword = &cfg.coupling.waiver_keyword;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(keyword.as_str()) {
            let reason = rest.trim();
            if !reason.is_empty() {
                return Some(Waiver {
                    reason: reason.to_string(),
                });
            }
        }
    }
    None
}

// ===== owner derivation (the ported algorithm) =====

/// The legitimate owners of a changed `(path, hunks)`. Unions span-aware
/// resolved-unit ownership with whole-file `implementingPaths`, applies
/// supersedes transfer, then amends-awareness under the FR-005 strict guard.
fn owners_for_path(
    path: &str,
    hunks: &[LineSpan],
    index: &CodebaseIndex,
    superseders: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    // 1. Candidate (owner, span) pairs across both linkage sources.
    let mut candidates: Vec<(String, Option<LineSpan>)> = Vec::new();
    for m in &index.traceability.mappings {
        for ru in &m.resolved_units {
            if !ru.ownership {
                continue; // `references` is non-owning
            }
            for loc in &ru.locations {
                if claim_matches(&loc.file, path) {
                    candidates.push((m.spec_id.clone(), loc.span));
                }
            }
        }
        for ip in &m.implementing_paths {
            if claim_matches(&ip.path, path) {
                candidates.push((m.spec_id.clone(), None)); // whole-file
            }
        }
    }

    // 2. Supersedes transfer: a successor inherits its predecessor's authority
    //    (with the same span). Additive — the predecessor is not removed.
    let mut transferred: Vec<(String, Option<LineSpan>)> = Vec::new();
    for (owner, span) in &candidates {
        for succ in transitive_superseders(owner, superseders) {
            transferred.push((succ, *span));
        }
    }
    candidates.extend(transferred);

    // 3. Keep owners whose span overlaps a hunk (empty hunks ⇒ whole-file change
    //    overlaps everything).
    let mut owners: BTreeSet<String> = BTreeSet::new();
    for (spec_id, span) in &candidates {
        let applies = hunks.is_empty() || hunks.iter().any(|h| span_overlaps_hunk(*span, *h));
        if applies {
            owners.insert(spec_id.clone());
        }
    }

    // 4. Amends-awareness — only when the base owner set is non-empty (FR-005
    //    strict-expansion guard) and the path is `specs/<id>/spec.md`.
    if !owners.is_empty() {
        if let Some(amended_id) = spec_id_for_spec_md_path(path) {
            for m in &index.traceability.mappings {
                if m.amends.iter().any(|a| a == amended_id) {
                    owners.insert(m.spec_id.clone());
                }
                if m.spec_id == amended_id {
                    if let Some(record) = &m.amendment_record {
                        owners.insert(record.clone());
                    }
                }
            }
        }
    }

    owners
}

/// True when at least one owner's `specs/<id>/spec.md` is in the diff (the
/// primary-owner heuristic — ported from OAP `OwnerSet::any_owner_in_diff`).
fn any_owner_in_diff(owners: &BTreeSet<String>, diff_paths: &BTreeSet<String>) -> bool {
    owners
        .iter()
        .any(|id| diff_paths.contains(&format!("specs/{id}/spec.md")))
}

/// Direct `predecessor → {superseders}` map from the registry's `supersedes`.
fn build_superseders(registry: &Registry) -> BTreeMap<String, BTreeSet<String>> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for spec in &registry.specs {
        for predecessor in &spec.supersedes {
            map.entry(predecessor.clone())
                .or_default()
                .insert(spec.id.clone());
        }
    }
    map
}

/// Transitive closure of superseders of `id` (handles supersedes chains).
fn transitive_superseders(
    id: &str,
    superseders: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = vec![id.to_string()];
    while let Some(cur) = stack.pop() {
        if let Some(succs) = superseders.get(&cur) {
            for s in succs {
                if out.insert(s.clone()) {
                    stack.push(s.clone());
                }
            }
        }
    }
    out
}

/// Inclusive span overlap. `None` (whole file) overlaps any hunk (ported from
/// OAP `span_overlaps_hunk`, here on inclusive [start,end] both sides).
fn span_overlaps_hunk(span: Option<LineSpan>, hunk: LineSpan) -> bool {
    match span {
        None => true,
        Some(s) => s.start_line <= hunk.end_line && hunk.start_line <= s.end_line,
    }
}

/// Slash-anchored prefix match: a directory `claim` (trailing `/`, or any path
/// treated as a directory) owns every file under it; an exact path matches
/// itself (ported from OAP `claim_matches`).
fn claim_matches(claim: &str, path: &str) -> bool {
    if claim == path {
        return true;
    }
    let claim_dir = if claim.ends_with('/') {
        claim.to_string()
    } else {
        format!("{claim}/")
    };
    path.starts_with(&claim_dir)
}

/// Bypass match against a prefix slice (ported from OAP `is_bypass_against`).
fn is_bypass<S: AsRef<str>>(path: &str, prefixes: &[S]) -> bool {
    prefixes.iter().any(|prefix| {
        let prefix = prefix.as_ref();
        if let Some(tail) = prefix.strip_prefix("**/") {
            path == tail || path.ends_with(&format!("/{tail}"))
        } else if prefix.ends_with('/') {
            path.starts_with(prefix)
        } else {
            path == prefix || path == format!("{prefix}/")
        }
    })
}

/// Parse `specs/<id>/spec.md` into `<id>` (ported from OAP
/// `spec_id_for_spec_md_path`). `None` for any other path.
fn spec_id_for_spec_md_path(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("specs/")?;
    let (id, tail) = rest.split_once('/')?;
    if tail == "spec.md" { Some(id) } else { None }
}

// ===== committed-artifact loaders (the IO half of `couple`) =====

fn load_committed_registry(cfg: &Config, repo_root: &Path) -> Result<Registry, Error> {
    let path = repo_root
        .join(&cfg.layout.derived_dir)
        .join("spec-registry")
        .join("registry.json");
    let bytes = fs::read(&path).map_err(|e| {
        Error::Io(format!(
            "read {} (run `spec-spine compile` first?): {e}",
            path.display()
        ))
    })?;
    load_registry(&bytes)
}

fn load_committed_index(cfg: &Config, repo_root: &Path) -> Result<CodebaseIndex, Error> {
    let path = repo_root
        .join(&cfg.layout.derived_dir)
        .join("codebase-index")
        .join("index.json");
    let bytes = fs::read(&path).map_err(|e| {
        Error::Io(format!(
            "read {} (run `spec-spine index` first?): {e}",
            path.display()
        ))
    })?;
    load_index(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_match_rules() {
        assert!(is_bypass(
            ".github/workflows/ci.yml",
            DEFAULT_BYPASS_PREFIXES
        ));
        assert!(is_bypass("docs/x.md", DEFAULT_BYPASS_PREFIXES));
        assert!(is_bypass("README.md", DEFAULT_BYPASS_PREFIXES));
        assert!(is_bypass("crates/core/Cargo.lock", DEFAULT_BYPASS_PREFIXES)); // **/ tail
        assert!(is_bypass(".derived/x.json", DEFAULT_BYPASS_PREFIXES));
        assert!(!is_bypass(
            "crates/core/src/lib.rs",
            DEFAULT_BYPASS_PREFIXES
        ));
    }

    #[test]
    fn claim_match_exact_and_dir_prefix() {
        assert!(claim_matches("crates/core", "crates/core/src/lib.rs"));
        assert!(claim_matches("crates/core/", "crates/core/src/lib.rs"));
        assert!(claim_matches("Makefile", "Makefile"));
        assert!(!claim_matches("crates/cor", "crates/core/src/lib.rs"));
    }

    #[test]
    fn span_overlap_inclusive() {
        assert!(span_overlaps_hunk(None, LineSpan::new(1, 1)));
        assert!(span_overlaps_hunk(
            Some(LineSpan::new(10, 20)),
            LineSpan::new(20, 25)
        ));
        assert!(!span_overlaps_hunk(
            Some(LineSpan::new(10, 20)),
            LineSpan::new(21, 25)
        ));
    }

    #[test]
    fn spec_md_path_parse() {
        assert_eq!(
            spec_id_for_spec_md_path("specs/005-x/spec.md"),
            Some("005-x")
        );
        assert_eq!(spec_id_for_spec_md_path("specs/005-x/plan.md"), None);
        assert_eq!(spec_id_for_spec_md_path("crates/core/src/lib.rs"), None);
    }
}
