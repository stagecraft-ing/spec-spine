# spec-spine API reference

> Stable surface: `spec-spine-core` (the engine) over `spec-spine-types` (the
> data substrate). **The library API, not the CLI, is the contract bindings
> wrap.** This document describes the public Rust API and the JSON-in/JSON-out
> facade. For the CLI surface see [adoption-guide.md](adoption-guide.md); for the
> design rationale see [design/00-architecture.md](design/00-architecture.md).

spec-spine turns a markdown spec corpus into a typed, hash-verifiable authority
ledger. Two views are emitted deterministically and joined at PR time:

- **`registry.json`**: the *spec-as-source* view (compiler output).
- **`index.json`**: the *code-as-source* view (indexer output), with a
  content-hash staleness mechanism.

Every artifact-producing function is a **pure function of `(Config, file
contents)`**: no ambient clock, no environment reads, and **no `git`** (the CLI
parses the diff and passes a typed `DiffInput` in). Same inputs ⇒ byte-identical
output. The one wall-clock value (`build-meta.json.builtAt`) is written by the
CLI and excluded from determinism/golden checks.

---

## 1. Crates

| Crate | Role | Depend on it when… |
|---|---|---|
| `spec-spine-types` | DTOs, frontmatter grammar, `Config`, schema-version constants, embedded JSON Schemas, the `Error` enum | you only need the data shapes (e.g. an overlay reading `registry.json`) |
| `spec-spine-core` | the engine: `compile` / `index` / `lint` / `couple` / query + `scaffold_init` + the JSON facade | you are embedding the engine or building over the artifacts |
| `spec-spine-cli` | the thin `spec-spine` multi-call binary | you want the command-line tool (`cargo install spec-spine-cli`) |

`spec-spine-core` re-exports the whole type substrate, so a Rust caller can
depend on `spec-spine-core` alone and reach everything via
`spec_spine_core::types::*` (or the flattened re-exports `Registry`,
`CodebaseIndex`, `Config`, `Error`, …).

```toml
[dependencies]
spec-spine-core = "0.1"
```

---

## 2. The five capabilities + freshness

Each is a pure function of `(Config, on-disk inputs under repo_root)`:

```rust
use std::path::Path;
use spec_spine_core::{compile, index, lint, couple, check_index_freshness};

pub fn compile(cfg: &Config, repo_root: &Path) -> Result<CompileOutcome, Error>;
pub fn index  (cfg: &Config, repo_root: &Path) -> Result<IndexOutcome,   Error>;
pub fn lint   (cfg: &Config, repo_root: &Path) -> Result<LintReport,     Error>;

// Coupling takes an already-parsed diff + optional waiver; it loads the
// committed registry + index from `derived_dir` itself.
pub fn couple(cfg: &Config, repo_root: &Path,
              diff: &DiffInput, waiver: Option<&Waiver>) -> Result<CoupleReport, Error>;

// Lower-level form for callers that already hold the artifacts (overlays, tests):
pub fn couple_with(cfg: &Config, registry: &Registry, index: &CodebaseIndex,
                   diff: &DiffInput, waiver: Option<&Waiver>) -> Result<CoupleReport, Error>;

// Cheap staleness check: does the committed index.json's contentHash match
// the current inputs?
pub fn check_index_freshness(cfg: &Config, repo_root: &Path) -> Result<Freshness, Error>;
```

Returned outcomes carry both the typed struct and the canonical bytes the CLI
writes, so overlays and tests need not re-parse:

```rust
pub struct CompileOutcome { pub registry: Registry,       pub json: String, pub validation_passed: bool }
pub struct IndexOutcome   { pub index:    CodebaseIndex,   pub json: String, pub content_hash: String }
pub enum   Freshness      { Fresh, Stale { expected: String, actual: String } }
```

### Coupling input

The gate never shells out; the caller passes a parsed diff:

```rust
pub struct DiffInput { pub files: Vec<DiffFile> }
pub struct DiffFile  { pub path: String, pub hunks: Vec<LineSpan> }  // empty hunks ⇒ whole-file change
pub struct Waiver    { pub reason: String }

// Build a Waiver from a PR body using the configured keyword:
pub fn parse_waiver(cfg: &Config, pr_body: &str) -> Option<Waiver>;
```

`couple` returns a `CoupleReport` **even on drift**: drift is data, not an
`Error`, so the JSON facade can return the structured report. Map it to an exit
code with `report.has_blocking_drift()` (the CLI does exactly this → exit 1).
`DEFAULT_BYPASS_PREFIXES` is exported so callers can see the always-applied
bypass floor that `coupling.bypass_prefixes` adds to.

---

## 3. Config load + init scaffolding

```rust
use spec_spine_types::{Config, load_config};

// Parse and validate a spec-spine.toml. Clean Error::Config on malformed input,
// never a panic. An absent file ⇒ use Config::default() (a working single-Cargo-
// workspace default with specs/ at the root).
pub fn load_config(toml_src: &str) -> Result<Config, Error>;

// init returns files-as-data; the CLI writes them. Keeps core IO-light & testable.
pub fn scaffold_init(cfg: &Config) -> Result<Scaffold, Error>;
pub struct Scaffold     { pub files: Vec<ScaffoldFile> }
pub struct ScaffoldFile { pub rel_path: String, pub contents: String, pub overwrite: bool }
```

