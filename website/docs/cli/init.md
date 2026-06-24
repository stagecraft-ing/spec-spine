---
id: init
title: spec-spine init
sidebar_position: 2
---

# spec-spine init

Scaffolds a new spec-spine governance corpus in the current repository.

## Usage

```bash
spec-spine init [--force]
```

## Description

`init` writes a starter governance corpus to your repository. It creates the `spec-spine.toml` configuration file, the `standards/` directory containing the constitution and templates, the initial `specs/000-bootstrap/spec.md`, and the agent rules under `.claude/rules/`.

By default, `init` skips any files that already exist.

## Flags

| Flag | Value | Default | Effect |
|---|---|---|---|
| `--force` | None | false | Overwrite existing files. |

## Exit Codes

- `0`: Scaffolded successfully.
- `1`: Target exists and `--force` was not provided.
- `3`: I/O write error.

## Example

```bash
$ spec-spine init
Scaffolded spec-spine.toml
Scaffolded standards/spec/constitution.md
...
Next step: customize specs/000-bootstrap/spec.md then run `spec-spine compile`
```
