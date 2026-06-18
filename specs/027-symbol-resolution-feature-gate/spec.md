---
id: "027-symbol-resolution-feature-gate"
title: "Feature-gate tree-sitter symbol/module resolution for dependency-free embedding"
status: draft
kind: "tooling"
created: "2026-06-18"
owner: "The spec-spine Authors"
implementation: complete
risk: low
depends_on:
  - "004-codebase-index"                 # symbol resolution (symbols.rs, index.rs)
  - "017-directory-crate-module-units"   # module resolution (build_module_index)
amends:
  - "004-codebase-index"                 # §3.3 symbol resolution is now build-feature-gated
  - "017-directory-crate-module-units"   # the Rust module index is likewise gated
extends:
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/symbols.rs"   # tree-sitter machinery moved behind the feature
      - "crates/spec-spine-core/src/index.rs"     # gated build call sites; the readers stay ungated
      - "crates/spec-spine-core/tests/index.rs"   # tree-sitter-dependent tests cfg-gated
references:
  - { unit: { kind: file, path: "docs/design/00-architecture.md" }, role: context }
summary: >
  Adds a default-enabled `symbol-resolution` cargo feature to `spec-spine-core`
  that gates the tree-sitter dependency tree (tree-sitter + the Rust/TypeScript
  grammars) and the symbol/module resolution code. The CLI and `spec-spine index`
  keep default features, so adopter and self-governance behavior is byte-for-byte
  unchanged. Library consumers that read only the committed shards
  (`load_committed_registry` / `load_committed_index`) can build with
  `default-features = false` to drop tree-sitter entirely, which removes a hard
  blocker: tree-sitter declares `links = "tree-sitter"`, so a host workspace that
  pins a different tree-sitter (e.g. an analysis crate on `^0.26` while this crate
  pins `=0.25.10`) cannot otherwise resolve a single lockfile, and adding
  `spec-spine-core` anywhere in that workspace fails resolution before compilation.
  The seam is minimal: the `SymbolIndex` / `ModuleIndex` types and their `resolve`
  lookups are plain `BTreeMap` wrappers (no tree-sitter) and stay compiled, so the
  index resolver, its readers, and `compile()` build with the feature off; only the
  `build_*` functions and their parse helpers are gated. No DTO, JSON Schema, or
  `INDEX_SCHEMA_VERSION` change: the on-disk index shape is identical. Filed off the
  OAP spec-217 engine-swap work, which is blocked workspace-wide by exactly this
  `links` clash across its eight planned read-path consumers.
---

# 027: Feature-gate symbol/module resolution

Filed off the OAP spec-217 engine-swap, which replaces an in-tree spec-spine
engine with the published library. The read-path repoint is type-correct, but the
swap is blocked workspace-wide by a native-library version conflict: this crate's
unconditional `tree-sitter =0.25.10` versus an analysis crate's `^0.26`.
`tree-sitter` sets `links = "tree-sitter"`, so cargo permits exactly one version
per dependency graph and the two are disjoint; because cargo resolves a workspace
as one lockfile, adding `spec-spine-core` anywhere makes resolution fail before
compilation. The consumers OAP repoints call only the committed-shard readers,
which do no symbol resolution, so the tree-sitter dependency they drag in is dead
weight that nonetheless gates the whole graph.

## 1. Purpose

