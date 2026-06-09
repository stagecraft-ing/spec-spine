# spec-spine contract (normative summary)

A one-page operational summary of the bootstrap spec
(`specs/000-spec-spine-bootstrap/spec.md`), for quick reference. The bootstrap
spec and the constitution are authoritative; where this summary is terser, they
govern.

## Inputs (authored truth: markdown only)

- `specs/NNN-slug/spec.md`: one spec per directory; directory name equals `id`;
  `NNN` is a unique three-digit ordinal.
- `standards/spec/`: this constitution, this contract, and templates.
- `spec-spine.toml`: the repo's configuration (all keys optional; absent ⇒
  conventional single-Cargo-workspace defaults).

## Outputs (machine truth: compiler-owned JSON, read via the typed consumer only)

- `<derived_dir>/spec-registry/registry.json`: spec-as-source (output of `compile`).
- `<derived_dir>/spec-registry/build-meta.json`: wall-clock metadata (the only
  non-deterministic artifact; excluded from determinism/golden checks).
- `<derived_dir>/codebase-index/index.json`: code-as-source (output of `index`).

## Required frontmatter

`id`, `title`, `status` (`draft`/`approved`/`superseded`/`retired`), `created`
(`YYYY-MM-DD`), and `summary`. Everything else is optional.

## Typed edges (8; `references` is the only non-owning one)

`establishes`, `extends`, `refines`, `supersedes`, `amends`, `co_authority`,
`constrains`, `references`. `origin` is a bootstrap marker, not an edge.

## Authority units (v1)

`file` (bare string shorthand; trailing slash ⇒ subtree), `section`
(`{file, anchor}`), `symbol` (`{id}`, resolved by the indexer). `crate`/`module`/
`directory` are reserved for later minors.

## The gate chain

`compile` → `index` → `lint` → `couple`: the coupling gate refuses a merge where
a claimed code unit and its owning spec disagree, unless a scoped
`Spec-Drift-Waiver:` reason is present in the PR body. Configured bypass prefixes
(docs, lockfiles, `.derived/`) are exempt.

## Determinism

Pure function of `(config, file contents)` → byte-identical output; the ledger is
diffable and mechanically mergeable; staleness is detected by content-hash
comparison alone.