Every `Config` sub-struct is `#[serde(default, deny_unknown_fields)]`: a
misspelled knob is a loud `Error::Config`, not a silently-ignored setting. See
[adoption-guide.md](adoption-guide.md) §Config for the full knob table.

---

## 4. The overlay seam: typed read-only loaders

These are the public functions an external **overlay** crate depends on to read
a generic artifact and emit its own enriched sibling (`*-<overlay>.json`) without
forking the core:

```rust
use spec_spine_core::{load_registry, load_index};

pub fn load_registry(bytes: &[u8]) -> Result<Registry,      Error>;  // rejects unknown MAJOR schema
pub fn load_index   (bytes: &[u8]) -> Result<CodebaseIndex, Error>;  // rejects unknown MAJOR schema
```

See [overlay-contract.md](overlay-contract.md) for the full extensibility
contract and [schema-versioning.md](schema-versioning.md) for what "rejects
unknown MAJOR" means.

---

## 5. Typed query layer

Read-only queries over a loaded `Registry`:

```rust
use spec_spine_core::{list, show, status_report, relationships, ListFilter};

pub fn list        (registry: &Registry, filter: &ListFilter)  -> Vec<&SpecRecord>;
pub fn show        (registry: &Registry, id: &str)             -> Result<&SpecRecord, Error>;
pub fn status_report(registry: &Registry)                      -> StatusReport;
pub fn relationships(registry: &Registry, id: &str)            -> Result<RelationshipView, Error>;
```

Authority-by-unit resolves over the **index**, where the compiler has already
pre-flattened the registry's edges into resolved units (so "who owns unit X" is a
set-membership lookup, not a runtime graph walk):

```rust
use spec_spine_core::authorities;
use spec_spine_core::types::Unit;

pub fn authorities(index: &CodebaseIndex, unit: &Unit) -> Vec<String>;  // → owning spec ids
```

---

## 6. The `Error` enum → exit codes

A single, stable, `#[non_exhaustive]` enum. The CLI is the **only** place that
maps an `Error` to a process exit code.

| Variant | Meaning | Exit |
|---|---|---|
| `Error::Validation(Vec<Violation>)` | compile validation failed | **1** |
| `Error::NotFound(String)` | spec id / view / path not found | **1** |
| `Error::Stale { expected, actual }` | committed index out of date | **2** |
| `Error::Config(String)` | malformed/invalid `spec-spine.toml` | **3** |
| `Error::Io(String)` | filesystem / read failure | **3** |
| `Error::Parse(String)` | frontmatter / TOML / JSON parse failure | **3** |
| `Error::Schema(String)` | emitted/loaded JSON failed schema or version check | **3** |

Coupling **drift** is *not* an `Error` variant; it is carried in the
`CoupleReport` and mapped to exit **1** by the CLI. `Error::exit_code(&self) ->
u8` is the authoritative mapping.

Per-subcommand exit-code table: see
[design/00-architecture.md](design/00-architecture.md) §6.

---

## 7. The JSON-in / JSON-out facade (the FFI seam)

Every top-level operation has a `&str → Result<String, Error>` facade function.
This is the seam napi / pyo3 / cgo will wrap (see
[bindings-plan.md](bindings-plan.md)); in Rust it returns a typed `Error`, which
the binding layer maps to a uniform `{ok, data, error}` envelope.

```rust
pub fn compile_json        (config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn index_json          (config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn lint_json           (config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn check_freshness_json(config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn couple_json         (request_json: &str)                 -> Result<String, Error>;
pub fn query_json          (request_json: &str)                 -> Result<String, Error>;
pub fn load_config_json    (toml_src: &str)                     -> Result<String, Error>;
pub fn scaffold_init_json  (config_json: &str)                  -> Result<String, Error>;
```

- `config_json` is a JSON object matching `Config`; `"{}"` ⇒ `Config::default()`.
- `query_json` request: `{ "registry": "<registry.json text>", "op":
  "list" | "show" | "status-report" | "relationships", "id"?: string,
  "status"?: string, "idsOnly"?: bool, "nonzeroOnly"?: bool }` (the projection
  fields, spec 010, default to `false`).
- `couple_json` request: `{ "config"?: Config, "repoRoot": string, "diff":
  DiffInput, "waiver"?: { "reason": string } }`.
- `check_freshness_json` returns `{ "fresh": bool, "expected"?, "actual"? }`.

All emitted JSON is **pretty-printed with sorted keys, LF line endings, and a
trailing newline** (diffability over compactness; see
[design/00-architecture.md](design/00-architecture.md) §10.1).

---

## 8. Binding-readiness invariants (what the boundary guarantees)

These hold across the public surface and are what make the library safe to wrap
from another language:

- Owned, `serde`-serializable plain-data DTOs: **no lifetimes, generics, or
  trait objects** at the boundary.
- A single `Error` enum with stable, documented variants → stable exit codes.
- **No `process::exit`, no `println!`-for-data, no `panic!`-on-user-input** inside
  the library. Those live only in `spec-spine-cli`.
- **No `unsafe`** anywhere (`unsafe_code = "forbid"` workspace-wide).
- Pure functions of `(Config, file bytes)`; no ambient clock/env in core.
