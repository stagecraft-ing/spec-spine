---
id: lint
title: spec-spine lint
sidebar_position: 6
---

# spec-spine lint

Checks the corpus for conformance and well-formedness.

## Usage

```bash
spec-spine lint [--fail-on-warn] [--fail-on-info]
```

## Description

`lint` runs a suite of conformance checks across the spec corpus. It prints every violation with its severity (error, warn, info) and summarizes the counts.

By default, it exits with status `1` only if there are error-tier violations.

## Flags

| Flag | Value | Default | Effect |
|---|---|---|---|
| `--fail-on-warn` | None | false | Exit 1 if there are any warn-tier violations. |
| `--fail-on-info` | None | false | Exit 1 if there are any info-tier violations. |

## Exit Codes

- `0`: Clean (or only warnings/info without the respective fail flags).
- `1`: Error-tier violations found, or warn/info violations found with `--fail-on-*` set.
- `3`: I/O or parse error.

## Example

```bash
$ spec-spine lint --fail-on-warn
Linting corpus...
WARN: spec 042-old-thing has no relationships.
1 warning(s) found.
# (Exits with 1)
```
