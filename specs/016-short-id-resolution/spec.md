---
id: "016-short-id-resolution"
title: "Compile: resolve short spec ids on `depends_on` and `superseded_by`"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
extends:
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/compile.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/tests/compile.rs", nature: additive }
summary: >
  Lets a `depends_on` or `superseded_by` reference name its target by the
  leading spec number alone (`109`) and have the compiler resolve it to the full
  id (`109-slug`) when exactly one spec matches that number, rewriting the
  reference before validation and emission. An exact id, an ambiguous number, or
  a number matching nothing is left unchanged, so V-008 (superseded_by must
  resolve) and V-010 (dangling depends_on) still fire on a genuinely broken
  reference. The resolution policy is the indexer's existing `resolve_id` (spec
  004), applied at compile time so the registry and the index agree. Unblocks
  adopting a corpus that references specs by number (106 specs in the OAP
  dry-run) without a mass frontmatter migration; a no-op on any corpus that
  already uses full ids (such as this repo's own).
---

# 016: resolve short spec ids on `depends_on` and `superseded_by`

## 1. Purpose

The predecessor dialect commonly references a spec by its number alone --
`depends_on: ["109"]`, `superseded_by: "042"` -- rather than its full
`NNN-slug` id. This library's compiler matches a reference against the literal
id set, so a numeric short form resolves to nothing: a `depends_on` short id
raises V-010 (dangling, warning) and a `superseded_by` short id raises V-008
(does not resolve, error), the latter failing the corpus. The Phase-0 dry-run
found 106 specs using the short form.

The id is unambiguous: V-004 already forbids two specs sharing a numeric
prefix, so a number identifies at most one spec. Resolving it is therefore a
deterministic convenience, not a semantic guess. The indexer already does
exactly this (`index.rs::resolve_id`, spec 004) when it resolves `amends`
references for the trace graph; this spec applies the same policy at compile
time, on the two fields whose short ids currently break validation, so the
emitted registry carries full ids and the two gates stop flagging a reference
that is merely abbreviated.

## 2. Territory

The compile resolution pass and its helper (`compile.rs`), plus tests
(`tests/compile.rs`). The resolution function is a local mirror of
`index.rs::resolve_id` rather than a shared call, to keep the compile gate (001)
from taking a code dependency on the indexer's file (004); the two are pinned
equal by behavior and a citing comment, not by linkage. No change to the
registry schema, the JSON Schema artifact, the indexer, lint, or couple.
Additive.

## 3. Behavior

### 3.1 Resolution policy

For a reference string `r` against the set of all compiled spec ids:

1. If `r` is exactly an existing id, keep it.
2. Otherwise, collect every id whose leading dash-segment (the text before the
   first `-`) equals `r`. If exactly one matches, resolve `r` to it.
3. Otherwise (no match, or -- only possible under an existing V-004 violation --
   more than one), keep `r` unchanged.

This is `index.rs::resolve_id` verbatim. Note it matches the **whole** leading
segment, not an arbitrary character prefix: `109` resolves `109-foo`, but `10`
resolves nothing, and a wrong full slug (`109-typo`) resolves nothing rather
than silently snapping to `109-foo`.

### 3.2 Scope: which references resolve

Only `depends_on` (each entry) and `superseded_by`. These are the two fields a
short id currently breaks (V-010, V-008). `supersedes` and `amends` are out of
scope here: they raise no compile V-code on a short id today, and the indexer
already resolves `amends` for its own graph (spec 004). Edge `unit:` targets are
paths, not spec ids, and are untouched.

### 3.3 Ordering, determinism, and the canonical no-op

Resolution runs once, after the full id set is known and before per-spec
validation and record construction, so both the V-codes and the emitted records
see the resolved value. It is a pure function of the id set, so the registry
stays deterministic. A reference that is already a full id is returned
unchanged, so a corpus that uses full ids throughout -- including this
repository's own -- compiles to a byte-identical registry (this spec adds its
own record and nothing else).

### 3.4 Tests (minimum)

- A short `depends_on` resolves to the full id, the record carries the resolved
  id, and no V-010 fires.
- A short `superseded_by` on a superseded spec resolves, clearing V-008, with
  the full id in the record.
- A reference matching no spec is left unchanged and still raises its V-code.

## 4. Out of scope

Resolution of `supersedes`/`amends` (no compile V-code today; `amends` already
resolved at index time -- file a follow-up only if an adopter shows a need).
Arbitrary-prefix or fuzzy matching (the policy is exact-segment, single-match,
by design). Cross-field reference rewriting in the index beyond what spec 004
already does. Any registry schema change. The structured `supersedes` form
(`{ spec, scope: partial, unit }`) surfaced by the same dry-run: that is
partial-supersession semantics, taken to design separately, not an id-spelling
concern.
