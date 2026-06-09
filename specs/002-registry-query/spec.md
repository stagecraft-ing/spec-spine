---
id: "002-registry-query"
title: "Typed read-only query over the registry"
status: approved
kind: "tooling"
created: "2026-06-08"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
establishes:
  - "crates/spec-spine-core/src/query.rs"
  - "crates/spec-spine-cli/src/cmd_registry.rs"
extends:
  - spec: "001-compile-registry"
    unit: { kind: file, path: "crates/spec-spine-core/src/lib.rs" }
    nature: additive
summary: >
  The query capability: typed, read-only access over a loaded registry: list,
  show, status-report, and relationships; plus load_registry (which rejects an
  unknown MAJOR schema version). Establishes the query module and the
  `spec-spine registry` CLI subcommands; extends 001's public surface additively.
---

# 002: Typed read-only query over the registry

## 1. Purpose

Reads of the compiled registry go through a typed consumer, never ad-hoc JSON
parsing (spec 000 §1 corollary). This capability is that consumer.

## 2. Territory

`spec-spine-core`'s `query.rs` (the query API and `load_registry`) and the
`spec-spine registry` CLI subcommands (`cmd_registry.rs`). It additively extends
001's `lib.rs` public surface with the query exports.

## 3. Behavior

- `load_registry(bytes)` MUST parse `registry.json` into a typed `Registry` and
  **reject an unknown MAJOR** schema version with `Error::Schema` (per the
  versioning policy, spec 000 §6 / design §7).
- `Registry::list(filter)` returns specs (optionally filtered by status), in
  `id` order.
- `Registry::show(id)` returns one spec or `Error::NotFound` (exit 1).
- `Registry::status_report()` returns counts by status.
- `Registry::relationships(id)` returns the spec's outgoing edges and the
  incoming edges that target it (who supersedes / amends / references / depends
  on it), computed by scanning the corpus.
- The CLI maps these to `spec-spine registry list|show|status-report|
  relationships`, printing human-readable output; `--json` emits the typed DTO.
  Exit `0` ok, `1` not found, `3` on I/O / parse / schema error.

## 4. Out of scope

`authorities(unit)`, "who currently owns this code unit?", requires the
codebase index and is defined with the indexer (spec 004). Mutations of any kind
are out of scope; this capability is read-only.
