//! The authority-unit grammar.
//!
//! A spec declares the units it owns via a `unit:` on a typed edge. The grammar
//! resolves six granularities: [`Unit::File`], [`Unit::Section`],
//! [`Unit::Symbol`], [`Unit::Directory`], [`Unit::Crate`], and [`Unit::Module`]
//! (ported from OAP `spec-types::LogicalUnit`). All six are implemented:
//! `file`/`section`/`symbol` shipped first, and `directory`/`crate`/`module`
//! landed in spec 017 (originally reserved in `docs/design/00-architecture.md`
//! §2.2 Q5). They were a MINOR bump because the schema is permissive on the unit
//! payload (no schema-file edit), and a bare string remains shorthand for a file
//! unit (a trailing-slash path denotes a directory subtree).

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/// An authority unit: the granularity at which a spec claims ownership.
///
/// Serializes internally-tagged on `kind` (e.g. `{ "kind": "file", "path": ... }`).
/// Deserializes from that tagged map, a bare string (= a file unit), **or** a
/// `{ unit: <unit> }` wrapper (spec 015 sugar), so authors can write
/// `establishes: ["src/lib.rs"]` or `establishes: [{ unit: "src/lib.rs" }]`
/// interchangeably; all three normalize to the same unit.
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
    /// tree-sitter (Rust `.rs` and TypeScript `.ts`/`.tsx`).
    Symbol { id: String },
    /// A directory subtree, named explicitly (`{ kind: directory, path }`). The
    /// subtree-prefix resolution is identical to a trailing-slash file unit; the
    /// distinct kind preserves the author's intent across the round-trip (spec
    /// 017). Resolves to the directory path; the gate prefix-matches it.
    Directory { path: String },
    /// A compilation unit by its manifest name (Cargo `[package].name` or npm
    /// `package.json:name`), resolved against the discovered package inventory to
    /// the package directory subtree (spec 017).
    Crate { id: String },
    /// A module by its `::`-qualified path (e.g. `my_crate::serialization`),
    /// resolved by the indexer's Rust module index: file-modules (whole file)
    /// and top-level inline `mod` blocks (line-span) (spec 017).
    Module { id: String },
}

impl Unit {
    /// A file unit from a path (the bare-string shorthand target).
    pub fn file(path: impl Into<String>) -> Self {
        Unit::File { path: path.into() }
    }

    /// True if this unit resolves to a directory subtree: a file unit whose path
    /// ends `/`, or an explicit [`Unit::Directory`] (spec 017).
    pub fn is_directory_subtree(&self) -> bool {
        match self {
            Unit::File { path } => path.ends_with('/'),
            Unit::Directory { .. } => true,
            _ => false,
        }
    }
}

impl<'de> Deserialize<'de> for Unit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept a bare string (-> file unit), the tagged map form, or the
        // `{ unit: <unit> }` wrapper (spec 015).
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Bare(String),
            Tagged(Tagged),
            // A predecessor dialect authors every `establishes` item as a
            // single-key `unit:` map. The wrapper carries no information beyond
            // the unit it wraps, so it normalizes away to that inner unit -- a
            // third 1:1 representation alongside the bare-string and tagged
            // forms, resolved by recursing through this same impl (so the inner
            // unit may itself be bare or tagged, and inherits its validation).
            Wrapped { unit: Box<Unit> },
        }
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
        enum Tagged {
            File { path: String },
            Section { file: String, anchor: String },
            Symbol { id: String },
            Directory { path: String },
            Crate { id: String },
            Module { id: String },
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
            Repr::Tagged(Tagged::Directory { path }) => Ok(Unit::Directory { path }),
            Repr::Tagged(Tagged::Crate { id }) => Ok(Unit::Crate { id }),
            Repr::Tagged(Tagged::Module { id }) => Ok(Unit::Module { id }),
            Repr::Wrapped { unit } => Ok(*unit),
        }
    }
}
