//! `spec-spine index`: write `index.json`; `spec-spine index check`: staleness;
//! `spec-spine index render` / `index orphans`: read-side projections of the
//! committed artifact (spec 011; never recompute, never check freshness).

use std::fs;
use std::path::Path;

use clap::Subcommand;
use spec_spine_core::{
    Freshness, check_index_freshness, check_slice_freshness, index, load_index, orphans,
    render_markdown,
};
use spec_spine_types::{CodebaseIndex, Config, Error};

use crate::load_repo_config;

#[derive(Subcommand)]
pub enum IndexAction {
    /// Check the committed index against current inputs (the staleness gate).
    Check {
        /// Gate one named [index.slices] slice instead of the global hash.
        #[arg(long, value_name = "NAME")]
        slice: Option<String>,
    },
    /// Render the committed index as markdown (a projection; never recomputes).
    Render,
    /// List orphaned specs from the committed index.
    Orphans {
        #[arg(long)]
        json: bool,
    },
}

/// Read and parse the committed `index.json` (spec 011 §3.1: the projections
/// read the artifact, never the working tree).
fn load_committed_index(repo: &Path, cfg: &Config) -> Result<CodebaseIndex, Error> {
    let path = repo
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

/// `index` (no action) writes the index; `index check` verifies freshness.
pub fn run(repo: &Path, action: Option<&IndexAction>) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;

    match action {
        Some(IndexAction::Render) => {
            let idx = load_committed_index(repo, &cfg)?;
            print!("{}", render_markdown(&cfg, &idx));
            Ok(0)
        }
        Some(IndexAction::Orphans { json }) => {
            let idx = load_committed_index(repo, &cfg)?;
            let ids = orphans(&idx);
            if *json {
                let s =
                    serde_json::to_string_pretty(&ids).map_err(|e| Error::Schema(e.to_string()))?;
                println!("{s}");
            } else {
                for id in ids {
                    println!("{id}");
                }
            }
            Ok(0)
        }
        Some(IndexAction::Check { slice }) => {
            let (freshness, subject) = match slice {
                Some(name) => (
                    check_slice_freshness(&cfg, repo, name)?,
                    format!("slice '{name}'"),
                ),
                None => (check_index_freshness(&cfg, repo)?, "index".to_string()),
            };
            match freshness {
                Freshness::Fresh => {
                    println!("{subject} is fresh");
                    Ok(0)
                }
                Freshness::Stale { expected, actual } => {
                    eprintln!("{subject} is STALE (run `spec-spine index` to refresh)");
                    eprintln!("  expected content-hash: {expected}");
                    eprintln!("  actual content-hash:   {actual}");
                    Ok(2)
                }
            }
        }
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
