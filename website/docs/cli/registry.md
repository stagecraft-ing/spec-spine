---
id: registry
title: spec-spine registry
sidebar_position: 5
---

# spec-spine registry

Typed read-only queries against the compiled registry.

## Usage

```bash
spec-spine registry list [--status S] [--ids-only] [--json]
spec-spine registry show <id> [--json]
spec-spine registry status-report [--nonzero-only] [--json]
spec-spine registry relationships <id> [--json]
```

## Subcommands

### `registry list`

Lists specs from the committed registry.

- **`--status S`**: Filter by status (`draft`, `approved`, `superseded`, `retired`).
- **`--ids-only`**: Print only the spec IDs, one per line.
- **`--json`**: Output as JSON.

### `registry show <id>`

Shows the details of a single spec.

- **`--json`**: Output as JSON.

### `registry status-report`

Shows counts of specs by their lifecycle status.

- **`--nonzero-only`**: Omit statuses with a count of zero.
- **`--json`**: Output as JSON.

### `registry relationships <id>`

Shows the relationship neighborhood (incoming and outgoing edges) for a specific spec.

- **`--json`**: Output as JSON.

## Exit Codes

- `0`: OK.
- `1`: Spec ID or view not found.
- `3`: I/O, parse, schema, or config error.

## Example

```bash
$ spec-spine registry list --status approved --ids-only
000-bootstrap
001-compile-registry
002-registry-query
```
