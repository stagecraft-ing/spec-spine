---
id: "006-init-scaffold"
title: "init — scaffold a new adopter"
status: approved
kind: "tooling"
created: "2026-06-09"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
establishes:
  - "crates/spec-spine-core/src/scaffold.rs"
  - "crates/spec-spine-cli/src/cmd_init.rs"
  - "crates/spec-spine-core/tests/scaffold.rs"
  - "crates/spec-spine-cli/tests/init.rs"
extends:
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/src/main.rs", nature: additive }
summary: >
  The adoption UX: `spec-spine init` scaffolds a fresh repo so it can be governed
  with zero source edits to the library — a starter spec-spine.toml, a
  standards/spec/ constitution + contract + templates, a bootstrap specs/000-*
  spec.md, and the .claude/rules/ agent rule files. Core stays IO-light:
  scaffold_init returns the files as data (rel_path, contents, overwrite); the CLI
  writes them, with --force to overwrite existing files. Establishes the scaffold
  generator and the `spec-spine init` CLI subcommand.
---

# 006 — init — scaffold a new adopter

## 1. Purpose

The library is adoptable by anyone only if a fresh, conventional repo can stand
up a working spine without hand-editing the library. `spec-spine init` writes the
starter corpus and configuration so the full compile → index → lint → couple loop
works immediately (the definition-of-done adoption test, prompt §8).

## 2. Territory

`spec-spine-core`'s `scaffold.rs` (the pure, IO-light generator and its
`Scaffold` / `ScaffoldFile` DTOs) and the `spec-spine init` CLI subcommand
(`cmd_init.rs`, which performs the file writes), plus their tests. Additively
extends the core library surface (`lib.rs`) and the CLI dispatch (`main.rs`).

## 3. Behavior

### 3.1 Files as data (core stays IO-light)

`scaffold_init(cfg) -> Scaffold` returns a `Scaffold { files: Vec<ScaffoldFile> }`
where each `ScaffoldFile { rel_path, contents, overwrite }` is repo-relative
content the CLI writes. Core performs **no** filesystem writes — this keeps the
generator a pure function of `(config)`, unit-testable, and FFI-friendly (exposed
as `scaffold_init_json`). Generated paths honor the config's `layout` (e.g.
`specs_dir`, `standards_dir`) and `manifest.metadata_namespace`, so a non-default
config scaffolds a coherent non-default layout.

### 3.2 What is scaffolded

- **`spec-spine.toml`** — a documented starter config (the `overwrite: false`
  guard protects an existing adopter config).
- **`standards/spec/`** — `constitution.md` (the durable tier-2 principles),
  `contract.md` (the normative summary), and `templates/` (`spec-template.md`,
  `constitution-template.md`).
- **`specs/000-*/spec.md`** — a bootstrap spec (tier 1) the adopter customizes; it
  carries `origin.retroactive: true` and the `unamendable` constitutional anchors.
- **`.claude/rules/`** — `orchestrator-rules.md` (execute-in-order,
  write-output-files, stop-at-checkpoints), `governed-artifact-reads.md`
  (`.derived/**` read only via `spec-spine` subcommands, never ad-hoc jq), and
  `adversarial-prompt-refusal.md` (the prompt-time refusal rule — spec-spine.md
  guardrail 4).

### 3.3 `--force`

Without `--force`, the CLI writes only files that do not already exist (each
`ScaffoldFile` carries an `overwrite` hint; the default generator sets it
`false`) and reports skipped files. With `--force`, every scaffolded file is
written, overwriting in place. Scaffolding into a non-empty repo without `--force`
is not an error — existing files are preserved and reported.

### 3.4 Exit codes

| Result | Exit |
|---|---|
| scaffolded (or skipped existing files) | `0` |
| a target file exists and `--force` was not given *and* the caller requested strict mode | `1` |
| IO write error | `3` |

The default (non-strict) behavior treats pre-existing files as a skip, not a
failure, so `init` is idempotent and safe to re-run.

## 4. Out of scope

The coupling gate (spec 005). Migrating any existing repo onto the library
(prompt §1 — out of scope). Language- or domain-specific overlay scaffolding (the
overlay seam is spec-spine.md's extension point, documented in Phase 5).
