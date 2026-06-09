//! The conformance lint (spec 003): corpus convention checks (`L-` codes),
//! disjoint from compile's structural `V-` codes. Severity gating
//! (error always / warning under `--fail-on-warn` / info under `--fail-on-info`)
//! is applied by the CLI; this layer just produces the diagnostics.

use std::collections::BTreeSet;
use std::path::Path;

use spec_spine_types::{Config, Error, Severity, SpecRecord, Violation};

use crate::compile::compile;

/// The result of a lint run.
pub struct LintReport {
    pub violations: Vec<Violation>,
}

impl LintReport {
    /// Count of violations at a given severity.
    pub fn count(&self, severity: Severity) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == severity)
            .count()
    }
}

/// Lint the corpus under `repo_root`: compile it, then run conformance checks.
pub fn lint(cfg: &Config, repo_root: &Path) -> Result<LintReport, Error> {
    let registry = compile(cfg, repo_root)?.registry;
    let ids: BTreeSet<&str> = registry.specs.iter().map(|s| s.id.as_str()).collect();
    let domains_enabled = !cfg.domains.allowed.is_empty();
    let kind_enabled = !cfg.kind.allowed.is_empty();

    let mut violations = Vec::new();
    for spec in &registry.specs {
        let at = || Some(spec.spec_path.clone());

        // L-001: ordinary spec claims no territory.
        let retroactive = spec.origin.as_ref().is_some_and(|o| o.retroactive);
        if !retroactive && !has_ownership_edge(spec) {
            violations.push(warn(
                "L-001",
                format!(
                    "spec '{}' declares no ownership edge (claims no territory)",
                    spec.id
                ),
                at(),
            ));
        }
        // L-002 / L-003: unclassified under an enabled taxonomy.
        if domains_enabled && spec.domain.is_none() {
            violations.push(warn(
                "L-002",
                format!("spec '{}' has no domain", spec.id),
                at(),
            ));
        }
        if kind_enabled && spec.kind.is_none() {
            violations.push(warn(
                "L-003",
                format!("spec '{}' has no kind", spec.id),
                at(),
            ));
        }
        // L-004: dangling edge target.
        for target in edge_targets(spec) {
            if !ids.contains(target.as_str()) {
                violations.push(warn(
                    "L-004",
                    format!("spec '{}' references unknown spec '{target}'", spec.id),
                    at(),
                ));
            }
        }
        // L-005: stub (no body sections).
        if spec.section_headings.is_empty() {
            violations.push(info(
                "L-005",
                format!("spec '{}' has no body sections", spec.id),
                at(),
            ));
        }
    }

    Ok(LintReport { violations })
}

fn has_ownership_edge(spec: &SpecRecord) -> bool {
    !spec.establishes.is_empty()
        || !spec.extends.is_empty()
        || !spec.refines.is_empty()
        || !spec.supersedes.is_empty()
        || !spec.amends.is_empty()
        || !spec.co_authority.is_empty()
        || !spec.constrains.is_empty()
}

/// Every spec id this spec names across its relationship edges.
fn edge_targets(spec: &SpecRecord) -> Vec<String> {
    let mut targets = Vec::new();
    targets.extend(spec.supersedes.iter().cloned());
    targets.extend(spec.amends.iter().cloned());
    targets.extend(spec.extends.iter().map(|e| e.spec.clone()));
    targets.extend(spec.refines.iter().flat_map(|r| r.refines_specs.clone()));
    targets.extend(spec.co_authority.iter().flat_map(|c| c.with_specs.clone()));
    targets.extend(spec.constrains.iter().flat_map(|c| c.target_specs.clone()));
    targets
}

fn warn(code: &str, message: String, path: Option<String>) -> Violation {
    Violation {
        code: code.to_string(),
        severity: Severity::Warning,
        message,
        path,
    }
}

fn info(code: &str, message: String, path: Option<String>) -> Violation {
    Violation {
        code: code.to_string(),
        severity: Severity::Info,
        message,
        path,
    }
}
