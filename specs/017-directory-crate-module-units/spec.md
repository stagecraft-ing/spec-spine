---
id: "017-directory-crate-module-units"
title: "Authority units: resolve the reserved directory / crate / module kinds"
status: draft
kind: "tooling"
created: "2026-06-12"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "004-codebase-index"
extends:
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-types/src/unit.rs"
      - "crates/spec-spine-core/src/index.rs"
      - "crates/spec-spine-core/src/symbols.rs"
      - "crates/spec-spine-core/tests/index.rs"
      - "crates/spec-spine-types/tests/grammar.rs"
      - "crates/spec-spine-types/src/version.rs"
      - "crates/spec-spine-types/tests/dtos.rs"
summary: >
  Lifts the v1 unit-grammar limitation (`docs/design/00-architecture.md` §2.2,
  Q5): the three reserved kinds — `directory`, `crate`, and `module` — are now
  parsed and resolved alongside `file`/`section`/`symbol`. A `directory` unit
  resolves to its subtree (existence-checked, I-007 on miss); a `crate` unit
  resolves a manifest name against the discovered package inventory to that
  package's directory subtree (I-003 on miss, hyphen/underscore interchangeable);
  a `module` unit resolves a `::`-qualified Rust module path via a new module
  index — file-modules whole-file, top-level inline `mod` blocks to their span
  (I-008 on miss). The registry/index schemas are permissive on the unit payload,
  so this is an additive MINOR (`INDEX_SCHEMA_VERSION` 0.2.0 → 0.3.0) with no
  schema-file edit. Unblocks adopting the OAP corpus, which authors these three
  kinds across ~192 unit declarations (93 `directory`, 99 `crate`, 1 `module`)
  with no mass frontmatter migration.
---

# 017: directory / crate / module authority units

## 1. Purpose

The Phase-0 design (`00-architecture.md` §2.2) shipped three of six unit kinds
in v1 — `file`, `section`, `symbol` — and explicitly **reserved** the other
three as "an additive minor … so `crate`/`module` slot in … without breaking
readers" (Q5). `directory` was folded into trailing-slash file units; `crate`
and `module` were named-but-unbuilt.

The OAP-corpus dry-run is the trigger to build them. OAP authors all three as
first-class kinds — 93 `{ kind: directory, path }`, 99 `{ kind: crate, id }`,
and one `{ kind: module, id }` — across 100 specs. Rejecting them (the `Tagged`
deserializer's `deny_unknown_fields` surfaces an opaque "untagged enum" parse
error) is the single largest blocker to adoption. Migrating 192 unit
declarations to trailing-slash file units would be backwards: the corpus is the
adopter's truth, and the kinds are genuinely meaningful (a `crate` is an id, not
a path; a `module` is a `::`-path, not a file). The fix finishes the designed
capability rather than emulating a dialect.

## 2. Territory

The unit grammar (`unit.rs`: three new `Unit` variants and their `Tagged`
deserialize arms) and the indexer's resolver (`index.rs`: three new
`resolve_unit` arms; `canonical_unit`; the span-backing hash set; a
`needs_modules` gate) plus a Rust module index (`symbols.rs`:
`build_module_index`). The schemas are unchanged — they are deliberately
permissive on the typed-unit payload (`additionalProperties` on `traceMapping`,
no unit-shape `$ref`), so the new kinds validate as-is; the change is recorded
as the `INDEX_SCHEMA_VERSION` minor bump alone.

## 3. Behavior

### 3.1 Grammar

`Unit` gains three variants, each tagged on `kind` (symmetric with the existing
three, and composing with the spec-015 `{ unit: … }` wrapper):

```yaml
- { kind: directory, path: "platform/services/stagecraft/api/db" }
- { kind: crate, id: "factory-engine" }       # Cargo or npm manifest name
- { kind: module, id: "my_crate::serialization" }
```

A bare string remains shorthand for a `file` unit; a trailing-slash file path
remains a directory subtree. The explicit `directory` kind is the author-facing
spelling for the same subtree semantics, preserved across the round-trip rather
than normalized into a file unit (so `registry show` renders what was authored).

### 3.2 Resolution (in the indexer)

- **`directory`** → the directory path itself as a single `ResolvedLocation`
  (`span: None`); the coupling gate prefix-matches it against changed paths
  (the same subtree match a trailing-slash file unit gets, `claim_matches`).
  The directory MUST exist: a non-directory path is a blocking **I-007**.
- **`crate`** → the discovered `PackageRecord` whose manifest `name` equals the
  unit `id` (hyphen and underscore interchangeable, the Rust crate convention),
  resolved to that package's directory subtree (`span: None`). No match is a
  blocking **I-003**. Both Cargo crates and npm packages are admitted (the
  workspace boundary is the manifest, not the language).
- **`module`** → the Rust module index: file-modules resolve whole-file
  (`span: None`, the crate root resolving to the bare crate name); a top-level
  inline `mod X { … }` block resolves to its line span. No match is a blocking
  **I-008** (distinct from the symbol band's I-005). The module index is built
  only when some spec declares a `module` unit (mirroring the `symbol`-unit
  gate), keeping file/section/directory/crate corpora from parsing source.

### 3.3 Staleness

A `module` unit's *inline-block* location carries a span, so its backing source
file folds into the content hash (a line shift that moves the block stales the
index, exactly as for `symbol`/`section`). `directory`, `crate`, and the
*file-module* form of `module` are whole-subtree / whole-file (`span: None`) and
contribute no span-backing source — consistent with `file` units, whose content
does not affect resolution.

### 3.4 Diagnostic codes

`I-003` (unknown crate), `I-007` (missing directory), `I-008` (unresolved
module) join the existing blocking band (`I-003`..`I-009`) that fails
`index check`. They mirror OAP's hard-error semantics for owning-field units.

### 3.5 Tests (minimum)

- Grammar: each new kind parses from its tagged form; the `{ unit: … }` wrapper
  composes; `is_directory_subtree()` is true for `Directory`.
- Resolution: a `crate` unit resolves to the package subtree (hyphen/underscore
  tolerant); a `directory` unit resolves to its subtree with `span: None`; a
  `module` unit resolves both an inline `mod` (with span) and a file-module
  (whole-file).
- Diagnostics: `I-003` on an unknown crate id, `I-007` on a missing directory,
  `I-008` on an unresolved module.
- `INDEX_SCHEMA_VERSION` is pinned at `0.3.0`.

## 4. Out of scope

TypeScript/Python module resolution (the module index is Rust-only; the corpus
has no TS module unit). Nested-module resolution beyond top-level inline blocks
(a deeper `mod a { mod b {} }` resolves `a` but not `a::b` — file-modules and
top-level inline mods cover the corpus; deepen on demand). Any registry/index
JSON Schema text change (the payload is permissive by design). Re-emitting a
derived `implements:` view.
