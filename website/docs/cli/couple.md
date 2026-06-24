---
id: couple
title: spec-spine couple
sidebar_position: 7
---

# spec-spine couple

The PR-time drift gate. It joins the registry and index against a Git diff and refuses the merge if code drifts from its owning spec.

## Usage

```bash
spec-spine couple --base <ref> --head <ref> [--pr-body FILE] [--paths-from FILE]
```

## Description

The coupling gate cross-references every modified code path against the authority graph. If a path is changed but its owning spec is not part of the diff (and no waiver is present), the gate fails.

It uses `git diff --no-color -U0 base...head` to determine the changed paths.

## Flags

| Flag | Value | Required | Effect |
|---|---|---|---|
| `--base` | Git ref | Yes | The merge base commit (e.g., `origin/main`). |
| `--head` | Git ref | Yes | The head commit of the PR (e.g., `HEAD`). |
| `--pr-body` | File path | No | A file containing the PR body text, used to scan for waivers. |
| `--paths-from` | File path | No | Read a list of changed paths from a file instead of running `git diff`. |

## Waivers

If a PR-body file is provided, the gate scans it for the configured waiver keyword (default: `Spec-Drift-Waiver:`). If found, blocking drift violations are downgraded to warnings, and the command exits with `0`.

Alternatively, if `SPEC_SPINE_PR_BODY` is set in the environment, the CLI will use it as a fallback if `--pr-body` is not provided.

## Exit Codes

- `0`: No drift, or drift was waived.
- `1`: Blocking drift (uncovered paths).
- `2`: Index is stale (you must re-run `spec-spine index` first).
- `3`: I/O, parse, load, or config error.

## Example

```bash
$ spec-spine couple --base origin/main --head HEAD --pr-body /tmp/pr-body.txt
Coupling check failed:
Drift detected on src/main.rs (owned by 001-hello-world).
Spec 001-hello-world was not modified in this diff.
# (Exits with 1)
```
