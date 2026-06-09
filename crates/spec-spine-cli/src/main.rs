//! `spec-spine` — the multi-call CLI. A thin wrapper over `spec-spine-core`:
//! it parses args, loads config, calls the engine, prints results, and maps the
//! typed `Error` to a stable process exit code. All `process::exit`, stdout, and
//! `git`/clock side effects live here, never in the library.

mod cmd_compile;
mod cmd_couple;
mod cmd_index;
mod cmd_init;
mod cmd_lint;
mod cmd_registry;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use spec_spine_types::{Config, Error};

#[derive(Parser)]
#[command(
    name = "spec-spine",
    version,
    about = "A typed, hash-verifiable authority ledger over a markdown spec corpus."
)]
struct Cli {
    /// Repository root (defaults to the current directory).
    #[arg(long, global = true, value_name = "DIR")]
    repo: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile specs/*/spec.md into a deterministic registry.
    Compile,
    /// Read-only queries over the compiled registry.
    Registry {
        #[command(subcommand)]
        query: cmd_registry::RegistryQuery,
    },
    /// Build the codebase index, or check it for staleness.
    Index {
        #[command(subcommand)]
        action: Option<cmd_index::IndexAction>,
    },
    /// Run the corpus conformance lint.
    Lint {
        /// Fail (exit 1) if any warning-tier diagnostic is present.
        #[arg(long)]
        fail_on_warn: bool,
        /// Fail (exit 1) if any info-tier diagnostic is present.
        #[arg(long)]
        fail_on_info: bool,
    },
    /// The PR-time coupling gate: refuse code that drifts from its owning spec.
    Couple {
        /// Base ref for the diff (merge-base of `base...head`).
        #[arg(long, default_value = "origin/main")]
        base: String,
        /// Head ref for the diff.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// PR body (waiver source); a file path. Falls back to $SPEC_SPINE_PR_BODY.
        #[arg(long)]
        pr_body: Option<PathBuf>,
        /// Override the diff: read newline-delimited changed paths from this file
        /// (whole-file authority; no hunk data).
        #[arg(long)]
        paths_from: Option<PathBuf>,
    },
    /// Scaffold a new adopter: config, standards, a bootstrap spec, agent rules.
    Init {
        /// Overwrite existing files instead of skipping them.
        #[arg(long)]
        force: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let repo = match cli.repo {
        Some(p) => p,
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let result = match &cli.command {
        Command::Compile => cmd_compile::run(&repo),
        Command::Registry { query } => cmd_registry::run(&repo, query),
        Command::Index { action } => cmd_index::run(&repo, action.as_ref()),
        Command::Lint {
            fail_on_warn,
            fail_on_info,
        } => cmd_lint::run(&repo, *fail_on_warn, *fail_on_info),
        Command::Couple {
            base,
            head,
            pr_body,
            paths_from,
        } => cmd_couple::run(
            &repo,
            &cmd_couple::CoupleArgs {
                base: base.clone(),
                head: head.clone(),
                pr_body: pr_body.clone(),
                paths_from: paths_from.clone(),
            },
        ),
        Command::Init { force } => cmd_init::run(&repo, *force),
    };

    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spec-spine: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

/// Load `<repo>/spec-spine.toml`, or the working default if it is absent.
pub(crate) fn load_repo_config(repo: &Path) -> Result<Config, Error> {
    let path = repo.join("spec-spine.toml");
    match std::fs::read_to_string(&path) {
        Ok(src) => spec_spine_types::load_config(&src),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(Error::Io(format!("read {}: {e}", path.display()))),
    }
}
