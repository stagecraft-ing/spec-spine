---
id: "024-index-sharding"
title: "Per-unit shard storage (conflict-free committed registry + index)"
status: approved
created: "2026-06-16"
authors: ["The spec-spine Authors"]
kind: tooling
implementation: complete
risk: medium
summary: >
  The two committed artifacts (the spec registry and the codebase index) were
  each emitted as one file behind one global content-hash line with id-sorted
  arrays. Because every spec or package PR rewrites that one hash line, two PRs
  touching different units collide textually, and GitHub's server-side merge (the
  web button and the merge queue's speculative build) does not run the spec-020
  merge driver, so the conflict surfaces there and the merge queue ejects sibling
  PRs. This spec shards both artifacts by authority unit: per-spec registry shards
  under spec-registry/by-spec/<id>.json, per-spec traceability shards under
  codebase-index/by-spec/<id>.json, and per-package inventory shards under
  codebase-index/by-package/<slug>.json, each carrying its own input hash. The
  global view (validation, orphans, untraced code, the aggregate content hash) is
  computed ON READ from the shard set rather than committed, so there is no shared
  file to conflict on. Two PRs touching different specs then write disjoint files
  and never conflict, so the merge queue forms clean speculative stacks and
  auto-merge stops ejecting in-flight PRs. Staleness becomes per-shard (sharper
  than the broad hash), the committed shards stay present-on-clone, and a MAJOR
  schema bump (registry + index -> 1.0.0) marks the new on-disk shape. This
  generalizes spec 012 (index hash slices) from scoped staleness CHECKING to
  per-unit STORAGE, and it removes the textual-conflict cause spec 020's merge
  driver only papers over locally.
depends_on:
  - "001-compile-registry"        # the registry compiler, now a shard emitter
  - "002-registry-query"          # the registry queries, now reading shards
  - "004-codebase-index"          # the index, now a shard emitter
  - "005-coupling-gate"           # the gate, now assembling both views from shards
  - "012-index-hash-slices"       # generalized: a slice was a hash field, a shard is its own file
  - "020-derived-artifact-merge-driver"  # narrowed to the rare same-shard conflict
establishes:
  - "crates/spec-spine-core/src/shard.rs"
  - "crates/spec-spine-types/schemas/codebase-index-spec-shard.schema.json"
  - "crates/spec-spine-types/schemas/codebase-index-package-shard.schema.json"
  - "crates/spec-spine-types/schemas/registry-spec-shard.schema.json"
extends:
  # The index surface (spec 004): emit shards, assemble on read, per-shard staleness.
  - { spec: "004-codebase-index", unit: "crates/spec-spine-core/src/index.rs" }
  - { spec: "004-codebase-index", unit: "crates/spec-spine-types/src/codebase.rs" }
  - { spec: "004-codebase-index", unit: "crates/spec-spine-cli/src/cmd_index.rs" }
  - { spec: "004-codebase-index", unit: "crates/spec-spine-core/tests/index.rs" }
  # The registry surface (spec 001): emit shards, assemble on read.
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/compile.rs" }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-types/src/registry.rs" }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/src/cmd_compile.rs" }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/tests/cli.rs", nature: additive }
  # The registry queries (spec 002): read the assembled registry.
  - { spec: "002-registry-query", unit: "crates/spec-spine-cli/src/cmd_registry.rs" }
  # The coupling gate (spec 005): committed-artifact loaders route through the assemblers.
  - { spec: "005-coupling-gate", unit: "crates/spec-spine-core/src/couple.rs" }
  - { spec: "005-coupling-gate", unit: "crates/spec-spine-cli/src/cmd_couple.rs" }
  # Schema-version constants + embedded shard schemas (spec 012 owns version/schema).
  - { spec: "012-index-hash-slices", unit: "crates/spec-spine-types/src/version.rs" }
  - { spec: "012-index-hash-slices", unit: "crates/spec-spine-types/src/schema.rs" }
  - { spec: "012-index-hash-slices", unit: "crates/spec-spine-types/tests/dtos.rs" }
  - { spec: "012-index-hash-slices", unit: "crates/spec-spine-core/tests/conformance.rs" }
  # The public type prelude (spec 000 owns the types lib root).
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/lib.rs", nature: additive }
  # The render/orphans projection tests (spec 011).
  - { spec: "011-index-render-orphans", unit: "crates/spec-spine-core/tests/render.rs" }
  # The adopter-loop test (spec 006) asserts the package shard, not index.json.
  - { spec: "006-init-scaffold", unit: "crates/spec-spine-cli/tests/init.rs" }
  # The merge driver (spec 020): narrowed to the rare same-shard conflict; its
  # registration moves to the shard globs.
  - { spec: "020-derived-artifact-merge-driver", unit: ".githooks/merge-derived-index.sh" }
  - { spec: "020-derived-artifact-merge-driver", unit: ".githooks/enable-merge-driver.sh" }
