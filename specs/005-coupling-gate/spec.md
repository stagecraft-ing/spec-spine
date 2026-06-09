---
id: "005-coupling-gate"
title: "The coupling gate — join the two views and refuse drift"
status: approved
kind: "tooling"
created: "2026-06-09"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
  - "002-registry-query"
  - "004-codebase-index"
establishes:
  - "crates/spec-spine-core/src/couple.rs"
  - "crates/spec-spine-cli/src/cmd_couple.rs"
  - "crates/spec-spine-core/tests/couple.rs"
  - "crates/spec-spine-cli/tests/couple.rs"
extends:
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/src/main.rs", nature: additive }
summary: >
  The PR-time coupling gate (spec-spine.md guardrail 2): cross-reference every
  changed code path against the authority graph and refuse the merge when code
  drifts from the spec that owns it. A pure function of (config, registry, index,
  diff, waiver); the CLI parses `git diff --no-color -U0 base...head` into a typed
  DiffInput so git stays out of core. Matches a diff hunk to its owning unit at
  file, section, and symbol granularity; is amends-aware with the FR-005
  strict-expansion guard; transfers authority across supersedes; honors PR-body
  waivers and an additive bypass list. Establishes the couple engine and the
  `spec-spine couple` subcommand.
---

# 005 — The coupling gate

## 1. Purpose

The registry (spec 001) is the spec-as-source view; the index (spec 004) is the
code-as-source view. They are inverses. This gate **joins them at PR time** and
refuses the merge when a changed code path is not accompanied by an edit to a
spec that has authority over it (spec-spine.md guardrail 2). It is the mechanical
enforcement that catches silent drift before it lands.

The behavioral semantics are ported **intact** from OAP
`tools/spec-spine/spec-code-coupling-check` (`lib.rs:legitimate_owners`,
`is_bypass_against`, `claim_matches`, `parse_waiver`, `span_overlaps_hunk`,
`build_unit_claim_index`) — the single most battle-tested algorithm in the
references. The structure around it is fresh; the behavior is re-derived, not
reinvented.

## 2. Territory

`spec-spine-core`'s `couple.rs` (the pure engine + the `DiffInput` / `Waiver` /
`CoupleReport` DTOs), the `spec-spine couple` CLI subcommand (`cmd_couple.rs`,
which owns the `git diff` invocation and PR-body read), and their tests. The gate
additively extends the core library surface (`lib.rs`) and the CLI dispatch
(`main.rs`).

## 3. Behavior

### 3.1 The boundary — git stays out of core

Core exposes two pure entry points (no clock, no env, no git):

- `couple(cfg, repo_root, diff, waiver)` — the freshness-guarded form: it checks
  the committed index is fresh, loads `registry.json` + `index.json` from
  `derived_dir`, and delegates to `couple_with`.
- `couple_with(cfg, registry, index, diff, waiver)` — the pure form for callers
  that already hold the artifacts (overlays, tests).

The CLI runs `git diff --no-color -U0 <base>...<head>`, parses it into a typed
`DiffInput` (per-file new-side hunk line-spans), reads the PR body for a waiver,
and calls `couple`. A `DiffInput` file with **no hunks** (a deletion, or
`--paths-from` mode) denotes a whole-file change that overlaps every span.

### 3.2 Owner derivation — three granularities

The *legitimate owners* of a changed `(path, hunk)` are the union of two sources,
joined under the primary-owner heuristic (any one owner clears):

1. **Unit-resolved ownership** (span-aware) — for every ownership-bearing
   `resolvedUnit` in the index whose location file matches `path` and whose span
   overlaps the hunk:
   - **file** unit → `span` is absent ⇒ whole file ⇒ overlaps every hunk. A
     trailing-slash directory path prefix-matches every file beneath it.
   - **section** unit → the anchor's resolved line-span; overlaps iff the hunk's
     line range intersects it.
   - **symbol** unit → the symbol's resolved line-span; same overlap rule.
