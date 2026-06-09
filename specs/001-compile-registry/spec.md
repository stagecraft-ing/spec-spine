---
id: "001-compile-registry"
title: "Compile the spec corpus into a deterministic registry"
status: approved
kind: "tooling"
created: "2026-06-08"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "000-spec-spine-bootstrap"
establishes:
  - "crates/spec-spine-core/src/lib.rs"
  - "crates/spec-spine-core/src/compile.rs"
  - "crates/spec-spine-core/src/hash.rs"
  - "crates/spec-spine-core/src/canonical_json.rs"
  - "crates/spec-spine-core/src/markdown.rs"
  - "crates/spec-spine-cli/src/main.rs"
  - "crates/spec-spine-cli/src/cmd_compile.rs"
summary: >
  The compile capability: read every specs/NNN-slug/spec.md, validate its
  frontmatter, and emit a deterministic spec-as-source registry.json plus a
  non-deterministic build-meta.json. Establishes the spec-spine-core engine
  scaffolding (content hashing, canonical JSON, markdown heading extraction) and
  the `spec-spine compile` CLI subcommand.
---

# 001: Compile the spec corpus into a deterministic registry

## 1. Purpose

Turn the authored markdown corpus into the compiler-owned `registry.json` (the
spec-as-source view), deterministically. This is the first guardrail from spec
000 §7 and the foundation every other capability reads from.

## 2. Territory

`spec-spine-core`'s compile engine and its shared scaffolding: content hashing
(`hash.rs`), canonical JSON serialization (`canonical_json.rs`), markdown heading
extraction (`markdown.rs`), the `compile` entry point (`compile.rs`), and the
crate's public surface (`lib.rs`), plus the `spec-spine compile` CLI subcommand
(`cmd_compile.rs`) and the CLI dispatch frame (`main.rs`).

## 3. Behavior

### 3.1 Discovery and parsing

- `compile(cfg, repo_root)` MUST discover specs at
  `<repo_root>/<layout.specs_dir>/NNN-slug/spec.md`, in sorted order.
- Each spec's frontmatter is parsed via the `spec-spine-types` grammar. A spec
  whose frontmatter cannot be parsed (bad YAML, missing required key, invalid
  enum) is recorded as an **error-tier violation** and skipped; compile does not
  abort the whole corpus on one bad spec. `Err` is reserved for I/O failures.

### 3.2 Validation (V-codes)

The compiler MUST emit these validation codes (error-tier unless noted):

- `V-001` directory name does not equal `id`.
- `V-002` malformed frontmatter (parse failure / missing required key / bad enum).
- `V-003` duplicate spec `id`.
- `V-004` duplicate numeric prefix (`NNN`) across different slugs.
- `V-005` `domain` not in `domains.allowed` (only when that allowlist is non-empty).
- `V-006` `kind` not in `kind.allowed` (only when that allowlist is non-empty).
- `V-007` more than the cap of **undeclared** `extra_frontmatter` keys (keys not
  in `frontmatter.extra_known_keys` are counted; the cap stops escape-hatch abuse,
  ported from OAP's V-002 ~8-entry cap).
- `V-008` `status: superseded` without `superseded_by` resolving to an existing id.
- `V-009` `status: retired` without `retirement_rationale`.
- `V-010` (warning) `depends_on` references a non-existent spec id.
- `V-012` `id` does not match `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*$`.

`validation.passed` is false iff any error-tier violation is present.

### 3.3 The registry record

For each valid spec the compiler builds a `SpecRecord` carrying the frontmatter
fields, the computed `spec_path`, the `section_headings` extracted from the body,
and, critically, **a verbatim copy of `extra_frontmatter`**. The overlay seam
depends on downstream-specific frontmatter reaching `registry.json`; the compiler
MUST NOT drop it.

### 3.4 Determinism

- `registry.json` MUST be a pure function of `(config, file contents)`: specs
  sorted by `id`, object keys sorted, pretty-printed with `\n` and a trailing
  newline. Compiling identical inputs twice MUST be byte-identical.
- `build.content_hash` is SHA-256 over the normalized, path-sorted spec inputs
  (strip BOM, `\r\n`→`\n`, `\r`→`\n`; `<path>\0<content>` pieces).
- The wall clock lives only in `build-meta.json` (`built_at`), written by the
  CLI, and is excluded from every determinism/golden check.

### 3.5 CLI

`spec-spine compile` writes `registry.json` and `build-meta.json` under
`<layout.derived_dir>/spec-registry/`, prints a summary, and exits `0` when
validation passed, `1` when it failed, `3` on I/O / parse / schema error.

## 4. Out of scope

Unit *resolution* (do these paths/symbols exist?) belongs to the indexer (spec
004). The coupling gate (005) and lint (003) consume the registry but are not
defined here.
