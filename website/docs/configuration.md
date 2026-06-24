---
id: configuration
title: Configuration
sidebar_position: 4
---

# Configuration

spec-spine is configured via a `spec-spine.toml` file at the root of your repository. 

An absent file yields a working default for a single-Cargo-workspace repository with `specs/` at the root. Every sub-table uses `deny_unknown_fields`: a typo is a loud configuration error (exit 3), not a silently ignored setting.

## Example `spec-spine.toml`

Below is a complete example showing all available sections and their default values.

```toml
[manifest]
# The key used in Cargo.toml or package.json to link code to a spec.
metadata_namespace = "spec-spine"

[domains]
# A closed enum for the optional `domain` field in spec frontmatter.
# Empty list means disabled (free-text allowed).
allowed = []

[kind]
# A closed enum for the optional `kind` field in spec frontmatter.
# Symmetric with [domains]. Empty list means disabled.
allowed = []

[layout]
specs_dir = "specs"
derived_dir = ".derived"
standards_dir = "standards/spec"
schemas_dir = "standards/schemas"
cargo_workspace = "Cargo.toml"
npm_workspaces = ["package.json", "pnpm-workspace.yaml"]
standalone_rust_workspaces = []
standalone_npm_packages = []

[index]
# Globs folded into the staleness content hash.
extra_hashed_inputs = ["standards/**", ".github/workflows/**"]
# Directory names pruned from symbol/section resolution walks.
resolver_exclusions = ["target", "node_modules", ".derived", "dist", "build", ".next"]
# Named glob groups for per-slice staleness checks (`index check --slice <name>`).
slices = {}

[branding]
# Identifiers stamped in emitted build metadata.
compiler_id = "spec-spine"
indexer_id = "spec-spine"

[coupling]
# Additions to the built-in bypass floor.
bypass_prefixes = []
# The PR-body waiver keyword.
waiver_keyword = "Spec-Drift-Waiver:"
# Auto-waive PRs that only change dependency versions in package.json.
auto_waive_dependency_only = false

[provenance.uri_schemes]
# Open map of provenance kind to URI scheme.
knowledge = "knowledge://"
code-fingerprint = "fingerprint://"

[frontmatter]
# Recognized frontmatter keys added without forking the types crate.
extra_known_keys = []
```

## Section Details

### `[manifest]`
- **`metadata_namespace`**: Determines the key used in package manifests to declare the owning spec. For Rust, this maps to `[package.metadata.<ns>].spec`. For npm, it maps to a top-level `"<ns>": {"spec": "..."}` object. Default: `"spec-spine"`.

### `[domains]` and `[kind]`
- **`allowed`**: Defines a closed enum for the `domain` and `kind` fields in spec frontmatter. If the list is empty (the default), the field is treated as free-text and not validated against an enum. If non-empty, any value provided in a spec must be a member of the list.

### `[layout]`
- Defines the directory structure conventions. 
- **`npm_workspaces`**: Manifests that declare npm/pnpm workspace members. The indexer reads member globs from whichever exists.
- **`standalone_*`**: Use these arrays to specify crates or packages that live outside the root workspace.

### `[index]`
- **`extra_hashed_inputs`**: Additional globs to include in the global content hash used for staleness checks.
- **`resolver_exclusions`**: Directories to skip during tree-sitter symbol and section resolution.
- **`slices`**: A map of named glob groups. Each is emitted as a `build.sliceHashes` entry and can be gated individually using `spec-spine index check --slice <name>`.

### `[branding]`
- **`compiler_id`** and **`indexer_id`**: Strings embedded into the `build-meta.json` output.

### `[coupling]`
- **`bypass_prefixes`**: An additive list of path prefixes that bypass the coupling gate. This adds to the built-in floor (which includes `.github/`, `docs/`, lockfiles, etc.). You cannot remove entries from the built-in floor.
- **`waiver_keyword`**: The string the gate looks for in a PR body to apply a waiver.
- **`auto_waive_dependency_only`**: If `true`, the gate mechanically self-waives PRs where every non-bypassed changed path is a `package.json` with only dependency version-string changes (e.g., Dependabot PRs).

### `[provenance.uri_schemes]`
- An open map defining URI schemes for different provenance kinds.

### `[frontmatter]`
- **`extra_known_keys`**: A list of custom frontmatter keys that your overlay or tooling recognizes. Keys listed here are accepted as first-class frontmatter and can carry arbitrary JSON-representable YAML values. Unknown keys overflow into a capped `extra_frontmatter` map and are restricted to scalars or string lists.
