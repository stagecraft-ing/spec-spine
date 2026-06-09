//! # spec-spine-core
//!
//! The spec-spine engine. Phase 2 shipped **compile** + **query**; Phase 3 adds
//! **index** (code-as-source view, staleness, authorities) and **lint**.
//! `couple` / `init` land in later phases.
//!
//! Every artifact-producing function is a pure function of `(config, file
//! contents)` — no ambient clock or environment reads. The public API returns
//! owned, `serde`-serializable DTOs (from [`spec_spine_types`]); the
//! JSON-in/JSON-out facade ([`compile_json`], [`query_json`], [`index_json`],
//! [`lint_json`], …) is the seam future FFI bindings wrap.

mod canonical_json;
pub mod compile;
mod hash;
pub mod index;
pub mod lint;
pub mod manifest;
mod markdown;
pub mod pathutil;
pub mod query;
pub mod sections;
pub mod symbols;

use serde::Deserialize;
use spec_spine_types::{Config, Error, Status, load_config};

// Re-export the type substrate so callers depend on one crate.
pub use spec_spine_types as types;
pub use spec_spine_types::{
    CodebaseIndex, Frontmatter, REGISTRY_SCHEMA_VERSION, Registry, SpecRecord, Unit, Violation,
};

pub use compile::{CompileOutcome, MAX_UNDECLARED_EXTRA_FRONTMATTER, compile};
pub use index::{Freshness, IndexOutcome, authorities, check_index_freshness, index};
pub use lint::{LintReport, lint};
pub use query::{
    ListFilter, RelationshipView, StatusReport, list, load_index, load_registry, relationships,
    show, status_report,
};

// ===== JSON-in / JSON-out facade (the FFI seam) =====

/// Compile the corpus under `repo_root`, returning the registry as JSON.
///
/// `config_json` is a JSON object matching [`Config`] (`"{}"` ⇒ defaults). The
/// returned string is the canonical `registry.json`; the caller inspects its
/// embedded `validation.passed`.
pub fn compile_json(config_json: &str, repo_root: &str) -> Result<String, Error> {
    let config = config_from_json(config_json)?;
    let outcome = compile(&config, std::path::Path::new(repo_root))?;
    Ok(outcome.json)
}

/// Run a read-only query described by `request_json`.
///
/// Request shape: `{ "registry": "<registry.json text>", "op": "list" |
/// "show" | "status-report" | "relationships", "id"?: string, "status"?: string }`.
pub fn query_json(request_json: &str) -> Result<String, Error> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Request {
        registry: String,
        op: Op,
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        status: Option<Status>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Op {
        List,
        Show,
        StatusReport,
        Relationships,
    }

    let request: Request = serde_json::from_str(request_json)
        .map_err(|e| Error::Parse(format!("invalid query request: {e}")))?;
    let registry = load_registry(request.registry.as_bytes())?;

    let json = match request.op {
        Op::List => {
            let filter = ListFilter {
                status: request.status,
            };
            to_json(&list(&registry, &filter))?
        }
        Op::Show => {
            let id = request
                .id
                .ok_or_else(|| Error::NotFound("missing 'id' for show".into()))?;
            to_json(show(&registry, &id)?)?
        }
        Op::StatusReport => to_json(&status_report(&registry))?,
        Op::Relationships => {
            let id = request
                .id
                .ok_or_else(|| Error::NotFound("missing 'id' for relationships".into()))?;
            to_json(&relationships(&registry, &id)?)?
        }
    };
    Ok(json)
}

/// Index the corpus under `repo_root`, returning `index.json`.
pub fn index_json(config_json: &str, repo_root: &str) -> Result<String, Error> {
    let config = config_from_json(config_json)?;
    Ok(index(&config, std::path::Path::new(repo_root))?.json)
}

/// Lint the corpus, returning the `L-` violations as a JSON array.
pub fn lint_json(config_json: &str, repo_root: &str) -> Result<String, Error> {
    let config = config_from_json(config_json)?;
    let report = lint(&config, std::path::Path::new(repo_root))?;
    to_json(&report.violations)
}

/// Check index freshness, returning `{ "fresh": bool, "expected"?, "actual"? }`.
pub fn check_freshness_json(config_json: &str, repo_root: &str) -> Result<String, Error> {
    let config = config_from_json(config_json)?;
    let value = match check_index_freshness(&config, std::path::Path::new(repo_root))? {
        Freshness::Fresh => serde_json::json!({ "fresh": true }),
        Freshness::Stale { expected, actual } => {
            serde_json::json!({ "fresh": false, "expected": expected, "actual": actual })
        }
    };
    Ok(value.to_string())
}

/// Parse a `spec-spine.toml` and return the normalized [`Config`] as JSON.
pub fn load_config_json(toml_src: &str) -> Result<String, Error> {
    let config = load_config(toml_src)?;
    to_json(&config)
}

// --- facade helpers ---

fn config_from_json(config_json: &str) -> Result<Config, Error> {
    serde_json::from_str(config_json)
        .map_err(|e| Error::Config(format!("invalid config JSON: {e}")))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|e| Error::Schema(e.to_string()))
}