references:
  # The determinism contract this storage change must preserve (bypass-floor docs).
  - { unit: { kind: file, path: "docs/design/00-architecture.md" }, role: context }
  - { unit: { kind: file, path: "specs/020-derived-artifact-merge-driver/spec.md" }, role: context }
  - { unit: { kind: file, path: "specs/012-index-hash-slices/spec.md" }, role: context }
---

# 024: Per-unit shard storage

**Input**: A downstream adopter (open-agentic-platform) hit the recurring
multi-PR conflict on the committed index and de-committed it as an interim (OAP
spec 188 Phase 4b), which works but loses present-on-clone. This spec is the
durable, generic fix that keeps both committed artifacts present-on-clone while
making concurrent PRs conflict-free.

## 1. Purpose

A committed artifact carrying a global content-hash line is a merge
serialization point. spec 020 (the merge driver) auto-resolves the conflict on a
local clone, but GitHub's server-side merge (web button and the merge queue's
speculative build) does not run a custom merge driver, so the conflict still
surfaces there: the merge queue cannot form a speculative stack of two PRs that
both rewrote the hash line, and it ejects the second. The adopter's symptom is
that PRs never merge concurrently and auto-merge "throws off" in-flight PRs.

The root cause is storage, not information. The registry already holds per-spec
data (`specs` is one record per spec) and the index already holds per-spec data
(`traceability.mappings`) and per-package data (`packages`), but each serializes
its entries into one file behind one global hash. This spec stores those per-unit
entries as disjoint files, so two PRs that touch different units write different
files and cannot conflict. The merge queue then forms clean speculative stacks
and auto-merge lands them in sequence.

This is a generic spec-spine capability that benefits every adopter, and it is
the committed-but-conflict-free successor to the de-commit interim some adopters
take.

## 2. Territory

This spec changes how the registry and the index are STORED and READ. It does
not change what they MEAN, the validation rules (spec 001/003), or the coupling
gate's contract (spec 005): the gate consumes the same logical `Registry` and
`CodebaseIndex`, now assembled from shards.

**Correction to the original sketch.** The draft scoped the registry out as
"already gitignored." That is false for spec-spine and for any adopter that
commits its registry: `.derived/spec-registry/registry.json` is tracked, carries
a global hash line, and is registered on the spec-020 merge driver exactly like
the index. Sharding the index alone would only half-solve the problem (every spec
PR also rewrites the registry). This spec therefore shards **both** artifacts.

## 3. The shard model

- **Per-spec registry shards.** One file per spec at
  `.derived/spec-registry/by-spec/<id>.json` carrying that spec's `SpecRecord`,
  its corpus-independent ("local") validations, and a `shardHash` over its
  `spec.md` (the registry's only hashed input, matching the pre-shard
  `build.contentHash` input set). A spec PR rewrites only its own shard.
- **Per-spec traceability shards.** One file per spec at
  `.derived/codebase-index/by-spec/<id>.json` carrying that spec's `TraceMapping`
  (implementing paths, resolved units, supersedes/amends edges), its resolver
  diagnostics, and a `shardHash` over its `spec.md`, the source files backing its
  resolved symbol/section/module spans, and the global-inputs scalar (config +
  `extra_hashed_inputs`). A spec PR rewrites only its own shard.
- **Per-package inventory shards.** One file per package at
  `.derived/codebase-index/by-package/<slug>.json` carrying that package's
  `PackageRecord` + a `shardHash` over its manifest (npm governance projection,
  spec 004 §3.5) folded with the global-inputs scalar. A package PR rewrites only
  its own shard. `<slug>` is a filesystem-safe slug of the package name.
- **Derived-on-read global view (NOT committed).** The aggregate `Registry` and
  `CodebaseIndex` are reconstructed from the shard set at read time: the registry
  `validation` (the corpus-wide checks: duplicate id/prefix, dangling edges) and
  both `build.contentHash`es (the fold of the shard hashes), plus the index
  `orphanedSpecs` and `untracedCode`, are pure functions of the shards. Nothing
  global is committed, so there is no shared file to conflict on. The shard
  directory listing is the manifest; no committed root manifest is required.
- **Per-slice sidecar.** The spec 012 `[index.slices]` hashes move from a
  monolithic `index.json` build block to a small `codebase-index/slices.json`
  sidecar, emitted only when slices are configured (a corpus with none commits no
  such file).
- **No single global content-hash file.** Staleness is per-shard. The aggregate
  "is the whole artifact fresh" answer is the conjunction of per-shard checks plus
  a shard-set membership check, computed on read.

## 4. Behavior (MUST / SHOULD)

- **FR-001 (shard emit).** `spec-spine compile` and `spec-spine index` MUST emit
  the shards above, each self-describing its input hash, and MUST prune a removed
  unit's shard (emit is a directory sync). Re-running on unchanged inputs MUST be
  byte-identical per shard.
- **FR-002 (disjoint writes).** A change confined to spec X's authored inputs
  MUST rewrite only `by-spec/<X>.json` (and any package shard whose manifest
  changed). It MUST NOT rewrite other specs' shards. This is the property that
  makes concurrent PRs conflict-free; it is the load-bearing requirement. A change
  to a globally shared input (`spec-spine.toml`, an `extra_hashed_inputs` file) is
  allowed to restamp every index shard: such a change is rare and inherently
  global, and disjoint spec PRs never touch it.
