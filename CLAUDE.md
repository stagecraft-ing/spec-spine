# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

spec-spine turns a markdown spec corpus into a typed, hash-verifiable authority
ledger and **refuses code that drifts from its owning spec** at PR time. It is a
three-crate Rust workspace publishing an installable library + CLI, plus npm and
PyPI shims that ship the prebuilt binary. Read `docs/design/00-architecture.md`
first: it is the load-bearing design doc (crate layout, full `Config`, public
API, exit codes, schema plan, and the provenance of every ported algorithm).

## Commands

The toolchain is pinned in `rust-toolchain.toml` (channel `1.92.0`); MSRV is
`1.85` / edition 2024. Always pass `--locked`; CI does, and the committed
`Cargo.lock` is part of the determinism contract (tree-sitter grammars are
pinned exact).

```sh
cargo build --workspace --locked
cargo test  --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check

# A single test / one test file:
cargo test -p spec-spine-core --test couple            # one integration test file
cargo test -p spec-spine-core compile::                # tests matching a path
cargo test --workspace emitted_registry_conforms       # one test by name

# Run the CLI from the checkout (the dogfood gate chain):
cargo run -p spec-spine-cli -- compile                 # writes .derived/spec-registry/registry.json
cargo run -p spec-spine-cli -- index check             # staleness gate (exit 2 if stale)
cargo run -p spec-spine-cli -- lint --fail-on-warn
cargo run -p spec-spine-cli -- couple --base origin/main --head HEAD
```

Exit codes are a stable contract: `0` ok · `1` validation failure / not found /
drift · `2` stale · `3` I/O / parse / schema / config. They are mapped in
exactly one place (`crates/spec-spine-cli/src/main.rs` via `Error::exit_code()`).

## Architecture

Three crates, strict one-directional dependency `types → core → cli`:

- **`spec-spine-types`**, the plain-data substrate: `Config` (the
  `spec-spine.toml` model), the frontmatter grammar, the typed-edge and
  authority-unit vocabulary, registry/index DTOs, schema-version `const`s, the
  embedded JSON Schemas (`schemas/*.schema.json`, `include_str!`'d so the crate
  is self-contained), and the `Error` enum. Everything is owned, serde, with
  no lifetimes/generics/trait-objects at the boundary, so the same types back
  both the engine and future FFI bindings.
- **`spec-spine-core`**, the engine. One module per capability (`compile`,
  `index`, `query`, `lint`, `couple`, `scaffold`) plus internal
  `canonical_json` / `hash` / `markdown` / `sections` / `symbols`. **The library
  API is the stable surface bindings wrap, not the CLI.**
- **`spec-spine-cli`**, a thin clap wrapper. One `cmd_*.rs` per subcommand.

**Invariants that shape every change to core (do not violate without updating
the design doc):**

