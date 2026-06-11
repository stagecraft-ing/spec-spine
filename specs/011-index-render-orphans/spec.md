---
id: "011-index-render-orphans"
title: "Index: implement the reserved render and orphans subcommands"
status: draft
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "004-codebase-index"
establishes:
  - "crates/spec-spine-core/src/render.rs"
  - "crates/spec-spine-core/tests/render.rs"
extends:
  - { spec: "004-codebase-index", unit: "crates/spec-spine-cli/src/cmd_index.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  # §3.4's missing-index / orphans-form e2e tests live in 001's exit-code
  # contract file, same additive shape as spec 010.
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/tests/cli.rs", nature: additive }
summary: >
  Implements the two subcommands spec 004's CLI reserved but stubbed:
  `spec-spine index render` (a deterministic human-shaped markdown projection
  of the committed index.json -- package inventory, traceability, diagnostics)
  and `spec-spine index orphans` (the orphanedSpecs list, newline ids or a
  --json array). Both are pure read-side projections of the committed
  artifact; neither recomputes the index. Establishes the render module;
  extends the index CLI dispatch.
---

# 011: `index render` and `index orphans`

## 1. Purpose

The committed `index.json` is machine truth; humans and agent session-init
protocols need a governed human-shaped view of it without ad-hoc JSON parsing.
OAP's Makefile and cross-agent init protocol call `render` on every session
start, and its workflow rules document `orphans` as the governed way to list
specs whose claims resolve nowhere. The subcommands already exist in the CLI
grammar as reserved stubs; this spec gives them their one honest
implementation: **projections of the committed artifact, never recomputation**
(recomputation is `index`; freshness is `index check` -- three verbs, three
jobs).

## 2. Territory

A new `render.rs` in `spec-spine-core` (markdown projection, plus the
`orphans` accessor) with its tests; the `cmd_index.rs` dispatch gains the two
live arms; `lib.rs` exports the projection API (and a `render_json` /
`orphans_json` facade entry consistent with the existing JSON facade). The
e2e exit-code tests of §3.4 additively extend 001's `cli.rs`.

## 3. Behavior

### 3.1 Common contract

- Both subcommands **read the committed** `<derived_dir>/codebase-index/
  index.json` via `load_index` (rejecting unknown MAJOR). Missing artifact ⇒
  `Error::Io` with a message pointing at `spec-spine index` (exit 3).
- Neither consults the working tree, recomputes hashes, nor warns about
  staleness -- a stale-but-loadable index renders; freshness is `check`'s job.
- Output is a pure function of `(config, index.json bytes)`: byte-identical
  across platforms, LF line endings, trailing newline.

### 3.2 `index render` -- the markdown projection

Emits to stdout, in this order:

1. **Header**: title (`branding.indexer_id`), the index `schemaVersion`, and
   the `build.contentHash` (so a rendered view is traceable to the exact
   artifact that produced it).
2. **Package inventory**: one table over all discovered packages -- name, path,
   kind, version, owning spec (the manifest-linkage spec id, or `-`). Sorted
   by name, ties by path.
3. **Traceability**: per-spec mapping summary (spec id, status, count of
   implementing paths, count of resolved units), id-sorted; then
   `orphanedSpecs` and `untracedCode` as flat lists (omit either section when
   empty).
4. **Diagnostics**: every `I-###` diagnostic with severity and message, sorted
   by (code, file). Omit the section when empty.

The section inventory above is the v1 contract: adopters MAY rely on the
*presence and order* of these sections; the precise prose between them is not
contractual. Exit `0` on success (diagnostics in the artifact do not fail a
render), `3` on I/O / parse / schema.

### 3.3 `index orphans`

- Text mode: newline-delimited spec ids from `traceability.orphanedSpecs`,
  id-sorted; empty list ⇒ empty output.
- `--json`: a JSON array of id strings.
- Exit `0` whether or not orphans exist (a query, not a gate; gating on
  orphans is an adopter's CI policy choice, e.g.
  `test -z "$(spec-spine index orphans)"`), `3` on I/O / parse / schema.

### 3.4 Tests (minimum)

- Golden-file test of the rendered markdown against a fixture index
  (byte-exact, the determinism proof).
- Render with empty orphans/untraced/diagnostics sections (omission rule).
- Orphans text + JSON forms; missing-index error path for both subcommands.

## 4. Out of scope

Overlay layers (domain-specific render sections belong to overlay crates per
`docs/overlay-contract.md` -- an overlay renders its own sibling view; this
render is the generic core's view and takes no extension hooks in v1).
Rendering the *registry* (the registry's human view is the spec corpus
itself). Any staleness signaling inside render output.
