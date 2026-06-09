# spec-spine — Phase 0 Architecture & Design Proposal

> **Status:** Phase 0 checkpoint ⛔ — load-bearing decisions confirmed by human
> (2026-06-08); holding for explicit go-ahead to Phase 1.
> **Confirmed decisions:** (Q1) license **Apache-2.0**; (Q2) **one multi-call
> binary**; (Q3) bootstrap corpus **minimal-original**; (Q4) v1 symbol resolution
> **Rust + TypeScript**, **Python deferred**.
> **Scope:** Design only. No binding code, no consumer migration, no overlays.
> **Mandate:** Fresh architecture; *port* the reference repos' proven behavioral
> semantics rather than reinventing them.
> **Reference repos (read-only, never modified):**
> `/Users/bart/Dev/open-agentic-platform` (OAP — origin, most mature),
> `/Users/bart/DevWork/aide-agentic-template` (aide — prior de-brand, closest target),
> `/Users/bart/DevWork/template-encore` (encore — the broken-vendoring cautionary tale).

This document is the deliverable for **Phase 0** of `spec-spine-agent-prompt.md`.
It fixes the crate layout, the full `Config` schema, the `spec-spine-core` public
API, the JSON facade, the exit-code table, the schema-version plan, the
distribution plan, the license recommendation, the bootstrap-corpus outline, and
every assumption and open question. **Nothing here is built yet.**

---

## 0. What spec-spine is (one paragraph, to anchor the design)

A markdown spec corpus becomes a typed, hash-verifiable authority ledger. Each
`specs/NNN-slug/spec.md` declares, in YAML frontmatter, **typed edges** to other
specs (`establishes`/`extends`/`refines`/`supersedes`/`amends`/`co_authority`/
`constrains`/`references`) and the **authority units** it owns (file / section /
symbol). A deterministic **compiler** emits the spec-as-source `registry.json`; a
deterministic **indexer** emits the code-as-source `index.json` (with a
content-hash staleness mechanism); a **coupling gate** joins the two views at PR
time and refuses drift; a **lint** enforces corpus well-formedness; a **refusal
rule** (agent-facing, shipped as a rules file) stops an agent from "resolving"
drift by rewriting the contract. Everything is a pure function of
`(config, file contents)` so the same inputs produce byte-identical output.

---

## 1. Final crate layout

A three-crate published workspace. This is a deliberate flattening of the
reference layout: OAP/aide ship **five binary crates** (`spec-compiler`,
`codebase-indexer`, `registry-consumer`, `spec-lint`, `spec-code-coupling-check`)
plus a `spec-types` leaf and a separate `canonical-json` crate, all under
`tools/`. For an *installable library* the engine must be one importable crate
with a thin CLI on top.

```
spec-spine/
├─ Cargo.toml                       # [workspace]; resolver = "2"
├─ spec-spine.toml                  # the library governing ITS OWN repo (dogfood)
├─ rust-toolchain.toml              # pinned toolchain for reproducible builds
├─ LICENSE                          # see §8
├─ crates/
│  ├─ spec-spine-types/             # DTOs, frontmatter grammar, Config, schema-version
│  │  │                             #   consts, EMBEDDED JSON Schemas, the Error enum.
│  │  └─ schemas/                   #   registry / index / config-hash / build-meta .schema.json
│  │                                #   (include_str!'d — the crate is self-contained)
│  ├─ spec-spine-core/              # THE library: compile / index / query / lint / couple
│  │                                #   + load_registry / load_index (overlay seam)
│  │                                #   + scaffold_init + JSON facade. Internal canonical-json
│  │                                #   + content-hash + tree-sitter symbol resolver modules.
│  └─ spec-spine-cli/               # thin clap wrapper → ONE `spec-spine` multi-call binary;
│                                   #   git invocation, stdout/stderr, process::exit live HERE only.
├─ specs/                           # the library's own spec corpus (000 = bootstrap) — dogfood
├─ standards/spec/                  # constitution.md, contract.md, templates/   (generic, scaffolded by init)
├─ .claude/rules/                   # orchestrator / governed-reads / refusal rules (generic)
├─ docs/
│  ├─ design/00-architecture.md     # this file
│  ├─ adoption-guide.md             # (Phase 5)
│  ├─ bindings-plan.md              # (Phase 5)
│  ├─ api.md                        # (Phase 5)
│  ├─ overlay-contract.md           # (Phase 5)
│  └─ schema-versioning.md          # (Phase 5)
├─ examples/
│  └─ overlay-min/                  # tiny example overlay crate proving the seam (Phase 5, optional)
└─ .github/workflows/               # build, determinism, schema-conformance, self-coupling, release
```

### 1.1 Why three crates, not five binaries

| Concern | Reference (OAP/aide) | spec-spine |
|---|---|---|
| Engine | 5 separate binary crates under `tools/` | one importable lib crate `spec-spine-core` |
| Shared types | `spec_types` + `canonical-json` (2 crates) | folded into `spec-spine-types` (+ internal `canonical_json` module in core) |
| CLI surface | 5 binaries (`./bin/<name>`), a copy-not-symlink "bin strategy" (aide spec 039) | one git-style multi-call binary `spec-spine` |
| Stable boundary | the CLIs (de-facto) | the **library API** (bindings wrap this, not the CLI) |

