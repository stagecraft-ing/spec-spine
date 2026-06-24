---
id: faq
title: FAQ and Troubleshooting
sidebar_position: 10
---

# FAQ and Troubleshooting

## Common Drift Cases

### "Drift detected on path X"
The coupling gate fails when a path is modified but its owning spec is not in the diff.
**Resolution:** Either modify the owning spec to reflect the code change, or provide a waiver in the PR body (e.g., `Spec-Drift-Waiver: mechanical refactor`).

### "Index is stale"
The `spec-spine index check` command fails with exit code 2. This means the committed index shards do not match the current codebase.
**Resolution:** Run `spec-spine index` locally and commit the resulting `.derived/` directory.

## Waiver Patterns

### Dependency Updates
For automated dependency updates (like Dependabot), you can enable `coupling.auto_waive_dependency_only = true` in your `spec-spine.toml`. This mechanically self-waives PRs where every non-bypassed changed path is a `package.json` with only dependency version-string changes.

### Refactors
For cross-cutting refactors that do not change the underlying specification, use the PR-body waiver:
`Spec-Drift-Waiver: renaming internal helper functions, behavior unchanged.`

## Symbol Resolution

If a symbol is not resolving correctly, ensure that the tree-sitter grammar for your language (Rust or TypeScript) is correctly parsing the file. Python symbol resolution is currently deferred. Also, check that the directory is not listed in `index.resolver_exclusions`.

## Manifest Discovery

If the indexer is not finding your packages:
- Check `layout.cargo_workspace` or `layout.npm_workspaces` in your config.
- If you have standalone crates outside the root workspace, add them to `layout.standalone_rust_workspaces` or `layout.standalone_npm_packages`.
- Ensure your manifests contain the correct metadata key (e.g., `[package.metadata.spec-spine]`).
