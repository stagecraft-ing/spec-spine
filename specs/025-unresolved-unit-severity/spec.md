---
id: "025-unresolved-unit-severity"
title: "Severity tiers for unresolved units: lifecycle- and edge-aware (warn, count, never skip)"
status: draft
kind: "tooling"
created: "2026-06-17"
owner: "The spec-spine Authors"
implementation: pending
risk: medium
depends_on:
  - "001-compile-registry"   # the registry relationship graph: the authority source
  - "004-codebase-index"     # the resolver whose diagnostic severity this amends
  - "005-coupling-gate"      # consumes registry authority, never the index mapping (the boundary)
amends:
  - "004-codebase-index"     # patches 004's "unresolved unit -> hard I-error" contract in place
extends:
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/index.rs"      # the resolver: severity decision + W-band
      - "crates/spec-spine-core/tests/index.rs"    # severity-tier acceptance fixtures
      - "crates/spec-spine-types/src/version.rs"   # INDEX_SCHEMA_VERSION minor bump
      - "crates/spec-spine-types/tests/dtos.rs"    # the schema-version pin test for the bump
references:
  - { unit: { kind: file, path: "docs/design/00-architecture.md" }, role: context }
summary: >
  Today the indexer hard-errors (I-003..I-009) on every declared unit that does
  not resolve, regardless of whether the edge is authoritative or whether the
  owning spec is even built yet. Two of those errors are not governance failures:
  an unresolved unit on a non-owning `references` edge is a stale provenance
  pointer, and an unresolved owning unit on a `draft` / `implementation: pending`
  spec is legitimate work-in-progress. This spec adds two orthogonal severity
  tiers over the same resolver, keyed on (a) edge authority and (b) owning-spec
  lifecycle, so those two cases become counted warnings (a fresh `W-001` /
  `W-002` band) instead of blocking errors, while an unresolved owning unit on an
  approved + implemented spec stays a hard error. The downgrade never skips:
  every such unit is still surfaced and counted, never silently dropped, so the
  authority-laundering / invisible-drift vector stays closed. The index never
  fabricates a code location for an unresolved path; authority remains a property
  of the registry relationship graph, which the coupling gate consults
  independently of index resolution. Additive: the `warnings` tier and the
  free-form diagnostic `code` already exist, so this is an `INDEX_SCHEMA_VERSION`
  MINOR (1.0.0 -> 1.1.0) with no schema-file edit and no registry change.
---

# 025: Severity tiers for unresolved units

Filed off the OAP spec-217 Phase-0 dry run of the published 0.5.0 library
against its 220-spec corpus. `index` emitted 116 error diagnostics; after the
adopter's own config and corpus cleanup, a residual class was the library's, not
the adopter's: the resolver treats every unresolved unit as a hard error with no
regard for the edge's authority or the owning spec's lifecycle. This spec settles
that severity model. It pairs with a separate defects spec for the mechanical
resolution bugs (region-marker dispatch, Makefile tag handling, nested-workspace
glob base); those are out of scope here (§6).

## 1. Purpose

The indexer's resolver (`index.rs::resolve_unit`) emits one of `I-003`..`I-009`
whenever a declared unit fails to resolve against the working tree, and routes it
to the blocking `errors` tier (`BLOCKING_CODES`). That single severity is too
blunt along two independent axes:

- **Authority.** spec-spine's model designates `references` as the one
  **non-owning** edge: the coupling gate already ignores it for ownership
  (`CLAUDE.md`; `contract.md` §"Typed edges"). Yet an unresolved `references`
  unit fails `index` exactly as hard as an unresolved `establishes` unit. A
  dangling provenance pointer is not a governance failure, because no authority
  ever flows through a non-owning edge. Hard-erroring on it taxes every adopter's
  provenance hygiene with no governance payoff and causes ongoing link rot as
  referenced docs move.

- **Lifecycle.** A spec that is `status: draft` or `implementation: pending` may
  legitimately declare owning units that do not resolve yet: the code is in
  flight. The previous in-tree OAP indexer silently *tolerated* (effectively
  skipped) such units; the current library hard-errors them. Both are wrong.
  Skipping is an invisible-drift vector (a perpetual-draft spec could
  `establishes:` a path whose units are never validated and whose claim a gate
  never sees, or point at attacker-staged files with nothing flagging it).
  Hard-erroring blocks legitimate in-progress work.

The correct rule on both axes is the same: **surface and count the unit, never
skip it, and let severity follow authority and lifecycle.** This spec encodes
that as two warning tiers over the existing resolver, leaving the
approved-and-implemented owning case a hard error.

These two axes are orthogonal and compose: the edge-authority tier (A) covers
provenance pointers regardless of lifecycle; the lifecycle tier (P2) covers
in-flight owning claims. Neither subsumes the other (most OAP errors are
`references` edges on approved + complete specs, which only the edge-authority
tier reaches).

## 2. Territory

