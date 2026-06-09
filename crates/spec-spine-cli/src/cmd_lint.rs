//! `spec-spine lint` — corpus conformance lint with tiered fail gating.

use std::path::Path;

use spec_spine_core::lint;
use spec_spine_types::{Error, Severity};

use crate::load_repo_config;

/// Returns the exit code: `1` if any error-tier diagnostic (always), or any
/// warning/info under the matching `--fail-on-*` flag; otherwise `0`.
pub fn run(repo: &Path, fail_on_warn: bool, fail_on_info: bool) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;
    let report = lint(&cfg, repo)?;

    for v in &report.violations {
        let tier = match v.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        let at = v.path.as_deref().unwrap_or("-");
        println!("  {} [{tier}] [{}] {}", v.code, at, v.message);
    }

    let errors = report.count(Severity::Error);
    let warnings = report.count(Severity::Warning);
    let infos = report.count(Severity::Info);
    println!("lint: {errors} error(s), {warnings} warning(s), {infos} info");

    let fail = errors > 0 || (fail_on_warn && warnings > 0) || (fail_on_info && infos > 0);
    Ok(if fail { 1 } else { 0 })
}
