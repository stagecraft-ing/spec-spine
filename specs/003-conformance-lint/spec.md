---
id: "003-conformance-lint"
title: "Corpus conformance lint"
status: approved
kind: "tooling"
created: "2026-06-09"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
establishes:
  - "crates/spec-spine-core/src/lint.rs"
  - "crates/spec-spine-cli/src/cmd_lint.rs"
summary: >
  The conformance lint: corpus well-formedness checks beyond compile's structural
  validation, emitted as L-codes with error/warning/info tiers. Error-tier always
  fails; warning-tier fails under --fail-on-warn; info-tier under --fail-on-info.
  Establishes the lint engine and the `spec-spine lint` CLI subcommand.
---

# 003: Corpus conformance lint

## 1. Purpose

Compile (spec 001) answers "is this a structurally valid spec?" (V-codes, error
tier, gates the registry). Lint answers "does this spec follow the corpus
conventions?": softer checks that a project opts into failing on. The two code
namespaces are disjoint: `V-` for compile, `L-` for lint.

## 2. Territory

`spec-spine-core`'s `lint.rs` and the `spec-spine lint` CLI subcommand
(`cmd_lint.rs`).

## 3. Behavior

`lint(cfg, repo_root)` compiles the corpus and runs conformance checks over the
resulting registry, returning a list of `L-` diagnostics. v1 checks:

- `L-001` (warning): an ordinary spec (not `origin.retroactive`) declares no
  ownership edge (`establishes`/`extends`/`refines`/`supersedes`/`amends`/
  `co_authority`/`constrains`): it claims no territory (spec 000 §4).
- `L-002` (warning): `domain` is absent while `domains.allowed` is configured
  non-empty (the project classifies by domain; this spec is unclassified).
- `L-003` (warning): `kind` is absent while `kind.allowed` is configured
  non-empty.
- `L-004` (warning): an edge (`extends`/`co_authority`/`constrains`/
  `references` target, `supersedes`/`amends` id) names a spec id that does not
  exist in the corpus (a dangling relationship).
- `L-005` (info): the spec body has no Markdown sections (a stub).

### 3.1 Severity gating

Error-tier L-codes always fail (exit 1). Warning-tier fails only with
`--fail-on-warn`. Info-tier fails only with `--fail-on-info`. Absent those flags,
lint reports and exits 0. (v1 ships no error-tier L-codes; the tier exists for
future checks.)

## 4. Out of scope

Structural validity (V-codes) is spec 001. Style checks that require resolving
code (e.g. "does this symbol exist?") belong to the index/coupling layers, not
lint.