- **FR-003 (per-shard + set staleness).** `spec-spine index check` MUST verify
  each committed shard against its current inputs AND compare the committed shard
  *set* to the current authority set (a spec or package added/removed is a
  membership change), reporting stale shards by id. It replaces the single
  global-hash comparison. The spec 012 `--slice` form is retained against the
  sidecar; a shard is itself the per-unit slice the draft envisioned.
- **FR-004 (read assembles the global view).** `registry list|show|status-report|
  relationships`, `index render|orphans`, the coupling gate's consumers, and the
  dependency-only auto-waiver pre-filter MUST assemble the logical `Registry` /
  `CodebaseIndex` from the shard set (validation / orphans / untraced computed on
  read). The coupling gate contract (spec 005) is unchanged; only the on-disk
  shape it reads changes.
- **FR-005 (conflict-free by construction).** Two changes to disjoint units, when
  merged (locally, via the web button, or via the merge queue's speculative
  build), MUST NOT produce a textual conflict on any committed registry or index
  file, WITHOUT relying on a custom merge driver. The spec 020 driver remains
  useful only for the rare same-shard conflict (two PRs editing the same unit).
- **FR-006 (present-on-clone preserved).** The shards are committed, so both
  artifacts are present on a fresh clone: `/init` and the read-side projections
  read the shard set without a rebuild.
- **FR-007 (MAJOR schema bump).** `REGISTRY_SCHEMA_VERSION` and
  `INDEX_SCHEMA_VERSION` become `1.0.0`. Each shard carries its artifact's schema
  version, and a reader rejects an unknown MAJOR at the shard boundary, so a 0.x
  reader cannot misread a 1.x shard tree. The in-memory aggregate DTOs are
  unchanged (the universal currency consumed by `attest`, the JSON facade, and the
  conformance tests).

## 5. Staleness fidelity (a deliberate, bounded trade)

Per-shard staleness is scoped to each shard's own authored inputs plus the
shard-set membership check. This is equivalent in fidelity to the pre-shard
global hash for every case that hash actually caught (a spec.md edit stales that
spec's shard; an added/removed spec or package is a membership change; a manifest
edit stales that package's shard; a span-shifting source edit stales the owning
spec's shard). A resolution flip caused *purely* by a sibling change that does not
alter a shard's own inputs (for example a `crate` unit that begins resolving only
because the crate appeared in another PR) is caught via package-set membership and
healed by a full `index` re-run; it was never caught by the pre-shard hash either
(which folded only manifests, specs, config, and already-resolved span files).
The disjointness FR-002 buys is worth this bounded scoping.

## 6. Migration

One-time, performed in this change: emit the shard trees, stop emitting the
monolithic `registry.json` / `index.json` (and delete a pre-024 monolithic file
on the next emit), update the schemas (a MAJOR bump), repoint every committed-
artifact reader to the shard assemblers, and move the spec-020 merge-driver
registration to the shard globs. An adopter that took the de-commit interim
re-commits the sharded tree once it adopts this spec-spine version, restoring
present-on-clone in a conflict-free form (tracked in prose as a cross-repo
data-contract dependency, not a spec-registry edge).

## 7. Relationship to the merge queue (complement, not replacement)

Sharding removes the SPURIOUS conflict (the derived artifacts). It does NOT
replace the merge queue, which catches GENUINE "green alone, broken together"
conflicts (the canonical case is two PRs allocating the same spec id, each green
alone, duplicate only together). Sharding is what lets the queue function: today
the artifact conflict prevents the queue from forming a speculative stack at all,
so it ejects constantly. With disjoint shards, the queue forms clean stacks, tests
them in parallel, and merges them in sequence, which is the throughput the queue
exists to deliver.

## 8. Out of scope

- The coupling-gate semantics (spec 005) and the relationship-graph grammar. This
  spec changes storage, not authority derivation.
- The `build-meta.json` wall-clock sidecar (still a single, gitignored file; it is
  not a serialization point because it is not committed).
- The merge queue and branch-protection configuration (adopter ops).

## 9. Acceptance criteria

- **AC-1.** Adding a new spec writes exactly one new `by-spec/<id>.json` in each
  artifact and no change to any other spec's shard.
- **AC-2.** Two branches that each add (or edit) a different spec merge with zero
  textual conflict on any committed registry or index file, with NO merge driver
  registered.
- **AC-3.** `index check` reports a single edited spec's shard as stale when its
  inputs change, reports all other shards fresh, and reports a stale shard set
  when a spec or package is added or removed.
- **AC-4.** `registry` queries, `index render`, and `index orphans` produce output
  equivalent to the pre-shard monolithic artifacts (same logical content),
  assembled from shards.
- **AC-5.** A merge queue stacking N PRs that each touch disjoint specs lands all
  N without ejection on a derived-artifact conflict.
- **AC-6.** A 0.x reader rejects a 1.x shard with a clean schema error (no silent
  misread); the determinism gate proves each shard tree byte-identical across the
  four release triples.