The five-binary split exists in the references because they were *internal repo
tooling* invoked from a Makefile, and the `./bin/` copy strategy (aide spec 039)
was invented to shorten callsites. An installable artifact wants the opposite:
one `cargo install spec-spine-cli` → one `spec-spine` on `PATH`. The five
capabilities become **subcommands**, not binaries (§5.2). The "bin strategy" is
obsolete for us and is dropped.

### 1.2 Crate dependency edges

```
spec-spine-types   (leaf: serde DTOs, Config, frontmatter grammar, schema consts, Error)
        ▲
spec-spine-core    (depends on types; internal modules: canonical_json, hash, resolver[tree-sitter])
        ▲
spec-spine-cli     (depends on core + types; clap; shells out to git; owns process exit codes)
```

No `path`-only deps on the published surface; all three publish to crates.io with
real `version`/`license`/`repository`/`description`/`keywords` and **no
`publish = false`** (the references set `publish = false` everywhere — we must
not, or bindings can never depend on a published crate). `examples/overlay-min`
is `publish = false` (it is not part of the shipped surface).

---

## 2. Authority model (the conceptual core the schema serves)

### 2.1 Typed edges — 8 total, 7 ownership-bearing + 1 non-owning

Ported verbatim from `spec-spine.md` and OAP `spec-types`. Frontmatter keys are
`snake_case`; registry JSON is `camelCase`.

| Edge (frontmatter) | Ownership? | Meaning |
|---|---|---|
| `establishes` | yes | first brings code into being (historical origin) |
| `extends` | yes | adds surface to a predecessor without disturbing it |
| `refines` | yes | tightens behavior on a named aspect |
| `supersedes` | yes | replaces a predecessor (partial/full); inherits current authority |
| `amends` | yes | patches a predecessor in place; grants co-authority over its `spec.md` |
| `co_authority` | yes | shares a path with another spec on a **named section** |
| `constrains` | yes | asserts an invariant others must respect |
| `references` | **no** | points at another spec/artifact without claiming authority |

`references` is the only non-owning edge; the coupling gate ignores it. (The
8-vs-9 discrepancy in recon is whether `origin: retroactive` counts as an edge —
it does not; it is a **bootstrap marker**, not a relationship. The concept doc and
`spec-spine.md` are authoritative: **eight edge types**, with `origin` tracked
separately as frontmatter, not as a graph edge.)

### 2.2 Authority unit grammar — v1 ships file / section / symbol

A spec declares the units it owns via a `unit:` object on an edge. The full
grammar (ported from OAP `LogicalUnit`, `spec-types/src/lib.rs`) is six kinds;
**v1 resolves three**, per the build mandate (§1 of the prompt):

| Unit kind | v1? | Shape | Resolution |
|---|---|---|---|
| `file` | **v1** | `{ kind: file, path }` | literal path; trailing-`/` path ⇒ directory subtree (prefix match) |
| `section` | **v1** | `{ kind: section, file, anchor }` | anchor parser by file type (Makefile target / Markdown heading slug / `region:` marker / workflow `jobs.<name>`) |
| `symbol` | **v1** | `{ kind: symbol, id }` | tree-sitter (**Rust + TypeScript** in v1; Python deferred — Q4) → `(file, line-span)` |
| `directory` | folded | — | expressed as a `file` unit with a trailing-slash path; **not a separate kind in v1** |
| `crate` | deferred | `{ kind: crate, id }` | workspace-member validation — reserved (additive minor) |
| `module` | deferred | `{ kind: module, id }` | tree-sitter module index — reserved (additive minor) |

A bare string on an edge is shorthand for `{ kind: file, path }`. The `Unit`
enum is designed additively so `crate`/`module` slot in as a MINOR schema bump
later without breaking readers.

### 2.3 Three linkage directions (how code ↔ spec connect)

Ported from OAP `TraceSource`. The coupling gate joins all three:

1. **Manifest key** — `[package.metadata.<ns>].spec = "NNN-slug"` (Cargo) and
   top-level `{"<ns>": {"spec": "NNN-slug"}}` (package.json). `<ns>` is the
   configurable `manifest.metadata_namespace`. (crate/package → spec)
2. **Comment header** — `// Spec: specs/NNN-slug/spec.md` doc-comment at file
   root. (file → spec)
3. **Spec edges** — a spec's `establishes`/`extends`/… `unit:` declarations.
   (spec → code)

### 2.4 Authority resolution & amends-awareness (ported algorithm)

"Who currently owns unit X" is a near-pure set-membership query, *not* a runtime
graph walk — the indexer pre-flattens edges into resolved units at index time
(OAP `xref.rs`). The gate's clearance rule (OAP
`spec-code-coupling-check/src/lib.rs:legitimate_owners`):

- The owners of a path = every spec whose edge units resolve to it.
- **Amends-awareness:** if the changed path is exactly `specs/<id>/spec.md`, the
  owner set is *expanded* to include every spec that `amends` `<id>` and the
  `amendment_record` target — but **only if the base owner set is non-empty**
  (the FR-005 "strict-expansion guard": amends can add owners to an
  already-firing path, never silently enroll a new one).
- A path is **cleared** if *any one* owner's `spec.md` is in the diff.
- `supersedes` contributes the predecessor's paths into the superseding spec's
  resolved units at index time (so current authority transfers); `establishes`
  is the historical-origin edge.

