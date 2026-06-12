---
id: "010-registry-query-projection-flags"
title: "Registry query: --ids-only and --nonzero-only projection flags"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "002-registry-query"
extends:
  - { spec: "002-registry-query", unit: "crates/spec-spine-core/src/query.rs", nature: additive }
  - { spec: "002-registry-query", unit: "crates/spec-spine-cli/src/cmd_registry.rs", nature: additive }
  # §3.3 widens the query_json facade (and the re-export list) in 001's lib.rs;
  # §3.4's e2e tests live in 001's cli.rs. Both additive, same shape as 002/005.
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/tests/cli.rs", nature: additive }
summary: >
  Two additive projection flags on the spec 002 query verbs: `registry list
  --ids-only` (newline-delimited spec ids; a JSON string array under --json)
  and `registry status-report --nonzero-only` (omit zero-count statuses).
  Both are load-bearing in adopter session-init protocols (OAP's /init and
  /setup read exactly these projections); neither changes any existing output
  shape when absent.
---

# 010: Query projection flags

## 1. Purpose

Adopter automation consumes two narrow projections of the registry that spec
002's verbs almost -- but not quite -- provide. OAP's cross-agent init protocol
(AGENTS.md) runs `list --ids-only` for latest-spec detection and
`status-report --json --nonzero-only` for lifecycle counts on every session
start. Without these flags the consumer either post-processes JSON in shell
(exactly the ad-hoc parsing spec 000's read discipline forbids) or carries a
fork. Both flags are pure projections: no new data, no new queries.

## 2. Territory

`query.rs` (projection options on `list` / `status_report`) and
`cmd_registry.rs` (flag parsing + rendering), plus additive touches to 001's
`lib.rs` (the `query_json` option fields of §3.3 and the re-export list) and
its e2e test file `cli.rs` (§3.4). Spec 002 remains the owner of the verbs
themselves.

## 3. Behavior

### 3.1 `registry list --ids-only`

- Text mode: newline-delimited spec ids, in `id` order, nothing else. Empty
  corpus ⇒ empty output, exit 0.
- With `--json`: a JSON array of id strings (not record objects), same order.
- Composes with `--status`: the filter applies first, then the projection.

### 3.2 `registry status-report --nonzero-only`

- Statuses whose count is zero are omitted from both human and `--json`
  output. The total line (human) / total field (JSON), if present, is
  unaffected -- it reflects the whole corpus.
- Without the flag, output is byte-identical to pre-010 behavior.

### 3.3 Contract

- Exit codes unchanged (`0` ok, `1` not found, `3` I/O / parse / schema).
- The JSON facade (`query_json`) accepts the corresponding option fields;
  absent fields default to current behavior, so existing callers are
  untouched (schema-versioning: additive, no bump needed -- the registry
  artifact itself is unchanged; only CLI/API output projections are added).
- Both flags are deterministic projections of `registry.json`; same input,
  byte-identical output.

### 3.4 Tests (minimum)

- `--ids-only` text and JSON forms; with and without `--status`.
- `--nonzero-only` against a corpus with at least one zero-count status; and
  the no-flag byte-identity check against the pre-010 fixture.

## 4. Out of scope

New query verbs (graph traversal, by-authority -- consciously deferred; see the
amendments README). Filtering extensions beyond composing with the existing
`--status`. Changes to `registry.json` itself.