Let a consumer embed `spec-spine-core` purely as a typed reader of the committed
registry/index shards without pulling tree-sitter into its dependency graph, while
keeping symbol/module resolution the default everywhere it is actually used (the
CLI, `spec-spine index`, and this repo's own self-governance).

The tree-sitter dependency is reachable from exactly one module (`symbols.rs`),
itself reached from exactly two call sites, both inside `index()`
(`build_symbol_index`, `build_module_index`). `compile()` and the committed-shard
readers (`load_committed_registry`, `load_committed_index`) never touch it.
Resolution is therefore separable from reading along a clean seam.

## 2. Territory

This spec additively claims, alongside spec 004 (and the spec-017 additive
extension already over the same files), the three files it edits, and amends the
contracts of 004 (§3.3 symbol resolution) and 017 (the Rust module index) in place
via edge: both capabilities are now conditional on a build feature, default-on.

- `crates/spec-spine-core/Cargo.toml`: the `[features]` table and the now-`optional`
  tree-sitter dependencies. (Manifest; not separately spec-owned, governed by this
  spec's prose and the source files below.)
- `crates/spec-spine-core/src/symbols.rs`: the `use tree_sitter::*` import, the
  `build_symbol_index` / `build_module_index` functions, and their parse helpers,
  constants, and inline tests are gated behind `symbol-resolution`. The
  `SymbolIndex` / `ModuleIndex` structs and their `resolve` methods are **not**
  gated (they carry no tree-sitter dependency).
- `crates/spec-spine-core/src/index.rs`: the two `build_*` call sites are gated,
  falling back to an empty index when the feature is off. The `symbols` type import,
  `resolve_unit`, and the resolve loop are unchanged.
- `crates/spec-spine-core/tests/index.rs`: tests that assert symbol or module span
  resolution are `#[cfg(feature = "symbol-resolution")]`-gated, with their
  resolution-only helpers/imports.

The package version bump and crates.io publish (a new cargo feature is a
semver-minor, target `0.7.0`) are out of scope here: they are a separate release
step (`docs/releasing.md`), as for every prior spec.

## 3. Behavior

### 3.1 The feature

`spec-spine-core` declares a feature `symbol-resolution`, in `default`, that
enables the (now-`optional`) `tree-sitter`, `tree-sitter-rust`, and
`tree-sitter-typescript` dependencies. The exact version pins (`=0.25.10`,
`=0.24.2`, `=0.23.2`) and the determinism rationale (a resolver line-span is a
release-matrix input) are unchanged; the dependencies become optional, not
unpinned.

### 3.2 Feature on (default)

Behavior is byte-for-byte identical to 0.6.0. `spec-spine-cli` keeps default
features, so `spec-spine compile | index | lint | couple` and the committed
shards are unchanged. Symbol units resolve to `(file, line-span)` and module units
to file/inline-block spans exactly as specs 004 and 017 define.

### 3.3 Feature off (`default-features = false`)

tree-sitter is absent from the dependency graph. `compile()` and the committed-shard
readers compile and behave identically (they never used symbols). `index()` still
runs: package discovery, comment-header and manifest linkage, and file / section /
directory / crate resolution are unaffected. Only `symbol` and `module` units go
unresolved, exactly as they would in a corpus that declared none: the symbol and
module indices are empty, and a declared symbol/module unit resolves to no
location.

This degradation is **not silent**. An unresolved *owning* symbol/module unit on a
settled spec is a blocking `I-0xx` diagnostic (spec 004 §3.5, spec 025 severity
policy), so `index check` fails (exit 2) rather than emitting a fresh-looking but
incomplete index. A consumer that compiles symbol resolution out is therefore told,
loudly, if its corpus actually needs it. Because the committed shards are only ever
produced by the CLI (default features), no `default-features = false` build can
commit a divergent artifact through the normal flow.

## 4. Functional requirements

- **FR-001 (gate exists, default-on).** `spec-spine-core` exposes a
  `symbol-resolution` feature in its `default` set that gates the three tree-sitter
  crates as `optional` dependencies.
- **FR-002 (no tree-sitter when off).** `cargo build -p spec-spine-core
  --no-default-features` succeeds with no tree-sitter crate in the dependency tree.
- **FR-003 (readers feature-independent).** `load_committed_registry`,
  `load_committed_index`, and `compile()` compile and behave identically with and
  without the feature.
- **FR-004 (default unchanged).** With default features, `index()` emits populated
  `resolved_units` with line-spans, and this repo's committed shards are unchanged.
- **FR-005 (loud degradation).** With the feature off, a corpus that declares an
  owning symbol/module unit on a settled spec yields a blocking diagnostic
  (`index check` exit 2), never a fresh-but-incomplete index.
- **FR-006 (no schema change).** No DTO, JSON Schema, or `INDEX_SCHEMA_VERSION`
  change; the on-disk index shape is identical with or without the feature.
- **FR-007 (tests both ways).** The `spec-spine-core` test suite passes both with
  and without the feature; tree-sitter-dependent tests are cfg-gated.

## 5. Acceptance criteria

- **AC-1 (no tree-sitter off).** `cargo tree -p spec-spine-core
  --no-default-features -i tree-sitter` reports no such package; with default
  features it shows `tree-sitter 0.25.10`.
- **AC-2 (default behaviorally unchanged).** `cargo test --workspace` is green and
  `spec-spine index` emits populated `resolved_units` and line-spans.
- **AC-3 (readers off).** `cargo test -p spec-spine-core --no-default-features` is
  green (the reader and file/section/directory/crate paths included), and clippy
  passes with `-D warnings` in both configurations.
- **AC-4 (CI enforces both).** CI builds, tests, and clippy-checks the crate under
  `--no-default-features`, and asserts tree-sitter is absent from that tree, so a
  regression that re-couples the readers to tree-sitter fails the build.
- **AC-5 (self-corpus delta).** The only committed-artifact change from this spec is
  its own two new registry/index shards plus a restamp of the
  `by-package/spec-spine-core` index shard (the `Cargo.toml` `[features]` edit
  re-hashes the package manifest). No other spec's `by-spec` shard changes: the
  source edits to `symbols.rs` / `index.rs` shift no resolved symbol/module span in
  this repo's own corpus, so the gate output is identical with default features.

## 6. Out of scope

- **The package version bump and release** (`0.7.0`): a separate release step.
- **Any schema or `INDEX_SCHEMA_VERSION` change** (FR-006): the index shape is
  identical with the feature on or off.
- **Removing or unpinning tree-sitter**, or changing what symbol/module resolution
  produces when it *is* enabled (specs 004 and 017 are unchanged in substance).
- **A Python grammar or any new resolved language**: still deferred.
- **Splitting the readers into a separate crate.** A feature gate is sufficient and
  keeps one published crate; a `spec-spine-read` crate is a heavier change not
  justified by the current consumers.
