//! `spec-spine compile`: write the per-spec registry shards (deterministic;
//! spec 024) under `<derived_dir>/spec-registry/by-spec/`, plus the wall-clock
//! `build-meta.json` sidecar. The single monolithic `registry.json` is no
//! longer emitted, so two PRs that add or edit different specs write disjoint
//! files and never conflict on a global content-hash line.

use std::fs;
use std::path::Path;

use spec_spine_core::shard::{self, BY_SPEC_DIR};
use spec_spine_core::{registry_dir, registry_shard_files};
use spec_spine_types::{BUILD_META_SCHEMA_VERSION, BuildMeta, Error, Severity};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::load_repo_config;

/// Returns the process exit code: `0` if validation passed, `1` if it failed.
pub fn run(repo: &Path) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;
    let outcome = spec_spine_core::compile(&cfg, repo)?;

    let out_dir = registry_dir(&cfg, repo);
    fs::create_dir_all(&out_dir)
        .map_err(|e| Error::Io(format!("create {}: {e}", out_dir.display())))?;

    // Per-spec shards. `sync_dir` prunes a removed spec's shard, so the shard set
    // always equals the current corpus.
    let shard_files = registry_shard_files(&outcome.shards)?;
    let by_spec = out_dir.join(BY_SPEC_DIR);
    shard::sync_dir(&by_spec, &shard_files)?;

    // Drop a pre-024 monolithic registry.json on upgrade (it is no longer the
    // committed form; the shard tree supersedes it).
    let legacy = out_dir.join("registry.json");
    if legacy.exists() {
        fs::remove_file(&legacy)
            .map_err(|e| Error::Io(format!("remove {}: {e}", legacy.display())))?;
    }

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
            by_spec.display(),
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