- Every artifact-producing function is a **pure function of `(Config, file
  contents)`**: no ambient clock, no env reads, **no `git`**. The CLI parses
  `git diff` and passes a typed `DiffInput` in; the library never shells out.
  The only wall-clock value (`build-meta.json`'s `builtAt`) is written by the
  CLI and excluded from determinism/golden tests.
- **`unsafe` is `forbid`-en** workspace-wide (`Cargo.toml [workspace.lints]`).
- Core is IO-light and panic-free on user input: malformed config/frontmatter
  yields a clean `Error`, never a panic. `init` returns files-as-data
  (`Scaffold`); the CLI writes them.
- The **JSON-in/JSON-out facade** in `core/src/lib.rs` (`compile_json`,
  `query_json`, `couple_json`, …) is the FFI seam. Keep it `&str → Result<String,
  Error>` and additive.

### The authority model

Each `specs/NNN-slug/spec.md` declares, in YAML frontmatter, **typed edges** to
other specs and the **authority units** it owns. Eight edges; `references` is the
only non-owning one (the coupling gate ignores it): `establishes`, `extends`,
`refines`, `supersedes`, `amends`, `co_authority`, `constrains`, `references`.
`origin.retroactive` is a bootstrap marker, not an edge. Units resolve to
code as `file` (bare string = file shorthand; trailing `/` = subtree), `section`
(`{file, anchor}`), or `symbol` (`{id}`, resolved by tree-sitter, Rust + TS in
v1; Python deferred). `crate`/`module`/`directory` kinds are reserved.

Two views, joined by the gate: `compile` emits `registry.json` (spec-as-source);
`index` emits `index.json` (code-as-source, with a content-hash staleness
mechanism). The **gate chain is `compile → index → lint → couple`**. The
coupling clearance algorithm (amends-awareness, the strict-expansion guard,
waiver parsing, bypass matching) is ported behaviorally intact from OAP. Modules
cite their source; preserve the cited semantics when editing `couple.rs`.

## Determinism is the central claim

Emitted JSON is sorted-key, pretty-printed (2-space), LF, trailing newline.
Content hashes are SHA-256 over `<repo-relative-POSIX-path>\0<normalized-bytes>`
sorted by path, where normalization strips the BOM and converts CRLF/CR to LF
(`.gitattributes` also enforces LF on checkout). `.github/workflows/determinism.yml`
proves `registry.json` + `index.json` are **byte-identical across four release
triples** (incl. tree-sitter symbol line-spans), not just locally. If you change
emission, expect that gate to be the real test.

## Self-governance (dogfood): why `.derived/` is committed

This repo runs its own gates against its own corpus in CI (`.github/workflows/ci.yml`
`dogfood` job). Consequences:

- `.derived/spec-registry/registry.json` and `.derived/codebase-index/index.json`
  are **committed** (only `build-meta.json` is gitignored). After any change that
  affects them, regenerate and commit: `cargo run -p spec-spine-cli -- compile`
  then `... index`. The `index check` step fails CI (exit 2) if the committed
  index is stale; the coupling gate (PR-only) fails if code drifts from its spec.
- Editing code under a path owned by a spec generally requires also editing that
  spec's `spec.md` (or adding a `Spec-Drift-Waiver:` line to the PR body). The
  bypass floor (docs, lockfiles, `.derived/`, per `couple.rs::DEFAULT_BYPASS_PREFIXES`,
  extended by `spec-spine.toml [coupling] bypass_prefixes`) exempts non-code paths.
- **Merge conflicts on the committed artifacts (spec 020).** Because both carry a
  global content hash, two branches that regenerated them conflict textually. An
  opt-in per-clone git merge driver resolves this by regenerating from the merged
  tree. Enable it once per clone: `./.githooks/enable-merge-driver.sh` (build the
  binary first; the driver shells out to `spec-spine compile && index`). It is
  inert until registered, and never replaces the `index check` staleness gate.

When working on a feature, add or amend the governing spec under `specs/` in the
same change. `standards/spec/` holds the constitution + contract + templates;
`specs/000-spec-spine-bootstrap` is tier-1 (its `unamendable` anchors are
non-overridable). New specs are filed as the next `NNN-slug` directory.

## Schema & release versioning (two decoupled axes)

- **Schema versions** (`registry`/`index`/`build-meta`/`config`) are compile-time
  `const`s in `spec-spine-types/src/version.rs`. The conformance test
  (`core/tests/conformance.rs`) asserts emitted JSON validates against the
  embedded schema of that version: a DTO/schema drift fails the **build**.
  MINOR = additive only; MAJOR = breaking (loaders reject an unknown MAJOR). See
  `docs/schema-versioning.md`.
- **Package version** lives in three files (`Cargo.toml`, `npm/package.json`,
  `py/pyproject.toml`) and must agree at release time. Bump them in lockstep with
  `scripts/bump_version.py <x.y.z>` (`--check` verifies agreement). This is
  **independent** of schema versions: a release can ship without a schema change.
  Maintainer release runbook: `docs/releasing.md`.

The npm (`npm/`) and PyPI (`py/`) directories are binary-distribution shims (specs
007/008): they ship the prebuilt binary per platform, assembled from release
archives at publish time. The platform packages and binaries are never committed
(`.gitignore`). Don't hand-edit generated platform packages.
