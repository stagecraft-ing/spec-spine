---
id: "018-constrains-discriminator-optional-unit"
title: "Constrains grammar: classification discriminator and optional unit"
status: draft
kind: "tooling"
created: "2026-06-12"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
  - "004-codebase-index"
extends:
  - spec: "000-spec-spine-bootstrap"
    nature: additive
    paths:
      - "crates/spec-spine-types/src/edges.rs"
      - "crates/spec-spine-types/tests/grammar.rs"
  - spec: "001-compile-registry"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/compile.rs"
      - "crates/spec-spine-core/tests/compile.rs"
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/index.rs"
      - "crates/spec-spine-core/tests/index.rs"
summary: >
  Widens the `constrains` item grammar to the two shapes the predecessor dialect
  authors: a **path-scoped** constraint (`unit:`, the canonical
  `invariant-freeze` over a file or schema) and a **spec-scoped** constraint
  (`target_specs:`, a sequencing/ordering plan over other specs with no code
  unit). Two changes: `unit` becomes optional, and an optional documentary
  discriminator is accepted under either spelling `flavor:` or `kind:` (synonyms,
  not gate-load-bearing). A new V-011 requires every item to scope at least one
  of `unit` / `target_specs`. A spec-scoped item contributes no resolved unit to
  the index. Unblocks the OAP corpus, which authors `{ flavor: invariant-freeze,
  unit }` (spec 130) and `{ kind: <plan>, target_specs }` (specs 078, 089) —
  both rejected by the unit-required, discriminator-less v1 grammar.

---

# 018: constrains discriminator and optional unit

## 1. Purpose

The v1 `ConstrainItem` required a `unit:` and rejected any other field
(`deny_unknown_fields`). The corpus authors two legitimate constraint shapes the
grammar could not express:

- **Path-scoped** — `{ flavor: invariant-freeze, unit: <schema/file> }` (the
  canonical invariant-freeze; OAP spec 130). Has a unit, plus a classification.
- **Spec-scoped** — `{ kind: <plan-name>, target_specs: [...] }` (a delivery /
  sequencing plan asserting an ordering invariant over *other specs*; OAP specs
  078, 089). Has **no** code unit at all.

Both trip the v1 grammar: the discriminator field (`flavor`/`kind`) is unknown,
and the spec-scoped form has no `unit`. The fix accepts both shapes, keeping the
discriminator documentary (the architecture §2.1 already names the constraint
edge "asserts an invariant others must respect" with the canonical
`invariant-freeze` kind — this records *which* invariant without the gate having
to interpret it).

## 2. Territory

The constrains item grammar (`edges.rs`: `unit` → `Option<Unit>`, add `flavor`
and `kind`), the index's constrains→unit projection (`index.rs`: skip an item
with no unit), and one new compile validation (`compile.rs`: V-011). The schema
is permissive on the edge payload, so no schema-file change.

## 3. Behavior

### 3.1 Grammar

```yaml
constrains:
  - { flavor: invariant-freeze, unit: { kind: file, path: "schema.json" } }  # path-scoped
  - { kind: sequencing-plan, target_specs: ["074-x", "075-y"] }              # spec-scoped
```

- `unit` is optional.
- `flavor` and `kind` are **interchangeable synonyms** for the documentary
  classification; both are accepted and preserved verbatim (the dialect uses
  both — 130 writes `flavor`, 078/089 write `kind`). Neither is read by the
  coupling gate. They are not normalized into one another, so `registry show`
  renders the authored spelling.
- `note` and `target_specs` are unchanged from v1.

### 3.2 Index projection

A path-scoped item (with `unit`) contributes a `SourceField::Constrains`
resolved unit, exactly as before. A spec-scoped item (no unit) contributes
**nothing** to `resolved_units` — it claims no code path; its authority is over
the listed specs, which the gate does not derive from `constrains`.

### 3.3 Validation: V-011

A constrains item that declares **neither** `unit` nor `target_specs` asserts an
invariant over nothing — an error-tier **V-011**. This is the floor that keeps
the now-optional `unit` from admitting a content-free item.

### 3.4 Tests (minimum)

- Grammar: a `flavor` + `unit` item and a `kind` + `target_specs` item both
  parse; the discriminator is preserved; `unit` is `None` on the spec-scoped form.
- Compile: V-011 fires on a `{ flavor: … }`-only item; both scoped forms clear
  V-011 (a file-unit need not exist — that is the indexer's I-004, not compile).
- Index: a spec-scoped constrains item yields no resolved unit.

## 4. Out of scope

Gate evaluation of constraints as a distinct failure mode (OAP's exit-code-3
invariant-violation semantics). In this library a path-scoped constraint is an
ordinary owning unit; a separate constraint-evaluation pass is a future spec if
an adopter needs the distinct remediation. Normalizing `flavor`/`kind` to a
single canonical field (kept faithful to authored intent here). Validating that
`target_specs` resolve to existing specs (a lint concern, not a grammar one).