This spec amends 004's resolver-diagnostic contract in place and additively
claims, alongside 004, the files it must touch:

- `crates/spec-spine-core/src/index.rs`: the severity decision at the
  `resolve_unit` call sites, and a fresh `W-001` / `W-002` warning band. The
  resolver already threads an `owning: bool` per unit (the third element of the
  gathered `(SourceField, Unit, bool)` tuple) and a per-spec `status`; this spec
  adds the owning spec's `implementation` to that same `SpecInfo` so the decision
  has both lifecycle signals at hand.
- `crates/spec-spine-core/tests/index.rs`: the acceptance fixtures (§5).
- `crates/spec-spine-types/src/version.rs`: the `INDEX_SCHEMA_VERSION` MINOR bump
  (§3.6).

It touches **nothing** in the registry (`compile`) or the coupling gate
(`couple`). The `warnings` tier (`Diagnostics::warnings`) and the free-form
`Diagnostic::code` string already exist, so the new W-band needs no DTO or
schema-file change. The change is recorded as the `INDEX_SCHEMA_VERSION` minor
bump alone, mirroring spec 017.

## 3. Behavior

### 3.1 The severity decision

For each declared unit that fails to resolve, the resolver MUST classify the
diagnostic by this precedence (the first matching arm wins):

1. **Non-owning edge** (the unit's source edge is `references`): emit `W-002`
   (warning). Lifecycle is irrelevant here, because a non-owning edge carries no
   authority in any lifecycle state.
2. **Owning edge on an in-flight spec** (owning spec `status: draft` OR
   `implementation: pending`): emit `W-001` (warning).
3. **Owning edge on a settled spec** (owning spec `approved` AND not
   `implementation: pending`): emit the existing `I-003`..`I-009` error
   (unchanged).

| edge | owning spec lifecycle | unresolved -> |
|---|---|---|
| `references` (non-owning) | any | **W-002** (warning) |
| owning | `draft` or `implementation: pending` | **W-001** (warning) |
| owning | `approved` + implemented | `I-003`..`I-009` (error, unchanged) |

A resolved unit is unaffected on every axis: severity tiers govern only the
failure path.

### 3.2 The two warning codes

- **`W-001` (unresolved unit on an in-flight spec).** An owning unit declared by
  a `draft` / `implementation: pending` spec did not resolve. The message MUST
  carry the unit kind, the path/id, and the underlying reason so the warning is
  as actionable as the error it replaces (e.g. "draft spec '023' file unit
  'src/x.rs' does not exist").
- **`W-002` (unresolved non-owning reference).** A `references` unit did not
  resolve. The message MUST identify the dangling target.

Both codes are new and live in the index `W-` band. They MUST NOT be added to
`BLOCKING_CODES`, so they never fail `index check` or any blocking-code gate.

### 3.3 Surface, never skip

Every downgraded diagnostic MUST be emitted into the owning spec's shard
`diagnostics.warnings` and counted. The unit MUST NOT be silently dropped from
the run. This is the guarantee that keeps a draft spec's unbuilt claim and a
stale reference both auditable: a consumer can enumerate and accept them
explicitly, but can never miss them.

### 3.4 No fabricated mapping (the index's boundary)

An unresolved unit MUST NOT contribute a `ResolvedLocation` / `TraceMapping`
entry. There is no code at an unresolved path, so a mapping entry would assert a
code-to-spec binding that does not exist: misleading, not protective. The
index's obligation under this spec is the warning (surface + count), nothing
more.

### 3.5 Authority is a registry property (the consumer boundary)

This spec changes only the index (code-as-source) view. It makes no change to the
registry or the coupling gate, and it is **not** the mechanism by which an
in-flight or referenced claim retains authority. Authority over a path `P`
derives from the declared `establishes:` / `extends:` (owning) edges in the
**compiled registry** relationship graph, which exist whether or not `P` is on
disk and whether or not the index ever mapped it. The coupling gate consults that
graph directly. So a draft spec's owning claim on `P` is already live to the gate
the moment code lands at `P`, independent of this spec.

> **Binding consumer note.** The safety of the edge-authority downgrade (§3.1
> arm 1) holds **only** while consumers derive "who owns `P`" from the registry
> relationship graph, not from the index mapping. A consumer that derived
> ownership from the index mapping would inherit a blind spot for unresolved
> units (which carry no mapping entry, §3.4). Authority queries MUST route
> through the registry.

### 3.6 Schema

The change is additive (new warning codes in an existing tier; no new field, no
shape change). `INDEX_SCHEMA_VERSION` MUST bump MINOR: `1.0.0` -> `1.1.0`. The
on-disk MAJOR is unchanged (`1`), so existing readers accept the index. No
`index.schema.json` edit is required (the schema is permissive on the diagnostic
`code` and already admits the `warnings` array). The conformance test
(`core/tests/conformance.rs`) MUST pass against the embedded `1.1.0` schema.

## 4. Functional requirements

- **FR-001 (edge-authority tier).** An unresolved unit from a non-owning edge
  (`references`) MUST be emitted as `W-002` in the `warnings` tier, never as a
  blocking error, on any owning-spec lifecycle.
- **FR-002 (lifecycle tier).** An unresolved unit from an owning edge whose
  owning spec is `status: draft` OR `implementation: pending` MUST be emitted as
  `W-001` in the `warnings` tier, never as a blocking error.
- **FR-003 (strictness preserved).** An unresolved unit from an owning edge whose
  owning spec is `approved` AND not `implementation: pending` MUST remain a hard
  `I-003`..`I-009` error, routed to the `errors` tier, exactly as before this
  spec.
- **FR-004 (surface, never skip).** Every unit downgraded under FR-001 / FR-002
  MUST appear and be counted in the owning spec's `diagnostics.warnings`; it MUST
  NOT be silently dropped. `W-001` / `W-002` MUST NOT be members of
  `BLOCKING_CODES`.
- **FR-005 (no fabricated mapping).** A unit that does not resolve MUST NOT
  produce a `ResolvedLocation` or `TraceMapping` entry, regardless of severity
  tier.
- **FR-006 (registry/gate untouched).** This spec MUST NOT change the registry
  (`compile`) output or the coupling gate (`couple`) behavior. Authority remains
  a registry-relationship-graph property (§3.5).
- **FR-007 (schema).** `INDEX_SCHEMA_VERSION` MUST bump `1.0.0` -> `1.1.0`
  (additive MINOR), with no schema-file edit; the conformance test MUST pass.

## 5. Acceptance criteria

- **AC-1 (references warn).** A `references` unit pointing at a missing
  file/section on an `approved` + `implementation: complete` spec yields exactly
  one `W-002`, zero errors; `index check` exits `0`.
- **AC-2 (draft owning warn).** An `establishes` unit pointing at a missing path
  on a `status: draft` spec yields one `W-001`, zero errors.
- **AC-3 (pending owning warn).** The same on an `approved` spec with
  `implementation: pending` yields one `W-001`, zero errors.
- **AC-4 (settled owning still errors).** The same owning unit on an `approved` +
  `implementation: complete` spec yields the existing `I-004` (or kind-matched
  `I-0xx`) error, in the `errors` tier, unchanged.
- **AC-5 (edge-type precedence).** A `references` unit on a `draft` spec yields
  `W-002` (arm 1), not `W-001`.
- **AC-6 (self-corpus determinism).** spec-spine's own corpus (all specs
  `approved` + `complete`, all `references` resolving) produces zero `W-001` /
  `W-002`; no committed shard gains a `warnings` block; the only committed-shard
  delta from this spec is the `schemaVersion` field restamp to `1.1.0`.
- **AC-7 (conformance).** `core/tests/conformance.rs` is green against the
  embedded `1.1.0` index schema.

## 6. Out of scope

- **The three indexer resolution defects** (non-workflow YAML `# region:`
  dispatch; Makefile `## tag:` silent overwrite; nested-workspace glob base
  resolution). They are mechanical correctness fixes, not a severity policy, and
  are filed as a separate spec amending their real owners (sections work amends
  spec 022; discovery work amends spec 004).
- **An opt-in fail-on-warning strictness knob.** This spec sets `W-001` /
  `W-002` as non-blocking by default (§3.3). A strict adopter, or spec-spine on
  its own corpus, MAY later wire a policy that fails on the `W-` band; that
  policy is not defined here. (Making `references` a hard error by default is the
  *rejected* alternative, decided against in §1 because it taxes provenance
  hygiene with no governance payoff, not a deferred option.)
- **Any registry or coupling-gate change** (§3.5, FR-006).
- **A downstream consumer's gate policy** (e.g. "zero errors plus zero
  un-accepted warnings"). How a consumer accepts the enumerated warnings is its
  own concern; this spec only guarantees the warnings are surfaced and counted.

## 7. Security rationale

Two properties make the downgrade safe rather than a drift-hiding hole:

- **Warn, never skip, keeps the unit auditable (FR-004).** A silently skipped
  unit is the actual laundering vector: a perpetual-draft spec could claim a path
  whose units are never validated, or point at not-yet-existing / attacker-staged
  files, and nothing would flag it. A counted `W-001` / `W-002` keeps every such
  declaration visible and enumerable, so a consumer accepts it deliberately or
  not at all.
- **Downgrading `references` cannot launder authority (FR-001).** Authority never
  flows through a non-owning edge: you cannot smuggle an `establishes`-grade
  claim into a `references` edge to dodge the error, because `references` confers
  no ownership in the registry graph the gate consults. The owning-edge severity
  is untouched except by lifecycle (FR-002 / FR-003), which itself never skips.
- **The boundary holds only with registry-sourced authority (§3.5).** The above
  is sound precisely because authority is a registry property, not an index
  property. The binding consumer note records the one way to break it (deriving
  ownership from the index mapping) so no future consumer does.
