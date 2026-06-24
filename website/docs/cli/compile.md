---
id: compile
title: spec-spine compile
sidebar_position: 3
---

# spec-spine compile

Validates spec frontmatter and emits the deterministic registry.

## Usage

```bash
spec-spine compile
```

## Description

The compiler reads the markdown spec corpus (`specs/*/spec.md`), validates the YAML frontmatter, and emits the spec-as-source view.

The output is written as per-unit registry shards to `.derived/spec-registry/by-spec/<id>.json`. The output is deterministic: the same inputs produce byte-identical output on every platform.

## Exit Codes

- `0`: Validation passed and registry written.
- `1`: Validation failed (e.g., malformed frontmatter, invalid edge).
- `3`: I/O, parse, schema, or config error.

## Example

```bash
$ spec-spine compile
Compiled 42 specs to .derived/spec-registry/
```
