//! The single, stable error type for spec-spine.
//!
//! Every variant maps to a stable CLI exit code (see [`Error::exit_code`]); the
//! CLI is the only place that translates an `Error` into a process exit. Inside
//! the library we never `panic!` on user input, never `process::exit`, and never
//! `println!` data — those belong only in the CLI crate.
//!
//! Exit-code contract (see `docs/design/00-architecture.md` §6):
//! `0` ok, `1` validation failure / not found, `2` stale,
//! `3` IO / parse / schema / config error. Coupling drift also exits `1`, but is
//! carried as a `CoupleReport` (not an `Error`) so the JSON facade returns the
//! structured report even on drift; the CLI maps the report to exit `1`.

use crate::registry::Violation;

/// The stable error enum returned across the `spec-spine-core` public boundary.
///
/// `#[non_exhaustive]` so new variants are an additive (non-breaking) change.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A malformed or invalid `spec-spine.toml`. → exit 3.
    Config(String),
    /// Compile validation failed (the registry's `validation.passed` is false). → exit 1.
    Validation(Vec<Violation>),
    /// A requested spec id / view / path was not found. → exit 1.
    NotFound(String),
    /// The committed index is out of date relative to current inputs. → exit 2.
    Stale { expected: String, actual: String },
    /// A filesystem / git / read failure. → exit 3.
    Io(String),
    /// A frontmatter / TOML / JSON parse failure. → exit 3.
    Parse(String),
    /// Emitted or loaded JSON failed schema or version checks. → exit 3.
    Schema(String),
}

impl Error {
    /// The stable process exit code for this error.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Validation(_) | Error::NotFound(_) => 1,
            Error::Stale { .. } => 2,
            Error::Config(_) | Error::Io(_) | Error::Parse(_) | Error::Schema(_) => 3,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Config(m) => write!(f, "config error: {m}"),
            Error::Validation(v) => write!(f, "validation failed: {} violation(s)", v.len()),
            Error::NotFound(m) => write!(f, "not found: {m}"),
            Error::Stale { expected, actual } => {
                write!(
                    f,
                    "index is stale: expected content-hash {expected}, got {actual}"
                )
            }
            Error::Io(m) => write!(f, "io error: {m}"),
            Error::Parse(m) => write!(f, "parse error: {m}"),
            Error::Schema(m) => write!(f, "schema error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
