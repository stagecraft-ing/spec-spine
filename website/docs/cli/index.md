---
id: index
title: spec-spine index
sidebar_position: 4
---

# spec-spine index

Scans manifests and specs to emit the codebase index, and provides staleness checks.

## Usage

```bash
spec-spine index
spec-spine index check [--slice NAME]
spec-spine index render [--json]
spec-spine index orphans [--json]
```

## Subcommands

### `index` (default)

Scans the repository for manifests (e.g., `Cargo.toml`, `package.json`) and specs, resolving authority units to their owning specs. Emits per-unit and per-package index shards to `.derived/codebase-index/by-spec/<id>.json` and `.../by-package/<slug>.json`.

### `index check`

The staleness gate. It recomputes the content hash of the current inputs and compares it against the committed index shards.

- **`--slice NAME`**: Checks staleness for a specific named slice defined in `[index.slices]` in the config, rather than the global content hash.

### `index render`

Renders the committed index as Markdown. This provides a human-readable view of the codebase index.
*(Note: `render` does not support `--json`.)*

### `index orphans`

Lists specs that have no resolved code units (i.e., specs that claim authority over paths that do not exist or cannot be resolved).

- **`--json`**: Output the list of orphaned spec IDs as a JSON array.

## Exit Codes

- **`index` (write):**
  - `0`: OK.
  - `3`: I/O, parse, schema, or config error.
- **`index check`:**
  - `0`: Fresh.
  - `2`: Stale (committed index is out of date).
  - `3`: I/O or parse error.

## Example

```bash
# Write the index
$ spec-spine index

# Check if the committed index is fresh
$ spec-spine index check
Error: Index is stale. Expected hash abc123def456, actual hash fed654cba321.
# (Exits with 2)
```
