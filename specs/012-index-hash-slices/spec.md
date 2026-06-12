---
id: "012-index-hash-slices"
title: "Index: named hash slices and `index check --slice <name>`"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "004-codebase-index"
extends:
  - { spec: "004-codebase-index", unit: "crates/spec-spine-core/src/index.rs", nature: additive }
  - { spec: "004-codebase-index", unit: "crates/spec-spine-cli/src/cmd_index.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/config.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/codebase.rs", nature: additive }
  # The MINOR bump touches the version constant and the embedded JSON schema;
  # the re-export and e2e tests touch 001's surface. All additive, same shape
  # as specs 010/011.
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/version.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/schemas/codebase-index.schema.json", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/tests/dtos.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/tests/cli.rs", nature: additive }
summary: >
  Named sub-hashes of the index's input set: an adopter declares glob groups
  under `[index.slices]`, the indexer emits a per-slice content hash into
  `build.sliceHashes` (additive index-schema MINOR), and `spec-spine index
  check --slice <name>` gates on exactly that slice -- fresh (0), stale (2),
  config/IO error (3). Generalizes the narrow high-value staleness gate OAP
  runs over its agent-config files (settings/MCP JSON) without hardcoding any
  adopter's file list into the core.
---

# 012: Named hash slices

## 1. Purpose

The full `index check` is the right default gate, but some inputs deserve a
*dedicated* gate with their own CI job and failure message: files whose quiet
drift is a governance event, not a freshness event. The motivating consumer is
OAP's config-slice gate (its specs 184/188): a narrow check over
`.claude/settings.json` + `.mcp.json` that fails PRs independently of -- and
with a sharper message than -- the broad staleness check. Today that exists as
a hardcoded `check-config` subcommand in OAP's fork-ancestor indexer. The
library generalizes it: **slices are adopter config, not core vocabulary.**

## 2. Territory

Config grammar (`config.rs`: the `[index.slices]` table), the index DTO
(`codebase.rs`: `build.sliceHashes`), slice hashing + checking (`index.rs`),
and the CLI flag (`cmd_index.rs`). The MINOR bump rides through `version.rs`
and the embedded index JSON schema; the new check surfaces through 001's
`lib.rs` re-export list and its e2e test file. All additive.

## 3. Behavior

### 3.1 Config

```toml
[index.slices]
agent-config = [".claude/settings.json", ".mcp.json"]
workflows    = [".github/workflows/**"]
```

- Each key is a slice name (`[a-z0-9][a-z0-9-]*`); each value is a non-empty
  glob list with the same pattern semantics as `index.extra_hashed_inputs`.
- Slices are **independent** of the global hash: a slice's files need not
  appear in the global input set, and listing them in a slice does NOT add
  them to it. An adopter wanting both gates lists the globs in both knobs.
  (Stated explicitly so nobody "deduplicates" the overlap away.)
- Default: no slices. Absent table ⇒ behavior identical to pre-012.

### 3.2 Emission

- `spec-spine index` computes, per slice, SHA-256 over the slice's matched
  files using the **same normalization and path-sorted folding** as
  `build.contentHash` (004 §3.5), and emits
  `build.sliceHashes: { "<name>": "<hex>" }` (key-sorted; omitted entirely
  when no slices are configured).
- A slice matching zero files is valid (hash of the empty input sequence) --
  deletion of a guarded file must read as a hash *change*, not a config error.
- Index schema: **MINOR bump** (additive optional field; loaders tolerate
  absence per `docs/schema-versioning.md`).

### 3.3 `index check --slice <name>`

- Recomputes only the named slice over the current tree and compares against
  the committed `build.sliceHashes.<name>`.
- Exit `0` fresh; `2` stale (mismatch, OR the committed index has no entry for
  a configured slice -- an index predating the slice config is by definition
  not vouching for it); `3` when the name is not in `[index.slices]`, or on
  I/O / parse / schema failure.
- Plain `index check` (no flag) is unchanged: it gates the global hash only.
  It does NOT additionally verify slices -- one gate per invocation keeps CI
  failure messages single-subject (the adopter wires one job per slice).
- `--slice` takes exactly one name; repeat the invocation for multiple slices.

### 3.4 Determinism

Slice hashes are pure functions of `(config, file contents)`; byte-identical
across the release matrix, proven by the same fixture style as the global
hash.

### 3.5 Tests (minimum)

- Emission: configured slices appear key-sorted; no config ⇒ field absent.
- Check: fresh, content-drift stale, file-deletion stale, missing-entry stale,
  unknown-name error.
- Independence: a slice-only file's edit does not trip plain `index check`,
  and vice versa.

## 4. Out of scope

Hardcoded slice names or any adopter's file list in core. Multi-slice or
all-slices check forms (`--slice a --slice b`, `check --all-slices`) -- defer
until a real consumer needs them. Folding slices into the coupling gate's
freshness guard (005 keeps consulting the global hash only).
