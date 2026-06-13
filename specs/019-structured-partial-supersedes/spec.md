---
id: "019-structured-partial-supersedes"
title: "Supersedes grammar: structured items and partial (unit-scoped) transfer"
status: draft
kind: "tooling"
created: "2026-06-12"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
  - "005-coupling-gate"
extends:
  - spec: "000-spec-spine-bootstrap"
    nature: additive
    paths:
      - "crates/spec-spine-types/src/edges.rs"
      - "crates/spec-spine-types/src/frontmatter.rs"
      - "crates/spec-spine-types/src/lib.rs"
      - "crates/spec-spine-types/tests/grammar.rs"
      - "crates/spec-spine-types/tests/dtos.rs"
  - spec: "001-compile-registry"
    nature: additive
    paths:
      - "crates/spec-spine-types/src/registry.rs"
      - "crates/spec-spine-types/src/version.rs"
      - "crates/spec-spine-core/src/compile.rs"
      - "crates/spec-spine-core/tests/compile.rs"
  - spec: "002-registry-query"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/query.rs"
  - spec: "003-conformance-lint"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/lint.rs"
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/index.rs"
  - spec: "005-coupling-gate"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/couple.rs"
      - "crates/spec-spine-core/tests/couple.rs"
summary: >
  Implements Option A from `docs/design/01-partial-supersession.md`: a
  `supersedes` item may be a bare predecessor id (full) or a structured
  `{ spec, scope, unit?, note?, rationale? }`. A full item (bare id or
  `scope: full`) normalizes to the bare-string form, so a full-only corpus emits
  a byte-identical registry — the wire is unchanged for every existing adopter;
  only a partial item serializes as an object. `scope: partial` with a `unit`
  transfers authority over that unit alone (additive — the predecessor keeps it
  and everything else), threaded through the index as a `supersedes` resolved
  unit so the gate already treats the successor as an owner of that unit's paths;
  `build_superseders` therefore contributes only **full** supersession to the
  whole-spec transfer. A partial item with no unit (a documentary lifecycle
  marker) transfers nothing. Short ids on the predecessor resolve like
  `depends_on` (spec 016). Unblocks the OAP corpus shapes 073 (`scope: full`),
  108 (bare short id), 114 (`partial` + unit), and 199 (`partial` + note).

---

# 019: structured and partial supersedes

## 1. Purpose

`supersedes` was a flat `Vec<String>` of predecessor ids. The architecture's
edge table (§2.1) already labels it *"replaces a predecessor (partial/full);
inherits current authority"* — **partial was named but unbuilt**. The
Phase-0 OAP dry-run surfaced four authored shapes the flat grammar rejects:

- `108` — `supersedes: ["088"]` (bare **short** id; full).
- `073` — `{ spec, scope: full }` (object, full).
- `114` — `{ spec, scope: partial, unit }` (scoped transfer over one unit).
- `199` — `{ spec, scope: partial, note }` (partial, no unit — documentary).

`docs/design/01-partial-supersession.md` analysed this and recommended **Option
A** once a second adopter appeared. OAP is that adopter, so this builds it. The
design's load-bearing constraints are honoured: the full form stays a bare
string (byte-identical registries), and partial transfer is **additive** (§4.3)
— the successor *gains* the unit, the predecessor is never stripped — so the
gate's "any one owner clears" rule stays monotonic.

## 2. Territory

The supersedes grammar (`edges.rs`: a `SupersedeItem` union + `SupersedeScoped`
+ `SupersedeScope`, normalization), wired into the shared parse path
(`frontmatter.rs`) and the registry wire (`registry.rs`); compile-time short-id
resolution (`compile.rs`); the gate's transfer rule (`couple.rs`:
`build_superseders` restricts to full) and the index's partial-unit projection
(`index.rs`); the id-only views that read supersedes (`query.rs`
relationships, `lint.rs` reference targets). `REGISTRY_SCHEMA_VERSION`
0.2.0 → 0.3.0. The schema is permissive on the edge payload, so no schema-file
change.

## 3. Behavior

### 3.1 Grammar

```yaml
supersedes:
  - "088"                                                  # bare id → full
  - { spec: "073-x", scope: full }                         # object, full
  - { spec: "113-y", scope: partial, unit: "src/clone.ts" } # scoped transfer
  - { spec: "140-z", scope: partial, note: "retires §2.2" } # documentary
```

`SupersedeItem` is an untagged union: a string (full) or an object. The object
form is `{ spec, scope=full, unit?, note?, rationale? }` with
`deny_unknown_fields`. `scope` defaults to `full`.

### 3.2 Normalization & wire (the full form never escapes)

At parse time a `Scoped` item whose scope is `full` collapses to
`SupersedeItem::Full(spec)` — so a bare id and `{ scope: full }` are
indistinguishable downstream, and a corpus that uses only full supersession
emits the **exact same** `"supersedes": ["…", "…"]` string array as before. Only
a `partial` item serializes as an object (`{ spec, scope: "partial", unit? }`).
The byte-equivalence of a full-only corpus is the compatibility contract.

### 3.3 Authority transfer

- **Full** (bare / `scope: full`): unchanged from spec 005 — the successor
  inherits the predecessor's entire authority surface (`build_superseders` →
  `owners_for_path` step 2, additive, transitive across chains).
- **Partial** (`scope: partial`, with `unit`): the index resolves the `unit` as
  a `SourceField::Supersedes`, ownership-bearing resolved unit on the
  **successor**, so `owners_for_path` step 1 makes the successor an owner of
  exactly that unit's paths — and `build_superseders` **excludes** partial items,
  so the successor does **not** also inherit the predecessor's other units.
  Additive: the predecessor keeps the unit too.
- **Partial without `unit`** (`note`/`rationale` only): a documentary lifecycle
  marker. No resolved unit, not in `build_superseders` → transfers nothing.

Chains stay scoped: because a partial edge never enters `build_superseders`, a
successor's own superseders never inherit the predecessor's other units through
it.

### 3.4 Short-id resolution

The predecessor named by a `supersedes` item resolves a leading-number short id
(`088` → `088-slug`) by the same `resolve_spec_ref` policy as `depends_on` /
`superseded_by` (spec 016), so the transfer keys match the registry ids.

### 3.5 Tests (minimum)

- Grammar: bare id and `{ scope: full }` both parse to `Full`; `partial` + unit
  exposes its scoping unit; `partial` + note has no transfer unit.
- Wire: a full corpus emits bare-string `supersedes`; a partial emits an object
  (scope + unit).
- Gate: full transfers the whole surface (existing test stays green); partial
  transfers the named unit only (the predecessor's other unit is NOT cleared by
  editing the successor); a unit-less partial transfers nothing.

## 4. Out of scope

Exclusive hand-off (the predecessor *losing* the unit) — the design's §4.3 first
cut is additive; a true removal is the graph's first owner-stripping operation
and waits for demand. Partial transfer scoped to a `section`/`symbol` unit is
supported by the same resolved-unit path but unexercised by the corpus (the only
partial-with-unit is a file unit). A distinct constraint/lifecycle-violation
exit code. Re-emitting a derived `implements:` view.