This is the single most battle-tested algorithm in the references; we port it
**behaviorally intact** and cite OAP in the implementing module.

---

## 3. The `Config` schema (every knob + default) — the heart of the task

`spec-spine.toml` at the consumer repo root deserializes into a typed `Config`.
**An absent file yields a working default** for a single-Cargo-workspace repo
with `specs/` at the root. Malformed config → a clean `Error::Config`, never a
panic. Every knob below traces to a concrete divergence observed across the three
repos.

### 3.1 Full TOML with defaults

```toml
# spec-spine.toml — all keys optional; shown values are the defaults.

[manifest]
# Drives BOTH the Cargo `[package.metadata.<ns>].spec` read and the
# package.json `"<ns>".spec` read. OAP="oap", aide/encore="spec".
metadata_namespace = "spec-spine"

[domains]
# Closed enum was the #1 fork driver (OAP ["opc","platform","substrate","tooling"]
# vs aide ["app","substrate","tooling"]) and caused spurious lint warnings.
# Empty list ⇒ the `domain` field is DISABLED (free-text, no enum check).
# Non-empty ⇒ closed enum: the `domain` value, WHEN PRESENT, must be a member
# (V-error otherwise). Field absence is allowed (no forced warning).
allowed = []

[kind]
# Symmetric with [domains] (Phase-0 checkpoint item 2): `kind` is an optional
# categorical taxonomy with identical semantics. Empty ⇒ DISABLED (free-text);
# non-empty ⇒ closed enum, value validated WHEN PRESENT (V-error otherwise).
# OAP's 16-value `kind` enum + capability/registry/profile machinery is dropped
# (§10.4); this restores symmetry without re-importing that machinery.
allowed = []

[layout]
specs_dir     = "specs"             # never hardcode `specs/`
derived_dir   = ".derived"          # compiler/indexer output root (OAP renamed build/ → .derived/)
standards_dir = "standards/spec"    # constitution.md, contract.md, templates/
schemas_dir   = "standards/schemas" # where adopter-side JSON schemas live (for extra-hash + parity)
cargo_workspace = "Cargo.toml"      # root Cargo workspace manifest (relative to repo root)
# Manifests that DECLARE npm/pnpm workspace members. The indexer reads member globs
# from whichever exists, then discovers each member's package.json. THIS FIXES the
# encore bug: the default reads root package.json#workspaces (encore hardcoded
# `public/pnpm-workspace.yaml`, making all npm packages invisible).
npm_workspaces = ["package.json", "pnpm-workspace.yaml"]
# Crates/packages OUTSIDE the root workspace (aide file-ized these as
# .spec-spine/standalone-*.toml; we promote them to first-class config).
standalone_rust_workspaces = []     # e.g. ["apps/desktop/src-tauri"]
standalone_npm_packages    = []     # e.g. ["services/api"]

[index]
# Globs folded into the content-hash beyond the always-hashed core
# (= all spec.md + all discovered manifests + spec-spine.toml). OAP hashed ~10
# project-specific paths; adopters declare their own. Documented base set:
extra_hashed_inputs = ["standards/**", ".github/workflows/**"]
# Directory names pruned from symbol/section resolution walks (OAP RESOLVER_EXCLUSIONS).
resolver_exclusions = ["target", "node_modules", ".derived", "dist", "build", ".next"]

[branding]
# Appear in emitted `build` metadata. OAP "open-agentic-spec-compiler" vs aide "spec-compiler".
compiler_id = "spec-spine"
indexer_id  = "spec-spine"

[coupling]
# Paths exempt from the gate. Default = docs + lockfiles + derived + vendored, the
# generic subset of OAP's BYPASS_PREFIXES. Match rules: trailing `/` ⇒ dir prefix;
# leading `**/` ⇒ tail-suffix anywhere; else exact file. (Adopter list is ADDITIVE;
# it cannot remove an entry — matching OAP's overlay semantics.)
bypass_prefixes = [
  ".github/", "docs/", "README.md", "CHANGELOG.md", "LICENSE",
  "CODEOWNERS", ".gitignore", ".gitattributes",
  "standards/spec/constitution.md", ".derived/",
  "**/Cargo.lock", "**/package-lock.json", "**/pnpm-lock.yaml",
]
# The PR-body waiver keyword (free-text reason follows the colon).
waiver_keyword = "Spec-Drift-Waiver:"

[provenance]
# OPEN scheme registry — the closed enum forced edits to shared types
# (OAP stagecraft://,xray-fingerprint:// vs aide knowledge://,fingerprint://).
# Map of provenance kind → URI scheme. Adopters add/override freely.
[provenance.uri_schemes]
knowledge        = "knowledge://"
code-fingerprint = "fingerprint://"

[frontmatter]
# Adopters add recognized keys without forking the types crate. Unknown keys
# otherwise fall into `extra_frontmatter` (scalar/string-list only, capped).
extra_known_keys = []
```

