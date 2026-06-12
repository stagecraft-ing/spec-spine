---
id: "014-edge-paths-grammar-sugar"
title: "Edge grammar: `paths:` list sugar on extends/refines items"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
extends:
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/edges.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/compile.rs", nature: additive }
  # The expansion runs in the shared parse path (parse_frontmatter_with), so
  # compile, index, lint and couple all see only single-unit edges; the
  # grammar and compile test files carry the coverage.
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/frontmatter.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/tests/grammar.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/tests/compile.rs", nature: additive }
summary: >
  Authoring sugar for the predecessor dialect: an `extends:` or `refines:`
  item may declare `paths: [<file>, ...]` instead of a single `unit:`; the
  compiler expands each path to a file unit at parse time, emitting N
  normalized single-unit edges into the registry. Consumers see exactly one
  edge shape -- `paths:` never reaches registry.json. `unit:` and `paths:` on
  the same item is malformed (V-002). Unblocks adopting a corpus authored in
  the OAP dialect (140+ extends-bearing specs) without a mass frontmatter
  migration.
---

# 014: `paths:` list sugar on extends/refines items

## 1. Purpose

This library's grammar descends from OAP's, but tightened one degree: an
`extends`/`refines` item carries a single `unit:`. The ancestor dialect lets
one item claim several files at once (`extends: [{spec: X, paths: [a, b]}]`,
`refines: [{aspect: A, paths: [a, b]}]`), and OAP's corpus uses that form
across 140+ extends-bearing and 60+ refines-bearing specs. Requiring a
mass-migration commit in the adopter's corpus as the price of adoption is
backwards -- the corpus is the adopter's truth; the compiler should meet it.
The fix is sugar, not semantics: `paths:` is an authoring convenience that the
compiler normalizes away, so every downstream consumer (query, index, couple,
overlays) continues to see exactly one edge shape.

## 2. Territory

The edge item grammar (`edges.rs`: accept the alternative field and expand
it) wired into the shared parse entry (`frontmatter.rs`), so the V-002
mapping in `compile.rs` and every downstream consumer (index, lint, couple)
see only single-unit edges with no changes of their own. Additive; no change
to `establishes` (whose bare-string list form already covers the multi-path
case), no change to registry consumers.

## 3. Behavior

### 3.1 Accepted forms

On an `extends:` item -- exactly one of:

```yaml
extends:
  - { spec: "001-x", unit: "src/lib.rs", nature: additive }            # existing
  - { spec: "001-x", unit: { kind: section, file: "Makefile", anchor: "ci" } }
  - { spec: "001-x", paths: ["src/lib.rs", "src/api/"], nature: additive }  # new
```

On a `refines:` item, symmetrically: `paths:` as the alternative to `unit:`,
with `aspect` (and `refines_specs`) carried unchanged.

- `paths:` MUST be a non-empty list of strings; each string uses **file-unit
  semantics only** (literal path; trailing `/` = directory subtree, per 004
  §3.3). Section/symbol units have no plural form -- a multi-section claim is
  written as multiple items.
- An item with **both** `unit:` and `paths:`, or with an empty `paths:` list,
  is malformed frontmatter: `V-002`, error tier, spec skipped (001 §3.1).

### 3.2 Normalization (the sugar never escapes)

At parse time each `paths:` item expands to N items, one per path, each with
`unit: { kind: file, path: <p> }` and every other field (`spec`, `nature`,
`aspect`, `refines_specs`) copied. Expansion preserves authored path order;
the registry's existing edge sorting then applies. `registry.json` therefore
contains **only** single-unit edges -- its schema, the JSON Schema artifact,
and every consumer are untouched (no schema bump; verify the embedded schema
does not need a text edit precisely because the wire shape is unchanged).

Lint (`L-001`, `L-004`) and the index/coupling pipeline run post-expansion and
need no changes; tests assert that, they don't re-implement it.

### 3.3 Equivalence guarantee

A corpus authored with `paths: [a, b]` MUST compile to a registry
byte-identical to the same corpus authored as two single-`unit` items in the
same order. This is the whole contract; the golden test for it is the
acceptance test.

### 3.4 Tests (minimum)

- The byte-equivalence golden of §3.3, for both `extends` and `refines`.
- Directory-form path inside `paths:`.
- `V-002` on `unit` + `paths` together; on empty `paths`; on a non-string
  entry.
- An OAP-dialect fixture spec (modeled on a real shape: `refines` with
  `aspect` + two paths) compiles clean.

## 4. Out of scope

Plural forms for `section`/`symbol` units (write multiple items). `paths:` on
`co_authority` or `constrains` (their items are unit-singular by design;
extend only if the Phase-0 corpus dry-run proves an adopter needs it -- file a
follow-up, don't widen here). Any registry schema change. Emission of a
derived `implements:` union view (an adopter-side migration concern, noted in
the amendments README).
