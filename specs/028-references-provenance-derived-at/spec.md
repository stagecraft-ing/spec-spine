---
id: "028-references-provenance-derived-at"
title: "References provenance: optional derived_at timestamp"
status: approved
kind: "tooling"
created: "2026-06-18"
owner: "The spec-spine Authors"
implementation: complete
risk: low
depends_on:
  - "001-compile-registry"               # the references edge + Provenance type + registry schema
extends:
  - spec: "000-spec-spine-bootstrap"
    nature: additive
    paths:
      - "crates/spec-spine-types/src/edges.rs"      # the Provenance struct gains an optional field
      - "crates/spec-spine-types/tests/grammar.rs"  # provenance round-trip + rejection tests
      - "crates/spec-spine-types/tests/dtos.rs"      # the pinned REGISTRY_SCHEMA_VERSION assertion
  - spec: "001-compile-registry"
    nature: additive
    paths:
      - "crates/spec-spine-types/src/version.rs"     # REGISTRY_SCHEMA_VERSION 1.0.0 -> 1.1.0
references:
  - { unit: { kind: file, path: "docs/design/00-architecture.md" }, role: context }
summary: >
  Adds an optional `derived_at` field to the `Provenance` struct carried on a
  `references` edge: `{ kind, ref, derived_at? }`. `derived_at` is a generic,
  optional ISO-8601 timestamp recording when the reference was derived, preserved
  verbatim with no format validation in the type (an adopter that wants format
  enforcement adds a lint). The field is additive and optional, so a corpus that
  declares none is unaffected and `deny_unknown_fields` is preserved (the field
  becomes known, not a hole in the schema). The registry format gains an
  emittable field, so `REGISTRY_SCHEMA_VERSION` bumps an additive minor 1.0.0 ->
  1.1.0 (the permissive shard schema file is unchanged; the bump only restamps the
  `specVersion` carried by each committed registry shard, leaving every
  `shardHash` and `record` body byte-identical, because a shard hash is taken over
  the `spec.md` source bytes). Filed off the OAP spec-217 engine-swap, whose
  decomposition synthesizer (OAP spec 165) emits this shape because OAP spec 161
  FR-007 requires `provenance.derived_at`; the published library rejected it at
  parse with `V-002: unknown field derived_at`, producing zero shards. This is the
  second small, broadly-useful additive concession from the 217 swap (the first
  was the spec-027 symbol-resolution feature gate, shipped as 0.7.0).
---

# 028: References provenance, optional `derived_at`

Filed off the OAP spec-217 engine-swap, which repoints OAP's decomposition
pipeline promotion step at the published `spec-spine-core::compile`. That compile
rejected pipeline-generated specs at parse time:

```
V-002: malformed frontmatter: unknown field `derived_at`, expected `kind` or `ref`
```

OAP's decomposition synthesizer (OAP spec 165) emits a `code-fingerprint`
provenance reference carrying a `derived_at` timestamp, because OAP spec 161
FR-007 requires `provenance.derived_at`:

```yaml
references:
  - role: decomposition-origin
    provenance:
      kind: code-fingerprint
      ref: "xray-fingerprint://<sha256>"
      derived_at: "<ISO-8601>"
```

The library `Provenance` struct was `{ kind, ref }` with `deny_unknown_fields`, so
`derived_at` was rejected at parse and the spec produced zero shards. OAP's in-tree
compiler tolerated the field; the strict library does not. The
`code-fingerprint` / `xray-fingerprint://` scheme is already registrable through
`[provenance.uri_schemes]` in the adopter's `spec-spine.toml`, so `derived_at` was
the only blocker.

## 1. Purpose

Let `Provenance` carry an optional, generic derivation timestamp so a fingerprint
or knowledge provenance reference can record *when* it was derived, without
weakening the strict grammar for anyone else. `derived_at` is not OAP-specific in
spirit: any adopter emitting fingerprint or knowledge provenance benefits from a
derivation timestamp. The change is additive and optional, so specs without it are
byte-for-byte unaffected.

## 2. Territory

This spec additively claims, alongside spec 000 (the type substrate) and spec 001
(the registry schema), the four files it edits. It amends no existing contract: the
`{ kind, ref }` shape stays valid and unchanged; the field is purely additive.

- `crates/spec-spine-types/src/edges.rs`: the `Provenance` struct gains
  `derived_at: Option<String>` with `#[serde(default, skip_serializing_if =
  "Option::is_none")]`. `deny_unknown_fields` is retained.
- `crates/spec-spine-types/src/version.rs`: `REGISTRY_SCHEMA_VERSION` bumps
  1.0.0 -> 1.1.0 (additive minor), because the registry format can now emit a
  field it could not before.
