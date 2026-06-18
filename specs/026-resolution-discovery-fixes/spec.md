---
id: "026-resolution-discovery-fixes"
title: "Indexer resolution + discovery fixes: foreign-YAML regions, Makefile tags, nested-workspace globs"
status: approved
kind: "tooling"
created: "2026-06-18"
owner: "The spec-spine Authors"
implementation: pending
risk: low
depends_on:
  - "004-codebase-index"          # section resolution + manifest discovery
  - "022-keypath-section-anchors" # the eligible-file predicate and §3.2 region promise
amends:
  - "004-codebase-index"          # Makefile `## tag:` determinism; nested-workspace glob base
  - "022-keypath-section-anchors" # fulfill §3.2's foreign-YAML region promise; retire §4's D4 deferral
extends:
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/sections.rs"   # foreign-YAML region dispatch + Makefile tag determinism
      - "crates/spec-spine-core/src/manifest.rs"   # nested-workspace glob base resolution
      - "crates/spec-spine-core/tests/index.rs"    # nested-workspace + foreign-YAML-region fixtures
references:
  - { unit: { kind: file, path: "docs/design/00-architecture.md" }, role: context }
summary: >
  Three mechanical correctness fixes surfaced by the OAP spec-217 Phase-0 dry run
  of 0.5.0, all in the indexer's resolution and discovery paths, none changing any
  schema or DTO shape. (1) Non-workflow YAML (Helm values, infra manifests) is
  dispatched to the CI-jobs parser, so its `# region:` markers never resolve and a
  declared section unit on one raises a spurious I-006; route foreign YAML to the
  region parser, which is exactly the whole-file / region ownership spec 022 §3.2
  already promised foreign structured configs (retiring §4's deliberately-deferred
  D4). (2) A Makefile `## tag:` is silently overwritten when a second `## tag:`
  precedes any consuming target, so a tagged region is never emitted; make the
  pending tag deterministic so it is never silently lost. (3) Workspace-member
  globs declared in a NON-root workspace file (a nested `pnpm-workspace.yaml`) are
  resolved against the repo root instead of the declaration file's directory, so
  they match nothing; pre-join such globs with the declaration file's parent. A
  related lower-priority guard covers a `standalone_rust_workspaces` entry that
  ends in `Cargo.toml`. Pairs with spec 025 (the severity policy); these are the
  mechanical half of the 0.6.0 indexer work.
---

# 026: Indexer resolution + discovery fixes

Filed off the OAP spec-217 Phase-0 dry run of the published 0.5.0 library against
its 220-spec corpus. After separating the adopter's own config and corpus issues,
three residual diagnostics were library defects in the resolution (`sections.rs`)
and discovery (`manifest.rs`) paths. This spec owns the fixes. It is the
mechanical companion to spec 025 (the unresolved-unit severity policy); the two
are split so each disposition stands on its own.

## 1. Purpose

Each defect makes the indexer report a false negative (a unit that should resolve,
or a package that should be discovered, silently does not):

- **D1 (foreign-YAML regions).** `enumerate_sections` routes any non-workflow
  `.yml` / `.yaml` to `ci_job_sections` (which only finds a `jobs:` block), never
  to `region_sections`. A `# region: <name>` marker in a Helm `values.yaml` or an
  infra manifest is therefore invisible, and a spec that declares a section unit
  on it gets a spurious `I-006` ("section unit not found"). This contradicts spec
  022 §3.2, which already states foreign structured configs "keep whole-file /
  `region:` ownership"; the code shipped only the whole-file half, deferring the
  region half as §4's D4. The marker exists on disk; only the dispatch is wrong.

- **D2 (Makefile tag overwrite).** `makefile_sections` sets
  `pending_tag = Some(...)` unconditionally on each `## tag:` line. If a `## tag:`
  is followed by variable-only lines (no target) and then a second `## tag:`, the
  first tag is clobbered before any target consumes it, so its region is never
  emitted: a silent, position-dependent loss, which is unpredictable behavior, not
  a documented limitation.

- **D3 (nested-workspace glob base).** `discover_npm` pushes the raw member globs
  from a workspace declaration file into the resolve set, and `glob_manifests`
  joins them against `repo_root`. For a declaration file that is **not** at the
  repo root (e.g. `product/pnpm-workspace.yaml` declaring `["apps/*"]`), the globs
  must resolve against the file's parent (`product/`), not the root, or they
  expand to `apps/*` at the root and match nothing. The library's own config doc
  comment references the related "template-encore" bug; this is the same class,
  not fully fixed.

## 2. Territory

This spec amends the two owning specs' contracts in place via edge (it does not
rewrite their bodies): spec 022's foreign-config ownership (D1 fulfils §3.2 and
retires the §4 D4 deferral) and spec 004's Makefile-section parsing (D2) and
manifest discovery (D3). It additively claims, alongside 004, the files it edits:

- `crates/spec-spine-core/src/sections.rs`: the `enumerate_sections` dispatch (D1)
  and `makefile_sections` tag handling (D2), with their inline `#[cfg(test)]`
  fixtures.
- `crates/spec-spine-core/src/manifest.rs`: the glob base join in `discover_npm`
  (D3) and the `standalone_rust_workspaces` guard.