### 3.2 Rust shape (in `spec-spine-types`)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]   // deny_unknown_fields ⇒ typos are loud, not silent
pub struct Config {
    pub manifest:    ManifestConfig,
    pub domains:     DomainsConfig,   // { allowed: Vec<String> }
    pub kind:        KindConfig,      // { allowed: Vec<String> } — symmetric with domains
    pub layout:      LayoutConfig,
    pub index:       IndexConfig,
    pub branding:    BrandingConfig,
    pub coupling:    CouplingConfig,
    pub provenance:  ProvenanceConfig,
    pub frontmatter: FrontmatterConfig,
}
impl Default for Config { /* the §3.1 defaults */ }
```

Each sub-struct is `#[serde(default, deny_unknown_fields)]`. `Config::default()`
is the working single-workspace default. `deny_unknown_fields` turns a misspelled
knob into a clear `Error::Config` instead of a silently-ignored setting — this is
the exact failure class that sank encore (a stale/missing config silently
producing wrong output).

### 3.3 Changes to the prompt's §3 knob table (recommended)

| Knob | Decision | Rationale |
|---|---|---|
| all §3 knobs | **kept** | each traces to a real divergence |
| `index.resolver_exclusions` | **added** | encore showed `RESOLVER_EXCLUSIONS` is a hardcoded layout assumption (`out/`, `coverage/` adopters get them walked) — promote to config |
| `coupling.waiver_keyword` | **added** | trivially configurable; some orgs want a house keyword |
| `provenance.uri_schemes` | **modeled as a map**, not a list | a kind→scheme map is what the closed enum actually was; cleaner than a flat list |
| `kind` enforcement | **symmetric with `domains`** (item 2) | OAP's 16-value `kind` enum + `shape`/`category`/capability/registry/profile machinery is dropped (§10.4). `kind` becomes an optional taxonomy with a `[kind] allowed` allowlist — empty ⇒ free-text/disabled, non-empty ⇒ validated-when-present — **identical semantics to `[domains]`**. Resolves the asymmetry before the `Config` struct freezes; additively extensible later. |

---

## 4. `spec-spine-core` public API

Binding-readiness rules honored throughout: owned `serde`-serializable plain-data
DTOs (no lifetimes/generics/trait-objects at the boundary); a single `Error`
enum; no `process::exit`/`println!`-for-data/`panic!`-on-user-input in the
library; pure functions of `(Config, file bytes)` with **no ambient clock/env**
(the one `build-meta.builtAt` wall-clock field lives in a separate file excluded
from determinism checks). **`git` is never invoked in core** — the CLI parses the
diff and passes a typed `DiffInput` in (a deliberate clean-up of OAP, which shells
out to git inside the coupling crate).

### 4.1 The five capabilities + freshness

```rust
// Each is a pure function of (Config, on-disk inputs under repo_root).
pub fn compile(cfg: &Config, repo_root: &Path) -> Result<CompileOutcome, Error>;
pub fn index  (cfg: &Config, repo_root: &Path) -> Result<IndexOutcome,   Error>;
pub fn lint   (cfg: &Config, repo_root: &Path) -> Result<LintReport,     Error>;

// Coupling takes already-parsed diff + optional waiver; loads registry+index from derived_dir.
pub fn couple (cfg: &Config, repo_root: &Path,
               diff: &DiffInput, waiver: Option<&Waiver>) -> Result<CoupleReport, Error>;
// Lower-level form for callers that already hold the artifacts (overlays, tests):
pub fn couple_with(cfg: &Config, registry: &Registry, index: &CodebaseIndex,
                   diff: &DiffInput, waiver: Option<&Waiver>) -> Result<CoupleReport, Error>;

// Cheap staleness check (does committed index.json's contentHash match current inputs?).
pub fn check_index_freshness(cfg: &Config, repo_root: &Path) -> Result<Freshness, Error>;
```

```rust
pub struct CompileOutcome { pub registry: Registry, pub json: String, pub validation_passed: bool }
pub struct IndexOutcome   { pub index: CodebaseIndex, pub json: String, pub content_hash: String }
pub enum   Freshness      { Fresh, Stale { expected: String, actual: String } }
```

`*.json` fields are the canonical bytes the CLI writes; typed structs let
overlays and tests work without re-parsing.

### 4.2 Config load + init scaffolding

```rust
pub fn load_config(toml_src: &str) -> Result<Config, Error>;   // validates; clean Error, no panic
// Config::default() provides the working default.

// init returns files-as-data; the CLI writes them. Keeps core IO-light & unit-testable.
pub fn scaffold_init(cfg: &Config) -> Result<Scaffold, Error>;
pub struct Scaffold { pub files: Vec<ScaffoldFile> }            // {rel_path, contents, overwrite: bool}
```

### 4.3 The overlay seam — typed read-only loaders

```rust
pub fn load_registry(bytes: &[u8]) -> Result<Registry,      Error>;  // rejects unknown MAJOR schema
pub fn load_index   (bytes: &[u8]) -> Result<CodebaseIndex, Error>;
```