- `crates/spec-spine-types/tests/grammar.rs`: round-trip and rejection tests for
  the new field (AC-1, AC-2, AC-3).
- `crates/spec-spine-types/tests/dtos.rs`: the pinned `REGISTRY_SCHEMA_VERSION`
  assertion follows the bump.

The package version bump and crates.io publish (an additive optional field is a
semver-minor, target `0.8.0`) are out of scope here: they are a separate release
step (`docs/releasing.md`), as for every prior spec.

## 3. Behavior

### 3.1 The field

`Provenance` is `{ kind, ref, derived_at? }`. `derived_at` is an opaque string
preserved verbatim: the type performs no timestamp-format validation, matching how
`ref`'s scheme is validated against config rather than hard-coded. An adopter that
wants ISO-8601 enforcement adds a lint over the compiled registry.

### 3.2 Absent field is byte-stable

`skip_serializing_if = "Option::is_none"` means a provenance reference that
declares no `derived_at` serializes exactly as before. Every existing corpus,
including this repo's own, emits byte-identical record bodies; the only on-disk
change is the `specVersion` restamp (3.3).

### 3.3 Schema-version bump

The registry format gains an emittable field, so `REGISTRY_SCHEMA_VERSION` bumps
an additive minor (1.0.0 -> 1.1.0), consistent with spec 013 (0.2.0), spec 019
(0.3.0), and the index policy in spec 025. The embedded shard schema file is
already permissive (`additionalProperties` on the record payload), so no schema
file changes and the conformance test (which validates emitted JSON against the
embedded schema) still passes. Because a registry shard hash is taken over the
`spec.md` source bytes, the bump restamps only the `specVersion` field of each
committed registry shard; every `shardHash` and `record` body is byte-identical.

### 3.4 The gate is not weakened

`deny_unknown_fields` is preserved on `Provenance`: a genuinely-unknown field
(neither `kind`, `ref`, nor `derived_at`) still produces the V-002 parse error.
The additive field closes one hole by name; it does not open the struct.

## 4. Functional requirements

- **FR-001 (additive field).** `Provenance` carries an optional
  `derived_at: Option<String>` that defaults to absent and is skipped on
  serialization when absent.
- **FR-002 (passthrough).** `derived_at` is preserved verbatim on parse and
  re-emit; the type performs no timestamp-format validation.
- **FR-003 (strictness retained).** `deny_unknown_fields` stays on `Provenance`:
  an unknown field still fails parse with V-002.
- **FR-004 (additive minor).** `REGISTRY_SCHEMA_VERSION` is 1.1.0; the embedded
  registry/shard schema files are unchanged and the conformance test passes.
- **FR-005 (byte-stable absence).** A corpus that declares no `derived_at` emits
  byte-identical registry record bodies; only the per-shard `specVersion` restamps.

## 5. Acceptance criteria

- **AC-1 (round-trips).** A `references` provenance item `{ kind, ref, derived_at }`
  parses and the timestamp is preserved on re-emit
  (`references_provenance_derived_at_round_trips` in `tests/grammar.rs`).
- **AC-2 (absent field omitted).** An item without `derived_at` parses and does
  NOT serialize the field
  (`references_provenance_without_derived_at_omits_the_field`).
- **AC-3 (strict).** An item with a genuinely-unknown provenance field still
  produces the `deny_unknown_fields` error
  (`references_provenance_unknown_field_is_rejected`).
- **AC-4 (suite green).** `cargo test --workspace --locked` passes, including the
  conformance and golden suites; `tests/dtos.rs` asserts the 1.1.0 pin.
- **AC-5 (self-corpus delta).** The committed-artifact change is exactly: (a) the
  `specVersion` field restamps 1.0.0 -> 1.1.0 in every registry `by-spec` shard,
  with every `shardHash` and `record` body byte-identical; (b) the two new 028
  shards (`spec-registry/by-spec/028-*.json`,
  `codebase-index/by-spec/028-*.json`). No existing index shard changes: 028
  claims its source files as file units (which track path, not content) and
  per-spec sharding (spec 024) confines 028's new claims to its own shard, so no
  other spec's `by-spec` record or any `by-package` shard moves.

## 6. Out of scope

- **The package version bump and release** (`0.8.0`): a separate release step.
- **Timestamp-format validation.** `derived_at` is an opaque passthrough string;
  format enforcement, if wanted, is an adopter lint, not a type concern.
- **Provenance scheme validation.** Wiring `kind` against
  `[provenance.uri_schemes]` is unchanged by this spec.
- **Any other provenance field.** Only `derived_at` is added; the struct stays
  closed (`deny_unknown_fields`) to everything else.
