# The overlay contract

> How to layer domain-specific output on top of spec-spine **without forking the
> core**. An overlay consumes the generic crates as libraries, reads the generic
> artifacts via stable loaders, and emits its own *sibling* artifact. This is the
> supported extensibility model; the generic core implements **none** of it.

The generic core deliberately omits domain machinery: OAP's `kind`-enum,
`shape`/`category` dimensions, capability/registry/profile system, compliance and
factory output, and the Claude `config-hash` gate are all out of scope (see
[design/00-architecture.md](design/00-architecture.md) §10.4). Anything in that
category is an **overlay's** job. This document is the contract an overlay
depends on.

---

## 1. The shape of an overlay

An overlay is a separate crate (published or private) that:

1. depends on `spec-spine-core` (and transitively `spec-spine-types`);
2. reads the committed generic artifacts (`registry.json`, `index.json`) via the
   public loaders;
3. computes whatever enriched view it needs;
4. emits a **sibling** artifact next to the generic one, by convention
   `<artifact>-<overlay>.json` (e.g. `registry-compliance.json`,
   `index-factory.json`).

It does **not** modify, replace, or re-emit the generic artifacts. The generic
`spec-spine` toolchain remains the single producer of `registry.json` /
`index.json`; overlays are strictly additive readers.

```
.derived/
├─ spec-registry/
│  ├─ registry.json            ← produced by `spec-spine compile` (generic, canonical)
│  └─ registry-compliance.json ← produced by your overlay (sibling)
└─ codebase-index/
   ├─ index.json               ← produced by `spec-spine index` (generic, canonical)
   └─ index-factory.json       ← produced by your overlay (sibling)
```

---

## 2. The stable seam: typed loaders

These two functions are the contract. They parse the canonical bytes into owned,
`serde`-serializable DTOs and **reject an unknown MAJOR schema version** (so an
overlay built for `0.x` will not silently misread a `1.x` artifact; see
[schema-versioning.md](schema-versioning.md)):

```rust
use spec_spine_core::{load_registry, load_index};
use spec_spine_core::types::{Registry, CodebaseIndex};

pub fn load_registry(bytes: &[u8]) -> Result<Registry,      Error>;
pub fn load_index   (bytes: &[u8]) -> Result<CodebaseIndex, Error>;
```

A minimal overlay:

```rust
use std::fs;
use spec_spine_core::{load_registry, load_index};

fn run(repo_root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let derived = repo_root.join(".derived");
    let registry = load_registry(&fs::read(derived.join("spec-registry/registry.json"))?)?;
    let index    = load_index(&fs::read(derived.join("codebase-index/index.json"))?)?;

    // ... compute your enriched view from the typed structs ...
    let enriched = my_enrichment(&registry, &index);

    // Emit a sibling. Match the generic canonicalization for diffable output:
    // sorted keys, 2-space pretty, LF, trailing newline.
    let bytes = canonical_json(&enriched);   // your serializer; see §4
    fs::write(derived.join("spec-registry/registry-compliance.json"), bytes)?;
    Ok(())
}
```

`spec-spine-core` also re-exports the full type substrate as
`spec_spine_core::types`, so an overlay depends on a single crate.

---

## 3. What is stable to depend on

| Stable (depend freely) | Not stable (do not depend on) |
|---|---|
| `load_registry` / `load_index` signatures and behavior | the internal `canonical_json`, `hash`, `symbols` modules of core |
| the `Registry` / `CodebaseIndex` / `SpecRecord` DTO field shapes within a MAJOR | the in-memory layout of any private struct |
| the `Error` enum variants and their exit-code mapping | the on-disk path layout *beyond* `derived_dir` (use the config) |
| the `extra_frontmatter` escape hatch (§5) | the wire format of `build-meta.json` (non-deterministic, excluded from goldens) |
| the canonicalization rules (sorted keys, pretty, LF, trailing newline) | grammar-internal symbol-resolution details |

Within a MAJOR version, fields are only **added**, never removed or retyped. A
breaking change is a MAJOR bump, which the loaders reject for an old reader.
Read the policy in [schema-versioning.md](schema-versioning.md).

---

## 4. Re-using the canonicalization (so your sibling is diffable too)

The generic artifacts are emitted with **sorted object keys, 2-space pretty
printing, LF line endings, and a trailing newline**, so they diff and merge
mechanically. An overlay's sibling artifact *should* match, but core's
`canonical_json` is an internal module (not part of the stable surface). Reproduce
the rules in your overlay: serialize via a `BTreeMap`-backed value (sorted keys),
pretty-print with two spaces, normalize to LF, and append a trailing newline.
This keeps your sibling artifact as review-friendly as the generic one.

---

## 5. The `extra_frontmatter` escape hatch: carrying overlay data through

An overlay usually needs spec authors to declare overlay-specific fields in their
frontmatter (e.g. `compliance_tier:`, `factory_target:`). spec-spine carries
those through the compiler **without forking the types crate**, two ways:

- **`frontmatter.extra_known_keys`** (config): list the keys your overlay
  recognizes. They are accepted as first-class frontmatter rather than triggering
  an unknown-key diagnostic.
- **`extra_frontmatter`** (DTO field): any frontmatter key not otherwise known
  overflows into a capped `extra_frontmatter` map on the `SpecRecord`. Undeclared
  keys are restricted to scalar and string-list values; keys you list in
  `extra_known_keys` carry any JSON-representable YAML value, including arbitrary
  nesting (spec 013). Your overlay reads it from the loaded `Registry`.

The compiler validates and emits these deterministically alongside the generic
fields; the overlay picks them up from the typed `Registry`. Neither path
requires editing `spec-spine-types` or `spec-spine-core`.

---

## 6. Worked example: OAP self-adoption

OAP (the origin repo) is the canonical overlay case. Its compliance reports,
factory artifacts, capability/registry/profile system, and Claude config-hash
gate are **not** generic. OAP adopts spec-spine as **generic core + an OAP
overlay crate**, not as a drop-in replacement for its current tooling:

- `spec-spine compile` / `index` produce the generic `registry.json` /
  `index.json`.
- An `oap-overlay` crate calls `load_registry` / `load_index`, reads OAP-specific
  fields via `extra_frontmatter` (declared in `frontmatter.extra_known_keys`),
  and emits `registry-oap.json` / `index-oap.json` plus its compliance/factory
  reports.
- OAP's config-hash gate becomes an overlay gate that runs after the generic
  coupling gate, reading the same diff.

This mirrors exactly how OAP's enrichers already work today (they consume the
generic crates as libraries and emit sibling `*-oap.json` artifacts); spec-spine
makes that the *supported* shape instead of an internal convention.

---

## 7. Rules of the road

- **Read, don't rewrite.** An overlay never re-emits `registry.json` /
  `index.json`. One generic producer; many sibling consumers.
- **Pin your core dependency** to the MAJOR you target; the loaders enforce it at
  read time.
- **Stay deterministic.** If your overlay is to be CI-gated the same way, keep it
  a pure function of `(generic artifacts, overlay config, file contents)`, no
  ambient clock/env, matching the core's determinism contract.
- **Name siblings `<artifact>-<overlay>.json`** so they are discoverable and never
  collide with the generic names.
