//! `spec-spine index` — write `index.json`; `spec-spine index check` — staleness.

use std::fs;
use std::path::Path;

use clap::Subcommand;
use spec_spine_core::{Freshness, check_index_freshness, index};
use spec_spine_types::Error;

use crate::load_repo_config;

#[derive(Subcommand)]
pub enum IndexAction {
    /// Check the committed index against current inputs (the staleness gate).
    Check,
}

/// `index` (no action) writes the index; `index check` verifies freshness.
pub fn run(repo: &Path, action: Option<&IndexAction>) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;

    match action {
        Some(IndexAction::Check) => match check_index_freshness(&cfg, repo)? {
            Freshness::Fresh => {
                println!("index is fresh");
                Ok(0)
            }
            Freshness::Stale { expected, actual } => {
                eprintln!("index is STALE (run `spec-spine index` to refresh)");
                eprintln!("  expected content-hash: {expected}");
                eprintln!("  actual content-hash:   {actual}");
                Ok(2)
            }
        },
        None => {
            let outcome = index(&cfg, repo)?;
            let out_dir = repo.join(&cfg.layout.derived_dir).join("codebase-index");
            fs::create_dir_all(&out_dir)
                .map_err(|e| Error::Io(format!("create {}: {e}", out_dir.display())))?;
            let path = out_dir.join("index.json");
            fs::write(&path, &outcome.json)
                .map_err(|e| Error::Io(format!("write {}: {e}", path.display())))?;

            let idx = &outcome.index;
            for diag in &idx.diagnostics.errors {
                let at = diag.path.as_deref().unwrap_or("-");
                eprintln!("  {} [{}] {}", diag.code, at, diag.message);
            }
            println!(
                "indexed {} package(s), {} mapping(s) -> {} ({} error diagnostic(s))",
                idx.packages.len(),
                idx.traceability.mappings.len(),
                path.display(),
                idx.diagnostics.errors.len()
            );
            Ok(0)
        }
    }
}
