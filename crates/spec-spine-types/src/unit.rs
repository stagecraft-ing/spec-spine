//! The authority-unit grammar.
//!
//! A spec declares the units it owns via a `unit:` on a typed edge. v1 resolves
//! three granularities: [`Unit::File`], [`Unit::Section`], [`Unit::Symbol`]
//! (ported from OAP `spec-types::LogicalUnit`; `crate`/`module`/`directory` are
//! reserved for an additive future minor — see `docs/design/00-architecture.md`
//! §2.2). A bare string is shorthand for a file unit; a trailing-slash path
//! denotes the directory subtree.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/// An authority unit: the granularity at which a spec claims ownership.
///
/// Serializes internally-tagged on `kind` (e.g. `{ "kind": "file", "path": ... }`).
/// Deserializes from either that tagged map **or** a bare string (= a file unit),
/// so authors can write `establishes: ["src/lib.rs"]` as shorthand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Unit {
    /// A file path (bare string shorthand resolves here). A trailing `/` denotes
    /// the directory subtree rooted at `path`.
    File { path: String },
    /// A named section within a file: a Makefile target, a Markdown heading slug,
    /// a `region:` marker, or a CI `jobs.<name>`.
    Section { file: String, anchor: String },
    /// A symbol (function / type / export), resolved by the indexer via
    /// tree-sitter (Rust + TypeScript in v1).
    Symbol { id: String },
}

impl Unit {
    /// A file unit from a path (the bare-string shorthand target).
    pub fn file(path: impl Into<String>) -> Self {
        Unit::File { path: path.into() }
    }

    /// True if this unit is a directory subtree (a file unit whose path ends `/`).
    pub fn is_directory_subtree(&self) -> bool {
        matches!(self, Unit::File { path } if path.ends_with('/'))
    }
}

impl<'de> Deserialize<'de> for Unit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept either a bare string (-> file unit) or the tagged map form.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Bare(String),
            Tagged(Tagged),
        }
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
        enum Tagged {
            File { path: String },
            Section { file: String, anchor: String },
            Symbol { id: String },
        }

        match Repr::deserialize(deserializer)? {
            Repr::Bare(path) => {
                if path.trim().is_empty() {
                    return Err(de::Error::custom("unit path must not be empty"));
                }
                Ok(Unit::File { path })
            }
            Repr::Tagged(Tagged::File { path }) => Ok(Unit::File { path }),
            Repr::Tagged(Tagged::Section { file, anchor }) => Ok(Unit::Section { file, anchor }),
            Repr::Tagged(Tagged::Symbol { id }) => Ok(Unit::Symbol { id }),
        }
    }
}