- `crates/spec-spine-core/tests/index.rs`: end-to-end fixtures (a foreign YAML
  with a `# region:` resolving a section unit; a nested `pnpm-workspace.yaml`
  discovering its members).

No schema, DTO, or `INDEX_SCHEMA_VERSION` change: these are behavioral
corrections within the existing grammar (the 0.6.0 schema minor is spec 025's).

## 3. Behavior

### 3.1 D1: foreign YAML resolves region markers

`enumerate_sections` MUST route a `.yml` / `.yaml` file that is **not** a governed
workflow (spec 022's `is_workflow_path` predicate is false) to `region_sections`
with the `#` comment token, so `# region: <name>` ... `# endregion` markers
resolve. Governed workflow files keep the spec-022 keypath grammar (a strict
superset of bare-`jobs.<name>`), unchanged. A foreign YAML that happens to contain
a top-level `jobs:` block but no region markers MUST NOT regress: the
implementation SHOULD fall back to (or union with) `ci_job_sections` so any
previously resolvable bare-job anchor still resolves. (In practice foreign YAML
with a `jobs:` block and a declared job section unit is rare-to-nonexistent, but
the fix must not silently drop it.)

### 3.2 D2: a pending Makefile tag is never silently lost

`makefile_sections` MUST NOT discard an unconsumed `## tag:` when a second
`## tag:` is seen. The pending tag MUST be resolved deterministically: a second
`## tag:` before any target MUST NOT silently overwrite the first; instead the
prior pending tag is preserved (emitted against the next consuming target, or
retained alongside the new one) so behavior does not depend on intervening
non-target lines. The existing `# BEGIN <name>` / `# END` explicit region keeps
its verbatim-name semantics (the name is exactly what follows `# BEGIN `); this
spec does not change it.

### 3.3 D3: nested-workspace globs resolve against the declaration file

When `discover_npm` reads member globs from a workspace declaration file at a
non-root path, it MUST pre-join each glob with the declaration file's parent
directory (relative to the repo root) before resolving, so a nested
`pnpm-workspace.yaml` (or nested `package.json#workspaces`) resolves its members
relative to itself. `standalone_npm_packages` globs are repo-root-relative by
contract and MUST remain so (the fix applies only to declaration-file-derived
globs). Lower priority: a `standalone_rust_workspaces` entry that ends in
`Cargo.toml` currently yields `.../Cargo.toml/Cargo.toml` and discovers nothing;
`discover_rust` SHOULD accept either a directory or a `Cargo.toml` path, or emit a
clear diagnostic rather than silently discovering nothing.

## 4. Functional requirements

- **FR-001 (D1).** A non-workflow YAML file's `# region: <name>` markers MUST
  resolve as section anchors; a section unit declared on one MUST NOT raise
  `I-006` when the marker is present.
- **FR-002 (D1 non-regression).** Governed workflow YAML keeps the spec-022
  keypath grammar unchanged; a foreign YAML with a bare `jobs:` block MUST NOT
  lose previously resolvable job anchors.
- **FR-003 (D2).** No `## tag:` is silently dropped: a tagged Makefile region MUST
  be emitted regardless of intervening non-target lines or a later `## tag:`.
- **FR-004 (D3).** Member globs from a non-root workspace declaration file MUST
  resolve relative to that file's directory; `standalone_npm_packages` globs MUST
  stay repo-root-relative.
- **FR-005 (D3 guard, SHOULD).** A `standalone_rust_workspaces` entry ending in
  `Cargo.toml` SHOULD resolve (directory-or-manifest) or emit a clear diagnostic,
  never silently discover nothing.
- **FR-006 (no schema change).** No DTO, schema file, or `INDEX_SCHEMA_VERSION`
  change; behavior corrections within the existing grammar only.

## 5. Acceptance criteria

- **AC-1 (D1).** A fixture with a plain `values.yaml` containing
  `# region: foo` ... `# endregion` and a spec section unit `{ file: values.yaml,
  anchor: foo }` resolves to the marker's span, zero `I-006`.
- **AC-2 (D1 non-regression).** A `.github/workflows/ci.yml` still resolves its
  keypath/job anchors unchanged.
- **AC-3 (D2).** A Makefile with `## tag: a` then variable-only lines, then
  `## tag: b` then a target, emits regions for **both** `a` and `b` (the first is
  not lost).
- **AC-4 (D3).** A fixture with `product/pnpm-workspace.yaml` declaring
  `["apps/*"]` and a package at `product/apps/web/package.json` discovers `web`;
  the same globs do not spuriously match a root-level `apps/`.
- **AC-5 (self-corpus determinism).** spec-spine's own committed shards are
  unchanged by these fixes (its corpus has no foreign-YAML region units, no
  multi-`## tag:` Makefile, no nested workspace), so the only committed-artifact
  delta is 026's own two new shards.

## 6. Out of scope

- **The severity policy** (lifecycle / edge-type warning tiers): spec 025.
- **Any schema or `INDEX_SCHEMA_VERSION` change** (FR-006).
- **New unit kinds or new config keys.** D3 reuses the existing `npm_workspaces`
  declaration mechanism; it does not add configuration.
- **The package version bump and release** (0.6.0): separate release steps once
  025 and 026 are on `main`.
