//! `spec-spine registry …` — typed, read-only queries over the compiled
//! registry. Reads `registry.json` via the library loader (never ad-hoc parsing,
//! per spec 000 §1).

use std::fs;
use std::path::Path;

use clap::Subcommand;
use spec_spine_core::{
    ListFilter, list, list_ids, load_registry, relationships, show, status_report,
};
use spec_spine_types::{Error, Status};

use crate::load_repo_config;

#[derive(Subcommand)]
pub enum RegistryQuery {
    /// List specs (optionally filtered by status).
    List {
        #[arg(long, value_name = "STATUS")]
        status: Option<String>,
        #[arg(long)]
        json: bool,
        /// Print bare spec ids, one per line (a JSON string array with --json).
        #[arg(long)]
        ids_only: bool,
    },
    /// Show one spec by id.
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Counts of specs by status.
    StatusReport {
        #[arg(long)]
        json: bool,
        /// Omit statuses whose count is zero (the total still covers the corpus).
        #[arg(long)]
        nonzero_only: bool,
    },
    /// Show a spec's relationship neighborhood.
    Relationships {
        id: String,
        #[arg(long)]
        json: bool,
    },
}

/// Returns `0` on success; `NotFound`/parse/schema errors propagate to the
/// caller's exit-code mapping.
pub fn run(repo: &Path, query: &RegistryQuery) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;
    let registry_path = repo
        .join(&cfg.layout.derived_dir)
        .join("spec-registry")
        .join("registry.json");
    let bytes = fs::read(&registry_path).map_err(|e| {
        Error::Io(format!(
            "read {} (run `spec-spine compile` first?): {e}",
            registry_path.display()
        ))
    })?;
    let registry = load_registry(&bytes)?;

    match query {
        RegistryQuery::List {
            status,
            json,
            ids_only,
        } => {
            let filter = ListFilter {
                status: status.as_deref().map(parse_status).transpose()?,
            };
            if *ids_only {
                // Spec 010 §3.1: ids and nothing else — an empty corpus prints
                // nothing (no "(no specs)" placeholder) and still exits 0.
                let ids = list_ids(&registry, &filter);
                if *json {
                    print_json(&ids)?;
                } else {
                    for id in ids {
                        println!("{id}");
                    }
                }
            } else {
                let specs = list(&registry, &filter);
                if *json {
                    print_json(&specs)?;
                } else if specs.is_empty() {
                    println!("(no specs)");
                } else {
                    for s in specs {
                        println!("{}  {:<11}  {}", s.id, status_label(s.status), s.title);
                    }
                }
            }
        }
        RegistryQuery::Show { id, json } => {
            let spec = show(&registry, id)?;
            if *json {
                print_json(spec)?;
            } else {
                println!("id:      {}", spec.id);
                println!("title:   {}", spec.title);
                println!("status:  {}", status_label(spec.status));
                println!("created: {}", spec.created);
                println!("path:    {}", spec.spec_path);
                println!("summary: {}", spec.summary.trim());
            }
        }
        RegistryQuery::StatusReport { json, nonzero_only } => {
            let report = status_report(&registry);
            if *nonzero_only {
                let projected = report.nonzero_only();
                if *json {
                    print_json(&projected)?;
                } else {
                    println!("total:      {}", projected.total);
                    print_count("draft:     ", projected.draft);
                    print_count("approved:  ", projected.approved);
                    print_count("superseded:", projected.superseded);
                    print_count("retired:   ", projected.retired);
                }
            } else if *json {
                print_json(&report)?;
            } else {
                println!("total:      {}", report.total);
                println!("draft:      {}", report.draft);
                println!("approved:   {}", report.approved);
                println!("superseded: {}", report.superseded);
                println!("retired:    {}", report.retired);
            }
        }
        RegistryQuery::Relationships { id, json } => {
            let view = relationships(&registry, id)?;
            if *json {
                print_json(&view)?;
            } else {
                println!("{}", view.id);
                print_ids("depends_on", &view.depends_on);
                print_ids("supersedes", &view.supersedes);
                print_ids("amends", &view.amends);
                print_ids("superseded_by (incoming)", &view.superseded_by);
                print_ids("amended_by (incoming)", &view.amended_by);
                print_ids("depended_on_by (incoming)", &view.depended_on_by);
            }
        }
    }
    Ok(0)
}

fn parse_status(s: &str) -> Result<Status, Error> {
    match s {
        "draft" => Ok(Status::Draft),
        "approved" => Ok(Status::Approved),
        "superseded" => Ok(Status::Superseded),
        "retired" => Ok(Status::Retired),
        other => Err(Error::NotFound(format!(
            "unknown status '{other}' (expected draft|approved|superseded|retired)"
        ))),
    }
}

fn status_label(s: Status) -> &'static str {
    match s {
        Status::Draft => "draft",
        Status::Approved => "approved",
        Status::Superseded => "superseded",
        Status::Retired => "retired",
    }
}

fn print_count(label: &str, count: Option<usize>) {
    if let Some(n) = count {
        println!("{label} {n}");
    }
}

fn print_ids(label: &str, ids: &[String]) {
    if !ids.is_empty() {
        println!("  {label}: {}", ids.join(", "));
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), Error> {
    let s = serde_json::to_string_pretty(value).map_err(|e| Error::Schema(e.to_string()))?;
    println!("{s}");
    Ok(())
}
