---
id: "013-declared-extra-frontmatter-passthrough"
title: "Frontmatter: declared extra keys carry arbitrary YAML, verbatim"
status: draft
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
extends:
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/frontmatter.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/registry.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/compile.rs", nature: additive }
  # The declared-aware parse entry must also be used by the indexer's spec
  # discovery (a nested declared value must not knock a spec out of the
  # index); the MINOR bump rides through version.rs, the embedded registry
  # JSON schema and the pinned-versions test; the types export list and the
  # parser/compile test files carry the surface. All additive, same shape as
  # specs 010-012.
  - { spec: "004-codebase-index", unit: "crates/spec-spine-core/src/index.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/lib.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/version.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/schemas/registry.schema.json", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/tests/dtos.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/tests/frontmatter.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/tests/compile.rs", nature: additive }
summary: >
  Widens the value domain of DECLARED extra-frontmatter keys
  (`config.frontmatter.extra_known_keys`) from scalars + string-lists to any
  JSON-representable YAML value (nested maps, mixed lists), carried verbatim
  into SpecRecord.extra_frontmatter under canonical-JSON normalization.
  Undeclared keys keep the existing scalar/string-list restriction and the
  V-007 cap -- the anti-bulk-YAML guard is untouched. Registry schema MINOR.
  This is the load-bearing half of the overlay contract: overlays cannot
  consume domain frontmatter the compiler drops.
---

# 013: Declared extra-frontmatter passthrough

## 1. Purpose

The overlay contract (`docs/overlay-contract.md`) names `extra_known_keys` +
`extra_frontmatter` as the seam by which an adopter's domain frontmatter
reaches its overlay crate without forking the types. But the current value
restriction -- scalars and string-lists only -- makes the seam too narrow for
real overlays: OAP's `compliance:` key (the OWASP control-to-spec mapping its
compliance-report overlay is built on, present on 33 specs) is a nested map
and would be rejected or mangled. A declared key is an explicit adopter
statement of intent; for declared keys the compiler's job is faithful
transport, not shape policing. For **undeclared** keys the restriction and cap
remain exactly as-is -- they are the guard against frontmatter becoming an
unaudited bulk-YAML channel (spec 000's authoring-boundary posture), and this
spec does not soften that by one bit.

## 2. Territory

The frontmatter parser's unknown-key handling (`frontmatter.rs`), the
`extra_frontmatter` value type on `SpecRecord` (`registry.rs`), and the
compile-time validation that polices the declared/undeclared split
(`compile.rs`). All additive on the types crate (floor-owned by 000) and on
001's compile engine.

## 3. Behavior

### 3.1 The declared/undeclared split

| Key is… | Accepted values | Cap | On violation |
|---|---|---|---|
| listed in `extra_known_keys` | any JSON-representable YAML value | exempt from the cap (unchanged) | `V-013` (see §3.3) |
| not listed | scalar or list-of-strings (unchanged) | counts toward the V-007 cap (unchanged) | `V-007` band (unchanged) |

### 3.2 Normalization (determinism)

Declared-key values are converted YAML → JSON once, at parse time:

- Mappings ⇒ JSON objects; **key order is canonical-JSON sorted** on emission
  (authoring order is not preserved -- document this; it is the price of
  byte-identical registries).
- Sequences ⇒ JSON arrays, order preserved.
- Scalars ⇒ JSON string/number/bool/null per YAML core schema resolution.
- The in-memory representation is a `serde_json::Value`; the registry emits it
  through the existing canonical-JSON serializer, so the determinism contract
  (sorted keys, LF, trailing newline) holds with no new machinery.

### 3.3 Unrepresentable values: `V-013`

A declared key whose value cannot be represented as JSON -- a non-string
mapping key, a YAML tag, an anchor/alias cycle -- is an **error-tier**
violation, new code `V-013` ("declared extra-frontmatter key carries an
unrepresentable YAML value"), recorded against the spec like any V-code (the
spec is skipped, compile continues, exit 1 -- 001 §3.1 semantics).
*Implementation note: V-013 is the next free code in the V band at time of
writing (V-001..V-010, V-012 assigned); if the band has moved by landing time,
take the next free code and amend this paragraph in the same PR.*

### 3.4 Schema impact

`registry.json`'s `extra_frontmatter` value type widens from
`string | string[]` to arbitrary JSON. This is a **MINOR** bump per
`docs/schema-versioning.md`: the field was already adopter-shaped and loaders
that read it generically (`serde_json::Value`) are untouched; loaders that
assumed the narrow shape were depending on an undocumented restriction.
Record the widening in the schema-versioning table and the embedded JSON
Schema in `spec-spine-types`.

### 3.5 Tests (minimum)

- A declared nested-map key (model it on a `compliance:`-shaped fixture)
  survives compile → registry round-trip byte-identically across two runs.
- Map-key sorting: two authoring orders, one registry output.
- Undeclared nested map still rejected exactly as pre-013 (guard regression).
- V-013 on a non-string map key under a declared key.
- Cap behavior unchanged for undeclared keys in the presence of declared ones.

## 4. Out of scope

Widening **undeclared** keys (explicitly preserved as-is). Semantic validation
of declared values (shape contracts for domain keys belong to the overlay that
consumes them, per the overlay contract -- the core transports, the overlay
validates). New first-class frontmatter fields (a declared key that earns
core semantics graduates by its own spec, not through this channel).
