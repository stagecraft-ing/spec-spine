---
id: overview
title: CLI Overview
sidebar_position: 1
---

# CLI Overview

The `spec-spine` command-line tool is a thin translation of `spec-spine-core` results into stdout/stderr and stable exit codes. All `process::exit`, stdout, and `git` side effects live here; the engine stays pure.

## Global Flags

| Flag | Value | Default | Effect |
|---|---|---|---|
| `--repo <DIR>` | Path | `.` (current directory) | Selects the repository root. |

## Exit Codes

Exit codes are a stable contract across the entire CLI surface:

| Code | Meaning |
|---|---|
| `0` | Success / OK. |
| `1` | Validation failure, not-found, or coupling drift. |
| `2` | Staleness (committed index is out of date). |
| `3` | I/O, parse, schema, or config error. |

## Command Surface

| Command | Capability |
|---|---|
| [`spec-spine init`](init.md) | Scaffold a new adopter (config, standards, specs/000, rules). |
| [`spec-spine compile`](compile.md) | Validate frontmatter and emit the deterministic registry. |
| [`spec-spine index`](index.md) | Scan manifests and specs to emit the codebase index. Includes `check`, `render`, and `orphans` subcommands. |
| [`spec-spine registry`](registry.md) | Typed read-only queries against the compiled registry. Includes `list`, `show`, `status-report`, and `relationships`. |
| [`spec-spine lint`](lint.md) | Check corpus conformance. |
| [`spec-spine couple`](couple.md) | The PR-time drift gate. |

*(Note: Additional commands like `attest` and `verify-attestation` may be present in the binary but are considered advanced/experimental surface outside the core workflow.)*
