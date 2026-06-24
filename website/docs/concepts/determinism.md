---
id: determinism
title: Determinism
sidebar_position: 5
---

# Determinism

spec-spine is deterministic by construction. Every artifact-producing function is a pure function of `(config, file contents)`.

Same inputs, byte-identical output, on every platform.

## How determinism is achieved

- **No ambient state:** The engine does not read the system clock, environment variables, or shell out to Git. (The CLI parses the Git diff and passes it in as data; the one wall-clock field, `build-meta.json`, is excluded from determinism checks).
- **Canonical JSON:** Emitted JSON is sorted by key, pretty-printed with two spaces, normalized to LF line endings, and ends with a trailing newline.
- **Normalized hashing:** Content hashes are computed over LF/BOM-normalized, path-sorted bytes.
- **Pinned parsers:** Tree-sitter grammars used for symbol resolution are pinned exact.

## Why determinism matters

Determinism makes the committed registry and index a reliable baseline. 

Two agents producing changes independently produce diffable, mechanically-mergeable registries: there is no interpretation drift at merge time, and staleness is detectable by content-hash comparison alone.

CI proves this by asserting byte-identical `registry.json` and `index.json` across four release triples (`x86_64` and `aarch64` Linux, `aarch64` macOS, `x86_64` Windows).