These are the public functions an external overlay crate depends on to read the
generic artifact and emit a sibling (`*-<overlay>.json`) — the supported
extensibility model (OAP's enrichers do exactly this).

### 4.4 Typed query layer (over a loaded `Registry`)

```rust
impl Registry {
    pub fn list   (&self, filter: &ListFilter)  -> Vec<&SpecRecord>;
    pub fn show   (&self, id: &SpecId)          -> Option<&SpecRecord>;
    pub fn status_report(&self)                 -> StatusReport;          // counts by status
    pub fn relationships(&self, id: &SpecId)    -> Option<RelationshipView>;
}
// Authority-by-unit needs both the registry (edges) and the index (resolved units):
pub fn authorities(registry: &Registry, index: &CodebaseIndex, unit: &UnitRef) -> Vec<SpecId>;
```

### 4.5 The `Error` enum (stable variants → exit codes)

```rust
#[non_exhaustive]
pub enum Error {
    Config(String),       // malformed/invalid spec-spine.toml          → exit 3
    Validation(Vec<Violation>),  // compile validation failed            → exit 1
    NotFound(String),     // spec id / view / path not found            → exit 1
    Stale { expected: String, actual: String },  // index out of date    → exit 2
    Drift(Vec<Violation>),// coupling: uncovered paths, no waiver        → exit 1
    Io(String),           // filesystem / git / read failure            → exit 3
    Parse(String),        // frontmatter / TOML / JSON parse failure     → exit 3
    Schema(String),       // emitted/loaded JSON fails schema/version    → exit 3
}
```

`#[non_exhaustive]` so new variants are additive. Each variant documents its exit
code; the CLI is the only place that maps `Error` → process exit.

---

## 5. JSON facade & CLI

### 5.1 JSON-in / JSON-out facade (the FFI seam)

One facade fn per top-level operation, all `&str → Result<String, Error>`. The
binding layer (later) wraps each into a uniform `{ok, data, error}` envelope; in
Rust they return typed `Error`. Documented explicitly in `docs/bindings-plan.md`.

```rust
pub fn compile_json        (config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn index_json          (config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn lint_json           (config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn couple_json         (request_json: &str)                 -> Result<String, Error>;  // bundles cfg+repo_root+diff+waiver
pub fn query_json          (request_json: &str)                 -> Result<String, Error>;  // dispatch: list|show|status|relationships|authorities
pub fn check_freshness_json(config_json: &str, repo_root: &str) -> Result<String, Error>;
pub fn load_config_json    (toml_src: &str)                     -> Result<String, Error>;  // → normalized Config as JSON
pub fn scaffold_init_json  (config_json: &str)                  -> Result<String, Error>;
```

### 5.2 CLI — one multi-call `spec-spine` binary (recommended; no blocker found)

```
spec-spine compile                                  # → .derived/spec-registry/registry.json (+ build-meta.json)
spec-spine index   [check | render | orphans]       # check = staleness gate; default subcmd writes index.json
spec-spine registry list|show|status-report|authorities|relationships
spec-spine lint    [--fail-on-warn] [--fail-on-info]
spec-spine couple  [--base origin/main] [--head HEAD] [--pr-body FILE] [--paths-from FILE]
spec-spine init    [--force]
```

The CLI is a pure translation of API result → stdout/stderr + exit code. It owns:
`git diff --no-color -U0 base...head` parsing into `DiffInput`, the
`$SPEC_SPINE_PR_BODY` / `--pr-body` read for waivers, and `std::process::exit`.

---

## 6. Exit-code table

The reference table is `0` ok / `1` validation-failure-or-not-found / `2` stale /
`3` IO-parse-schema. **One reconciliation:** OAP's coupling check overloads `2`
for operational/load errors, which collides with `2 = stale`. spec-spine routes
coupling load/IO errors to `3` and reserves `2` strictly for staleness. The
unified table:

| Subcommand | `0` | `1` | `2` | `3` |
|---|---|---|---|---|
| `compile` | validation passed | validation failed | — | IO / parse / schema |
| `index` (write) | ok | — | — | IO / parse / schema |
| `index check` | fresh | — | **stale** | IO / parse |
| `registry *` | ok | not found | — | IO / parse / schema |
| `lint` | clean | error-tier (always) or warn-tier w/ `--fail-on-warn` | — | IO / parse |
| `couple` | no drift, or waived | **drift** (uncovered paths) | index stale (recompute first) | IO / parse / load |
| `init` | scaffolded | target exists w/o `--force` | — | IO write error |

---

## 7. Schema-version plan (fresh, library-owned, starts at 0.1.0)

The references run registry `specVersion 2.2.0` and index `schemaVersion 3.0.0`.
We **do not inherit those lines.** spec-spine starts every schema fresh at
`0.1.0`, decoupled from any consumer's history.

| Artifact | Field | v1 value | Owner |
|---|---|---|---|
| `registry.json` | `specVersion` | `0.1.0` | library |
| `index.json` | `schemaVersion` | `0.1.0` | library |
| `spec-spine.toml` | `config_version` (optional) | `0.1.0` | library |
| `build-meta.json` | `schemaVersion` | `0.1.0` | library (non-deterministic; excluded from golden) |

**Policy (documented in `docs/schema-versioning.md`):**

- Schema version is a **compile-time `const`** in `spec-spine-types`. The
  conformance test asserts emitted JSON validates against the embedded schema of
  that version; a mismatch fails the **build**, not runtime.
- **MINOR** bump = additive only (new optional field, new enum variant, new unit
  kind like `crate`/`module`). Old readers keep working.
- **MAJOR** bump = breaking (removed/renamed/retyped field, changed semantics).
  Loaders **reject an unknown MAJOR** with `Error::Schema`.
- **Pre-1.0 caveat:** under `0.x`, MINOR may break (standard SemVer `0.x`
  semantics). Adopters pin the toolchain version (`cargo`/release tag); the
  binary embeds the schema version; emitted artifacts carry it.
- **Schemas live INSIDE `spec-spine-types/schemas/` and are `include_str!`'d**
  (a deliberate divergence — OAP keeps them in `standards/schemas/` loaded at
  runtime). Embedding makes the published crate self-contained and the version a
  true compile-time constant. The adopter's `standards/schemas/` is for *their*
  schemas (contract-parity), not ours.

---

## 8. License recommendation

**Recommended: Apache-2.0.** The explicit patent grant (§3 of Apache-2.0) matters
the moment FFI bindings and corporate adopters arrive — exactly the trajectory of
this library. The reference repos are AGPL by deliberate choice (audit-chain as a
public good); a broadly-adoptable library + bindings wants permissive licensing.
**Not AGPL.**

Two viable permissive picks (your call at this checkpoint — §10 Q1):

- **Apache-2.0** (recommended) — patent grant + explicit contribution terms.
- **`MIT OR Apache-2.0` dual** — the Rust-ecosystem idiom; maximal downstream
  compatibility (some downstreams prefer MIT's brevity, some need Apache's patent
  grant). Slightly more boilerplate (two LICENSE files, dual SPDX in every
  `Cargo.toml`).

I lead with single **Apache-2.0** for simplicity + patent protection; dual is the
defensible alternative if you weight ecosystem-idiom over single-license
simplicity. Applied consistently across all three crates' `license =` field and a
top-level `LICENSE`.

---

## 9. Bootstrap spec corpus outline (dogfood)

**Recommendation: minimal-original** (per prompt §9 lean), not a re-derivation of
the reference corpora. spec-spine governs itself from day one with a small, clean,
purpose-built corpus.

```
specs/
├─ 000-spec-spine-bootstrap/spec.md     # THE bootstrap spec (hand-authored before the compiler exists)
├─ 001-compile-registry/spec.md         # compiler capability
├─ 002-registry-query/spec.md           # query/consumer capability
├─ 003-conformance-lint/spec.md         # lint capability
├─ 004-codebase-index/spec.md           # indexer + unit grammar capability
├─ 005-coupling-gate/spec.md            # coupling gate capability
└─ 006-init-scaffold/spec.md            # adoption / init capability
standards/spec/
├─ constitution.md                      # durable principles (tier 2)
├─ contract.md                          # normative summary
└─ templates/{spec-template.md, constitution-template.md}
.claude/rules/
├─ orchestrator-rules.md                # execute-in-order, write-output-files, stop-at-checkpoints
├─ governed-artifact-reads.md           # .derived/** read only via `spec-spine` subcommands, never ad-hoc jq
└─ adversarial-prompt-refusal.md        # the prompt-time refusal rule (coherence guard)
spec-spine.toml                         # this repo's own config (dogfood)
```

### 9.1 Constitutional tiers (ported)

1. **`specs/000` bootstrap spec** — non-overridable; defines what a spec *is*.
2. **`standards/spec/constitution.md`** — durable principles, subordinate to 000.
3. **Ordinary specs** (`001`+) — within the constitutional envelope.

### 9.2 `specs/000` frontmatter sketch (hand-authored)

```yaml
---
id: "000-spec-spine-bootstrap"
title: "Bootstrap spec system (markdown → compiled JSON authority ledger)"
status: approved
created: "2026-06-08"
summary: >
  Foundational contract: authored truth lives only in markdown (+YAML frontmatter);
  machine-consumable truth is compiler-emitted JSON only; full compilation from day one;
  determinism is non-negotiable; the typed authority graph governs who-owns-what.
origin:
  retroactive: true            # I declare authority held since before the graph existed
unamendable:                   # frozen constitutional anchors (ported concept from aide spec 000)
  - "markdown-truth-boundary"
  - "json-truth-boundary"
  - "determinism-requirement"
  - "directory-name-equals-id"
  - "typed-authority-graph"
  - "refusal-rule"
---
```

The compiler is built to satisfy `000`; once built, it compiles its own corpus
(the bootstrap order the prompt mandates).

---

## 10. Determinism, ported algorithms, and what we drop

### 10.1 Determinism rules (ported, non-configurable)

- **Content hash:** SHA-256 over `<repo-relative-POSIX-path>\0<normalized-bytes>`
  pieces, **sorted by path** before hashing. Normalization = strip UTF-8 BOM,
  `\r\n`→`\n`, `\r`→`\n`. (Ported from OAP `hash.rs`.)
- **Canonical JSON:** object keys sorted (BTreeMap), **pretty-printed** (2-space,
  LF, trailing newline). *Divergence:* OAP emits the registry compact; we emit
  pretty everywhere for diffable/mechanically-mergeable registries (the concept
  doc's stated goal). (§10 Q6.)
- **No clock/env in core.** The only wall-clock is `build-meta.json.builtAt`,
  written by the CLI, excluded from determinism/golden tests.
- **The symbol resolver is a determinism input** (Phase-0 checkpoint item 1).
  tree-sitter core and each grammar crate are pinned to **exact** versions
  (`=x.y.z`) with `Cargo.lock` committed — an unpinned grammar shifts symbol
  line-spans and surfaces as flaky goldens late, across the 5-triple release
  matrix. Phase 3 (specs 004/005) adds a **per-platform golden test for symbol
  line-spans** so a span drift fails CI on every target, not just locally.

### 10.2 Ported semantics → provenance map (cite-on-reuse)

| spec-spine behavior | Ported from |
|---|---|
| coupling diff: `git diff --no-color -U0 base...head` (merge-base, tight hunks) | OAP `spec-code-coupling-check/src/main.rs:run_git_diff_unified` |
| hunk→section by line-range overlap; Makefile/markdown/region anchor parsers | OAP `hunk_attribution.rs`, `section_parser/*` |
| amends-aware clearance + strict-expansion guard | OAP `lib.rs:legitimate_owners` (FR-005) |
| waiver = first PR-body line after keyword; global-to-run; violations → warnings | OAP `lib.rs:parse_waiver` |
| bypass match rules (dir-prefix / `**/` tail-suffix / exact) | OAP `lib.rs:is_bypass_against` |
| content-hash + LF/BOM normalization + sort-by-path | OAP `hash.rs:compute_content_hash` |
| symbol spans via tree-sitter at index time, consumed as line ranges by gate | OAP `resolver/symbol_index.rs` + `LineSpan` |
| frontmatter split (`---` fences), required keys, `extra_frontmatter` overflow | OAP/aide `spec-types:split_frontmatter`, `KNOWN_KEYS` |
| standalone-workspace override files → first-class config | aide `.spec-spine/standalone-*.toml` |
| npm workspace discovery from root manifest (NOT a hardcoded path) | the encore bug (the anti-pattern we fix) |

### 10.3 Lint / validation / diagnostic code scheme (fresh, minimal)

A clean three-band namespace (the references' V-/W-/I- soup is pruned to generic
checks; full enumeration lands in the Phase 1/3 specs):

- **`V###`** — compile-time *validation* (gate `registry.validation.passed`):
  missing required key, malformed frontmatter, duplicate id, duplicate numeric
  prefix, invalid `domain` (when `domains.allowed` non-empty), invalid `kind`
  (when `kind.allowed` non-empty), dangling
  `depends_on`, malformed `unit:`, amend-into-`unamendable`,
  `superseded` without `superseded_by`, `retired` without `retirement_rationale`.
- **`L###`** — *lint* conformance (severity error/warn/info; `--fail-on-warn`
  semantics ported): no-relationship-and-not-retroactive, missing `domain` (when
  enabled), legacy bare-path inside a workspace member, etc.
- **`I###`** — *index* diagnostics; a small **blocking** band (resolver hard
  errors) fails `index check`.

### 10.4 Deliberately dropped from the generic core (overlay territory)

OAP's `kind` 16-value enum, `shape`/`category` dims, capability/registry/profile
machinery (`provides`/`composition`/`selectable_by`/`selector`/`member_contract`/
`identity`/`selects`/`policy`), `compliance`, `factoryProjects`, and the **Claude
`config-hash.json` gate** are all OAP-specific. They are **not** in the generic
core. The overlay seam (§4.3) + `extra_frontmatter`/`frontmatter.extra_known_keys`
escape hatch let a downstream rebuild any of them as a sibling artifact without
forking.

---

## 11. Distribution plan

- **crates.io:** publish `spec-spine-types`, `spec-spine-core`, `spec-spine-cli`
  with full metadata. `cargo install spec-spine-cli` → working `spec-spine`
  binary. Publish-clean: no `path`-only deps, no `publish = false` on shipped
  crates. This also unblocks bindings (they depend on the published `*-core`).
- **Prebuilt binaries:** a tag-gated GitHub Actions release workflow producing
  per-triple archives for `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-pc-windows-msvc`, each with a `.sha256` sidecar, plus an `install.sh`
  (`curl … | sh`) that detects platform/arch and drops the binary on `PATH`.
- **SBOM** (CycloneDX per archive): nice-to-have; flagged as low-but-nonzero
  workflow time — defer unless requested (§10 Q7).

---

## 12. Quality bar (how Phase 2–5 prove correctness)

- **Determinism tests** — compile/index a fixture corpus twice; assert
  byte-identical output; golden-file tests for registry + index shapes.
- **Schema conformance** — emitted JSON validates against the embedded schema;
  version mismatch fails at build (compile-time const).
- **Exit-code tests** — assert the §6 table per subcommand.
- **Config tests** — absent (defaults) / minimal / full / malformed (clean
  `Error::Config`, no panic).
- **Adoption integration test (the §8 definition-of-done)** — scaffold a
  throwaway repo via `init`, run the full compile→index→lint→couple loop with a
  **non-default `manifest.metadata_namespace` and a custom `domains.allowed`**,
  zero source edits to the library.
- **Self-coupling (dogfood)** — this repo's own coupling gate green in its own CI.

---

## 13. Assumptions

1. **Git available at couple-time.** The CLI shells out to `git`; core takes a
   typed `DiffInput`. CI and dev machines have git.
2. **Rust + TypeScript symbol resolution in v1; Python deferred** (confirmed Q4).
   tree-sitter-rust **and** tree-sitter-typescript are wired for `symbol` units;
   `file`/`section` units are language-agnostic. Python symbols are a later minor.
   **TS resolver file scope in v1 = `.ts` / `.tsx` only** (Phase-0 checkpoint
   item 3); `.vue` `<script lang="ts">` blocks are deferred — Vue-heavy adopters
   won't parse with tree-sitter-typescript directly, and `.vue` is excludable via
   `index.resolver_exclusions` until SFC-block extraction lands in a later minor.
   (OAP only had the Rust resolver active, so the TS resolver is new clean work —
   budget for it in Phase 3.)
3. **Edition 2024 / pinned toolchain**, matching the references (rust 1.85+),
   unless a lower MSRV is requested for broader adopter reach (minor; flagged).
4. **JSON output is pretty-printed sorted-key** (diffability > compactness).
5. **Schemas embedded in the types crate** (`include_str!`), not loaded from disk.
6. **Core reads declared inputs from disk** (that is not an "env read"); the
   determinism rule forbids clock/ambient-env, not reading the corpus.
7. **`build-meta.json` is the sole non-deterministic artifact** and is excluded
   from golden/determinism checks.
8. **The Claude `config-hash` gate, compliance, factory, capability/registry/
   profile, and the `kind` enum are NOT generic** and are excluded from v1 core.
9. **Bootstrap corpus is minimal-original** (6 capability specs + 000), not a
   port of the reference corpora.
10. **Reference repos stay untouched**; nothing is migrated onto the library.

---

## 14. Open questions (with my recommendation)

Q1–Q4 are **resolved** (confirmed by human, 2026-06-08). Q5–Q11 I will proceed on
as recommended unless you redirect.

| # | Question | Resolution / recommendation |
|---|---|---|
| Q1 | **License:** Apache-2.0 vs MIT vs dual? | ✅ **Apache-2.0** (confirmed) |
| Q2 | **CLI shape:** one multi-call binary vs five? | ✅ **One multi-call `spec-spine` binary** (confirmed) |
| Q3 | **Bootstrap corpus:** minimal-original vs re-derive? | ✅ **Minimal-original** (6 capability specs + 000) (confirmed) |
| Q4 | **v1 symbol-resolution languages?** | ✅ **Rust + TypeScript** in v1; **Python deferred** (confirmed). Expands Phase 3: two tree-sitter grammars. |
| Q5 | Include `directory`/`crate`/`module` unit kinds in v1? | `directory` folded into trailing-slash `file` units; `crate`/`module` reserved for an additive minor |
| Q6 | Registry/index JSON: pretty (diffable) vs compact (OAP)? | **Pretty**, sorted keys, LF, trailing newline |
| Q7 | Per-archive CycloneDX SBOM in the release workflow? | Defer (low value/time ratio for v1); add on request |
| Q8 | `index.extra_hashed_inputs` default base set contents? | `["standards/**", ".github/workflows/**"]` + always-hashed core (specs, manifests, config) |
| Q9 | `manifest.metadata_namespace` default `"spec-spine"` ⇒ `[package.metadata.spec-spine]` (hyphenated TOML key — legal but unusual). Prefer `"spec"`? | Keep **`"spec-spine"`** (self-describing; hyphenated bare keys are valid TOML) |
| Q10 | How much provenance/`references` semantics in v1? | Ship the `references` edge + open `provenance.uri_schemes` config + basic URI well-formedness; defer rich knowledge-graph semantics |
| Q11 | MSRV / edition — match references (2024/1.85) or lower MSRV for reach? | **Match references (edition 2024)** unless you want broader adopter MSRV |

---

## 15. Phase boundary

This is the Phase 0 deliverable. **Approved 2026-06-08** (Q1–Q4 confirmed; Q5–Q11
proceed as recommended). Phase 1 implements `spec-spine-types` (DTOs, frontmatter
grammar, `Config` incl. the symmetric `[kind]`/`[domains]` allowlists,
schema-version consts, embedded schemas, `Error`) and hand-authors `specs/000`,
the constitution, contract, templates, and this repo's `spec-spine.toml` — then
stops for review.

---

## 16. Checkpoint follow-ups (fold into the named phase)

Confirmations from the Phase-0 approval, each tracked to where it lands:

- **[Phase 1]** ✅ folded here — `kind`/`domains` symmetry resolved via a `[kind]
  allowed` allowlist (§3.1/§3.2/§3.3), before the `Config` struct freezes.
- **[Phase 3 — specs 004/005]** Pin tree-sitter core + grammar crate versions
  (exact `=x.y.z`, lockfile committed) and add a **per-platform golden test for
  symbol line-spans** across the 5-triple matrix (recorded in §10.1).
- **[Phase 3 — spec 004]** TS resolver scope = `.ts`/`.tsx`; `.vue`
  `<script lang="ts">` deferred, excludable via `resolver_exclusions` (§13).
- **[Phase 5 — `docs/schema-versioning.md`]** Document that `deny_unknown_fields`
  means an **older pinned binary errors on a newer config** — correct behavior
  under the pre-1.0 pin caveat (§7), so adopters aren't surprised.
- **[Phase 5 — `docs/adoption-guide.md`]** Note that **OAP self-adoption =
  generic core + an OAP overlay crate, not drop-in** — a direct consequence of
  the §10.4 prune (compliance/factory/capability machinery lives in an overlay,
  per §4.3).
