---
id: extending-and-overlays
title: Extending and Overlays
sidebar_position: 7
---

# Extending and Overlays

To layer domain-specific output on top of spec-spine without forking the core, you build an **overlay**. An overlay consumes the generic crates as libraries, reads the generic artifacts via stable loaders, and emits its own sibling artifact.

The generic core deliberately omits domain-specific machinery (like compliance reports or factory artifacts). That is the job of an overlay.

## The Shape of an Overlay

An overlay is a separate crate or script that:

1. Depends on `spec-spine-core` (and transitively `spec-spine-types`).
2. Reads the committed generic artifacts (`registry.json`, `index.json`) via the public loaders.
3. Computes an enriched view.
4. Emits a **sibling** artifact next to the generic one, typically named `<artifact>-<overlay>.json` (e.g., `registry-compliance.json`).

It does **not** modify, replace, or re-emit the generic artifacts.

## The Stable Seam

The stable contract is the typed loaders:

```rust
use spec_spine_core::{load_registry, load_index};

pub fn load_registry(bytes: &[u8]) -> Result<Registry, Error>;
pub fn load_index   (bytes: &[u8]) -> Result<CodebaseIndex, Error>;
```

These loaders parse the canonical bytes into owned DTOs and **reject an unknown MAJOR schema version**.

### What is stable to depend on

- The `load_registry` and `load_index` signatures.
- The `Registry` and `CodebaseIndex` DTO field shapes within a MAJOR version.
- The `Error` enum variants.
- The `extra_frontmatter` escape hatch.

Internal modules like `canonical_json` or `symbols` are not stable.

## The `extra_frontmatter` Escape Hatch

Overlays usually require spec authors to declare overlay-specific fields in frontmatter (e.g., `compliance_tier:`). You can carry these through the compiler without forking the types crate:

1. **`frontmatter.extra_known_keys`**: List the keys your overlay recognizes in `spec-spine.toml`. They are accepted as first-class frontmatter and can carry arbitrary JSON-representable YAML values.
2. **`extra_frontmatter`**: Any unknown key overflows into this capped map on the `SpecRecord`. Undeclared keys are restricted to scalar and string-list values.

Your overlay reads these from the loaded `Registry`.

## Re-using Canonicalization

To keep your sibling artifact diffable, serialize it with sorted keys, two-space pretty printing, LF line endings, and a trailing newline. This matches the generic canonicalization rules.