2. **Path-level ownership** (whole-file) — for every `implementingPath` whose
   claim matches `path` (exact, or directory prefix for a package directory).
   This carries the manifest-metadata and comment-header linkages.

Span overlap uses inclusive 1-based ranges aligned with `git diff -U0`
(`span_overlaps_hunk`); a `None` span (whole file) overlaps unconditionally.

### 3.3 Amends-awareness and the strict-expansion guard (FR-005)

When the changed path is exactly `specs/<id>/spec.md`, the owner set is
**expanded** to include every spec that `amends` `<id>` (and the amended spec's
`amendment_record` target, when present) — **but only if the base owner set is
non-empty**. This is the FR-005 strict-expansion guard: amends may add owners to
an already-firing path; it must never silently enrol a path that has no owner
today, or editing your own spec while an unrelated amender exists would newly
fire. The set strictly expands; it never shrinks.

In practice a `specs/<id>/spec.md` path has no base owner (no spec claims another
spec's source of truth), so it is skipped as unclaimed — editing a spec is always
permitted. The guard is what makes that safe.

### 3.4 Supersedes — authority transfer

`supersedes` transfers current authority to the superseding spec: if spec `S`
supersedes spec `P`, then `S` joins the legitimate owners of every path/unit `P`
owns (derived from the registry's `supersedes` edges joined to the index's
resolved ownership). The transfer is **additive** — `S` is added, `P` is not
removed — preserving the "strictly expands, never removes" contract, so editing
either the predecessor or the successor clears the path.

### 3.5 Clearance, waivers, and bypass

- **Clearance**: a path is cleared when **any one** of its legitimate owners has
  its `specs/<owner-id>/spec.md` in the diff. An owned path with no owner edit is
  a **drift violation**.
- **Bypass**: a path exempt from the gate is skipped. The match rules
  (`is_bypass_against`): trailing `/` ⇒ directory prefix; leading `**/` ⇒
  tail-suffix anywhere in the tree; else exact file. The effective bypass set is
  a **hardcoded generic floor** unioned with `config.coupling.bypass_prefixes` —
  the adopter list is **additive and cannot remove a floor entry**.
- **Waiver**: the gate reads the first PR-body line beginning with
  `config.coupling.waiver_keyword` (default `Spec-Drift-Waiver:`); the trimmed
  remainder is the reason. A present waiver suppresses the failure exit but the
  violations are **retained** in the report for review-time visibility.

### 3.6 Granularity reconciliation (the crate-spec vs file-establishes question)

Two ownership granularities coexist coherently:

- A crate's `[package.metadata.<ns>].spec` makes that spec the **whole-crate
  coverage floor** — every file under the package directory has at least that
  spec as an owner, so no code can be added to a governed crate with zero
  authority.
- Per-file `establishes` / per-unit `extends`/`refines`/`co_authority` **add**
  finer-grained owners on top of the floor.

Because any one owner clears and the owner set only ever expands, the natural
edit (touch the spec that established the file) always clears, while the floor
guarantees coverage for new or unclaimed files. The two are never in conflict.

### 3.7 Exit codes

| Result | Exit |
|---|---|
| no drift, or all drift waived | `0` |
| drift (uncovered changed paths), no waiver | `1` |
| committed index is stale (recompute the index first) | `2` |
| IO / parse / load failure | `3` |

`couple` / `couple_with` return `Ok(CoupleReport)` for any completed analysis
(clean, drift, or waived); the CLI maps the report to the exit code. Stale and
IO/parse/load are `Err` (exit 2 / 3). This keeps the JSON facade
(`couple_json`) returning the structured report even on drift.

### 3.8 Determinism

`couple_with` is a pure function of its inputs. Violations are sorted by path;
owner sets are `BTreeSet`-ordered. No clock, env, or git in core.

## 4. Out of scope

The indexer and unit resolution (spec 004). The `init` scaffolder (spec 006).
Source-line staleness beyond the manifest/spec/config content-hash (the gate
relies on a freshly recomputed index in CI; the freshness guard catches
spec/manifest/config drift only — see spec 004 §3.5).
