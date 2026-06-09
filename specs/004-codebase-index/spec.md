---
id: "004-codebase-index"
title: "Codebase index and the file/section/symbol unit grammar"
status: approved
kind: "tooling"
created: "2026-06-09"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
  - "002-registry-query"
establishes:
  - "crates/spec-spine-core/src/index.rs"
  - "crates/spec-spine-core/src/manifest.rs"
  - "crates/spec-spine-core/src/sections.rs"
  - "crates/spec-spine-core/src/symbols.rs"
  - "crates/spec-spine-core/src/pathutil.rs"
  - "crates/spec-spine-cli/src/cmd_index.rs"
summary: >
  The codebase indexer (code-as-source view): discover compilation units via the
  configurable manifest namespace, link code to specs three ways, resolve the
  file/section/symbol unit grammar to physical (file, line-span) locations
  (tree-sitter for symbols, Rust + TypeScript in v1), emit a deterministic
  index.json with a content-hash staleness mechanism, and answer
  authorities(unit). Establishes the manifest/section/symbol resolution layer and
  the `spec-spine index` CLI subcommand.
---

# 004 — Codebase index and the unit grammar

## 1. Purpose

The inverse of the registry. For each compilation unit and each owned
file/section/symbol, record which spec(s) claim it, so the coupling gate (spec
005) can join the two views. This is where spec 000 §4.2's authority-unit grammar
becomes physical line-spans.

## 2. Territory

`spec-spine-core`'s `manifest.rs` (package discovery), `sections.rs` (section
anchor parsers), `symbols.rs` (tree-sitter symbol index), `index.rs`
(orchestration, traceability, content hash, staleness, `authorities`), and the
`spec-spine index` CLI subcommand (`cmd_index.rs`).

## 3. Behavior

### 3.1 Package discovery (manifest scan)

- **Rust**: the root Cargo workspace (`layout.cargo_workspace`) members, plus
  `layout.standalone_rust_workspaces`. Each crate's owning spec is read from
  `[package.metadata.<ns>].spec`, where `<ns>` is `manifest.metadata_namespace`.
- **npm/pnpm**: the workspace member globs declared by whichever of
  `layout.npm_workspaces` exists at the root — `package.json`'s `workspaces`
  array and/or `pnpm-workspace.yaml`'s `packages` — plus
  `layout.standalone_npm_packages`. Each package's owning spec is read from a
  top-level `"<ns>".spec`. The default reading root `package.json#workspaces`
  is the fix for the template-encore failure (a hardcoded `public/pnpm-workspace.yaml`
  made every npm package invisible).
- Package discovery MUST NOT descend into `index.resolver_exclusions` directories.

### 3.2 Linkage (code → spec), three sources

1. **manifest** — the `[package.metadata.<ns>].spec` / `"<ns>".spec` key.
2. **comment header** — a `// Spec: <specs_dir>/NNN-slug/spec.md` doc comment at
   a source file's root.
3. **spec edges** — a spec's `establishes`/`extends`/`refines`/`supersedes`/
   `amends`/`co_authority`/`constrains` `unit:` declarations.

`references` is non-owning and contributes no traceability.

### 3.3 Unit resolution (the grammar → physical locations)

- **file** — a literal repo-relative path (`span` absent ⇒ whole file). A
  trailing-slash path resolves to every file in that directory subtree
  (excluding `resolver_exclusions`).
- **section** — `{file, anchor}`, resolved by `sections.rs` dispatching on file
  type to a span: a **Makefile target** (`name:` or `## tag: name`), a
  **Markdown heading** (kebab-slug of the heading text, span to the next
  same-or-higher heading), a **`region:` marker** (`<comment> region: name` …
  `<comment> endregion`), or a **CI job** (`jobs.<name>:` block by indentation).
- **symbol** — `{id}`, a `::`-qualified path to a top-level item, resolved by
  `symbols.rs` via tree-sitter to `(file, line-span)`.
  - **v1 language scope**: Rust (`.rs`) and **TypeScript `.ts`/`.tsx` only**.
    `.vue` `<script lang="ts">` single-file-component blocks are **deferred** —
    tree-sitter-typescript cannot parse a `.vue` file directly; such files are
    excluded via `index.resolver_exclusions` until SFC-block extraction lands in
    a later minor.
  - Only top-level items are indexed in v1 (no `impl` methods, no inline `mod`
    bodies); an unresolved symbol unit is a diagnostic, not a panic.

### 3.4 Determinism

`index.json` is a pure function of `(config, file contents)`. **All discovery is
path-sorted before hashing and before emission** (package records, mappings,
resolved units, and locations are sorted by stable keys), and the content hash
folds its inputs in path order regardless of filesystem walk order. The
tree-sitter core and grammar crates are pinned to exact versions so symbol
line-spans are identical across the release matrix.

### 3.5 Content hash & staleness

`build.contentHash` is SHA-256 over the normalized, **path-sorted** set of: every
discovered manifest, every `spec.md`, and every file matched by
`index.extra_hashed_inputs`. `spec-spine index check` recomputes this over
current inputs and compares it to the committed `index.json`; a mismatch is
`Stale` (exit 2). Resolver hard-error diagnostics (`I-003`..`I-009`) also fail
`check`.

### 3.6 Traceability and diagnostics

`traceability` carries `mappings` (per spec: implementing paths + resolved
units), `orphanedSpecs` (specs claiming code that resolves nowhere), and
`untracedCode` (packages with no governing spec). Diagnostics use `I-###` codes;
the `I-003`..`I-009` band are resolver hard errors.

### 3.7 authorities(unit)

`authorities(registry, index, unit)` answers "who currently owns this unit?" — a
set query over the resolved traceability, the input the coupling gate (spec 005)
and the `registry`-side consumer share.

## 4. Out of scope

The coupling gate itself (diff-hunk → unit matching, waivers) is spec 005. Symbol
resolution for languages beyond Rust/TypeScript, `impl`-method and inline-`mod`
granularity, and `.vue` SFC blocks are deferred to later minors.
