//! `spec-spine compile`: write `registry.json` (deterministic) and
//! `build-meta.json` (wall-clock sidecar) under `<derived_dir>/spec-registry/`.

use std::fs;
use std::path::Path;

use spec_spine_types::{BUILD_META_SCHEMA_VERSION, BuildMeta, Error, Severity};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::load_repo_config;

/// Returns the process exit code: `0` if validation passed, `1` if it failed.
pub fn run(repo: &Path) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;
    let outcome = spec_spine_core::compile(&cfg, repo)?;

    let out_dir = repo.join(&cfg.layout.derived_dir).join("spec-registry");
    fs::create_dir_all(&out_dir)
        .map_err(|e| Error::Io(format!("create {}: {e}", out_dir.display())))?;

    let registry_path = out_dir.join("registry.json");
    fs::write(&registry_path, &outcome.json)
        .map_err(|e| Error::Io(format!("write {}: {e}", registry_path.display())))?;

    // build-meta.json carries the wall clock; the CLI owns it. Excluded from
    // determinism/golden checks and from version control (see .gitignore).
    let meta = BuildMeta {
        schema_version: BUILD_META_SCHEMA_VERSION.to_string(),
        built_at: now_rfc3339(),
        compiler_id: cfg.branding.compiler_id.clone(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let meta_json =
        serde_json::to_string_pretty(&meta).map_err(|e| Error::Schema(e.to_string()))? + "\n";
    let meta_path = out_dir.join("build-meta.json");
    fs::write(&meta_path, meta_json)
        .map_err(|e| Error::Io(format!("write {}: {e}", meta_path.display())))?;

    let errors = outcome
        .registry
        .validation
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .count();
    let warnings = outcome
        .registry
        .validation
        .violations
        .iter()
        .filter(|v| v.severity == Severity::Warning)
        .count();

    if outcome.validation_passed {
        println!(
            "compiled {} spec(s) -> {} ({} warning(s))",
            outcome.registry.specs.len(),
            registry_path.display(),
            warnings
        );
        Ok(0)
    } else {
        // Validation failures go to stderr so they surface in CI logs.
        for v in &outcome.registry.validation.violations {
            if v.severity == Severity::Error {
                let at = v.path.as_deref().unwrap_or("-");
                eprintln!("  {} [{}] {}", v.code, at, v.message);
            }
        }
        eprintln!(
            "validation FAILED: {errors} error(s), {warnings} warning(s) across {} spec(s)",
            outcome.registry.specs.len()
        );
        Ok(1)
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}
