---
id: "015-establishes-wrapper-na-alias"
title: "Frontmatter sugar: `unit:`-wrapped establishes items and the `n/a` implementation alias"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "000-spec-spine-bootstrap"
extends:
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/unit.rs", nature: additive }
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/frontmatter.rs", nature: additive }
  # Both behaviors live in the shared parse layer (the Unit deserializer and the
  # Implementation enum), so compile, index, lint and couple see only canonical
  # units and the canonical `n-a` spelling; the grammar and compile test files
  # carry the coverage.
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/tests/grammar.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/tests/compile.rs", nature: additive }
summary: >
  Two pieces of authoring sugar for the predecessor dialect, both normalized
  away at parse so nothing new reaches registry.json. (1) An authority unit may
  be written as a single-key `{ unit: <unit> }` wrapper -- a third 1:1
  representation alongside the existing bare-string and tagged-map forms --
  which the Unit deserializer unwraps to its inner unit. The corpus uses this on
  `establishes` items (one wrapper per item, `unit` the only field). (2)
  `implementation: n/a` is accepted as a deserialize-only alias for the
  canonical `n-a`, which stays the only spelling ever emitted. Neither changes
  the registry schema or any consumer; both unblock adopting a corpus authored
  in the OAP dialect (389 wrapped establishes items across ~97 specs; 5 specs
  using `n/a`) without a mass frontmatter migration.
---

# 015: `unit:`-wrapped establishes items and the `n/a` implementation alias

## 1. Purpose

This library's grammar descends from OAP's. The Phase-0 corpus dry-run surfaced
two spellings the ancestor dialect uses that this grammar did not yet accept,
both of which are pure surface convenience carrying no information the canonical
form lacks:

1. **The `unit:` wrapper.** Where this library writes an `establishes` item as a
   bare unit (`establishes: ["src/lib.rs"]` or a tagged `{ kind, ... }` map), the
   ancestor wraps each item in a single-key map: `establishes: [{ unit: "src/lib.rs" }]`.
   The dry-run found this on 389 items across ~97 specs, and in every one `unit`
   is the only key -- the wrapper is redundant punctuation.
2. **The `n/a` alias.** This library spells "not applicable" `n-a` (the
   kebab-case family its other `implementation` values use); the ancestor writes
   `n/a`. The dry-run found 5 specs using the slash form.

Requiring a mass-migration commit in the adopter's corpus as the price of
adoption is backwards: the corpus is the adopter's truth and the compiler should
meet it where the difference is spelling, not meaning. The fix is sugar, not
semantics -- both forms normalize to the canonical representation at parse time,
so every downstream consumer (query, index, couple, overlays) continues to see
exactly one unit shape and one `implementation` spelling.

## 2. Territory

The Unit deserializer (`unit.rs`: accept the wrapper as a third representation)
and the `Implementation` enum (`frontmatter.rs`: a serde alias on the `n-a`
variant), both in the shared parse entry the whole pipeline funnels through. No
change to `compile.rs`, `index`, `lint`, `couple`, the registry schema, or the
JSON Schema artifact. Additive.

The wrapper is defined at the unit level, so it is uniformly accepted wherever a
`Unit` is parsed (it would also unwrap on a `co_authority`/`constrains`/
`references` item's `unit:` value), not special-cased to `establishes`. This is
the simplest correct implementation -- one 1:1 representation rule rather than an
edge-specific carve-out -- and is harmless: the wrapper is only ever authored on
`establishes`, and on any edge it still normalizes to the same single unit.

## 3. Behavior

### 3.1 The `unit:` wrapper (a third 1:1 unit representation)

A `Unit` already deserializes from either a bare string (= a file unit) or a
tagged `{ kind, ... }` map (000 §4.2). This adds a third accepted form:

```yaml
establishes:
  - "src/lib.rs"                                   # existing: bare string
  - { kind: symbol, id: "crate::run" }             # existing: tagged map
  - { unit: "src/lib.rs" }                         # new: wrapper over a bare string
  - { unit: { kind: section, file: "Makefile", anchor: "ci" } }  # new: wrapper over a tagged map
```

- The wrapper is a map whose single key is `unit:`; its value is itself a unit
  in any accepted form (bare or tagged). The deserializer recurses through the
  same impl, so the inner unit inherits its own validation -- e.g. an empty
  wrapped path (`{ unit: "" }`) is rejected exactly as a bare `""` is.
- The wrapper is 1:1 (one wrapper -> one unit), unlike 014's `paths:` list
  (1:N), which is why it lives in the `Unit` deserializer beside the bare/tagged
  forms rather than as a frontmatter normalize step.

### 3.2 The `n/a` implementation alias

`implementation: n/a` is accepted as a deserialize-only alias for the
`Implementation::Na` variant whose canonical spelling is `n-a`. Both spellings
parse to the same value; `n-a` remains the only spelling ever serialized into
`registry.json`. No other `implementation` value gains an alias.

### 3.3 Normalization and equivalence (the sugar never escapes)

Both behaviors resolve entirely inside the parse layer. A `{ unit: X }` wrapper
becomes the unit `X`; `n/a` becomes `Na`. `registry.json` therefore contains
**only** bare/tagged units (never a wrapper) and **only** `n-a` (never `n/a`) --
its schema, the JSON Schema artifact, and every consumer are untouched (no
schema bump; the wire shape is unchanged precisely because the sugar normalizes
away before emission).

The contract: a corpus authored in the predecessor dialect (every `establishes`
item `{ unit: ... }`-wrapped, `implementation: n/a`) MUST compile to a registry
byte-identical to the same corpus authored canonically (bare/tagged units,
`implementation: n-a`), modulo `build.contentHash` (which hashes the authored
spec bytes, and those differ by construction). This byte-equivalence golden is
the acceptance test.

### 3.4 Tests (minimum)

- The §3.3 byte-equivalence golden: a wrapped + `n/a` corpus compiles to a
  registry byte-identical to the canonical spelling.
- The wrapper unwraps over both a bare-string and a tagged inner unit, mixed
  within one `establishes` list with un-wrapped items.
- A wrapped empty path (`{ unit: "" }`) is still rejected.
- `implementation: n/a` and `n-a` parse to the same value, and `Na` serializes
  back as `n-a`.

## 4. Out of scope

Aliases for any other `implementation` value, or for `status`/`risk` (the
dry-run surfaced only `n/a`; widen only on evidence). A wrapper form for the
edge item structs themselves (`extends`/`refines`/`constrains` items are
`{ spec/aspect, unit, ... }` maps by design, not bare units -- their `unit:`
field already is the unit). Any registry schema change. The structured
`supersedes` form (`{ spec, scope: partial, unit }`) the dry-run also surfaced:
that is partial-supersession **semantics**, not spelling, and is being taken to
design separately rather than widened into the grammar here.
