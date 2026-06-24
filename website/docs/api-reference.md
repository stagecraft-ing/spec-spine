---
id: api-reference
title: API Reference
sidebar_position: 6
---

# API Reference

The stable surface of spec-spine is the `spec-spine-core` library API. The CLI is merely a thin wrapper around this API. If you are building bindings or overlays, you interact with the library API, not the CLI.

Every artifact-producing function is a **pure function of `(Config, file contents)`**: there is no ambient clock, no environment reads, and no Git invocation within the core.

## Crate Layout

- **`spec-spine-types`**: Data Transfer Objects (DTOs), frontmatter grammar, `Config`, schema-version constants, embedded JSON Schemas, and the `Error` enum. Depend on this if you only need the data shapes.
- **`spec-spine-core`**: The engine (`compile`, `index`, `lint`, `couple`, `query`, `scaffold_init`) and the JSON facade. Depend on this to embed the engine or build over the artifacts.
- **`spec-spine-cli`**: The thin multi-call binary.

## The Five Capabilities

The core capabilities are exposed as pure Rust functions:

```rust
use std::path::Path;
use spec_spine_core::{compile, index, lint, couple, check_index_freshness};

pub fn compile(cfg: &Config, repo_root: &Path) -> Result<CompileOutcome, Error>;
pub fn index  (cfg: &Config, repo_root: &Path) -> Result<IndexOutcome,   Error>;
pub fn lint   (cfg: &Config, repo_root: &Path) -> Result<LintReport,     Error>;

// Coupling takes a parsed diff and optional waiver
pub fn couple (cfg: &Config, repo_root: &Path,
               diff: &DiffInput, waiver: Option<&Waiver>) -> Result<CoupleReport, Error>;

// Staleness check
pub fn check_index_freshness(cfg: &Config, repo_root: &Path) -> Result<Freshness, Error>;
```

### Coupling Input

The coupling gate does not shell out to Git. The caller parses the diff and passes a `DiffInput`:

```rust
pub struct DiffInput { pub files: Vec<DiffFile> }
pub struct DiffFile  { pub path: String, pub hunks: Vec<LineSpan> }
pub struct Waiver    { pub reason: String }
```

Coupling returns a `CoupleReport` even if drift is detected. Drift is data, not an `Error` variant. The CLI maps `report.has_blocking_drift()` to exit code 1.

## Config Load and Init

```rust
// Parse and validate spec-spine.toml
pub fn load_config(toml_src: &str) -> Result<Config, Error>;

// Returns files-as-data for the CLI to write
pub fn scaffold_init(cfg: &Config) -> Result<Scaffold, Error>;
```

## The Overlay Seam

Overlays use typed, read-only loaders to consume the generic artifacts:

```rust
pub fn load_registry(bytes: &[u8]) -> Result<Registry,      Error>;
pub fn load_index   (bytes: &[u8]) -> Result<CodebaseIndex, Error>;
```

These loaders reject unknown MAJOR schema versions.

## Typed Query Layer

Queries over a loaded `Registry` or `CodebaseIndex`:

```rust
pub fn list        (registry: &Registry, filter: &ListFilter) -> Vec<&SpecRecord>;
pub fn show        (registry: &Registry, id: &str)            -> Result<&SpecRecord, Error>;
pub fn status_report(registry: &Registry)                     -> StatusReport;
pub fn relationships(registry: &Registry, id: &str)           -> Result<RelationshipView, Error>;

// Resolve authority over the index
pub fn authorities(index: &CodebaseIndex, unit: &Unit) -> Vec<String>;
```

## The JSON Facade

Every top-level operation has a `&str -> Result<String, Error>` facade function. This is the seam that FFI bindings (napi, pyo3, cgo) wrap.

```rust
pub fn compile_json(config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn couple_json (request_json: &str)                 -> Result<String, Error>;
// ...
```

All emitted JSON is pretty-printed with sorted keys, LF line endings, and a trailing newline to ensure diffability.
