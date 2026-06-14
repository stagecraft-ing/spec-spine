---
name: setup
description: One-time contributor setup. Build the spec-spine binary and verify the governed loop (compile, index check, lint, couple) so `/init` can report lifecycle and structural counts.
allowed-tools: Bash, Read
---

# Setup

Get a fresh clone operational. After this completes, `/init` can report
lifecycle and structural counts through the in-tree binary (no ad-hoc
parsing of `.derived/**/*.json`: see `.claude/rules/governed-artifact-reads.md`).

The self-governance loop dogfoods the library through its own built
binary, `target/release/spec-spine`, not the published npm/py
distributions. Those are for adopters.

## Process

### 1. Build the binary

```bash
cargo build --release -p spec-spine-cli
```

Halt on non-zero exit and surface the failing step verbatim. The build
needs a Rust toolchain (the pinned version lives in `rust-toolchain.toml`).

### 2. Compile a fresh registry

```bash
target/release/spec-spine compile
```

`.derived/spec-registry/registry.json` is a tracked, committed artifact
(spec-spine intentionally does not gitignore `.derived/`; only
`build-meta.json` is ignored). `compile` regenerates it deterministically
from `specs/*/spec.md`, and is a no-op on a fresh tree. Run it before any
read so the registry reflects the working tree, and commit the
regenerated registry whenever `specs/*/spec.md` changes.

### 3. Verify the governed loop

Smoke-test the gates `/init` and CI depend on. Passing here means the
loop works on this clone:

```bash
target/release/spec-spine index check       # codebase index staleness gate
target/release/spec-spine lint               # corpus conformance
target/release/spec-spine couple             # PR-time coupling gate (vs origin/main)
```

If `index check` exits non-zero the committed index is stale against
current inputs. Regenerate and re-commit it via the index merge driver
or rebuild flow, then re-check. Do not parse `.derived/**/*.json`
directly to "verify" success.

### 4. Emit summary

Report exactly:

```
## setup: spec-spine

**Build:** {ok / failed at <step>}
**Governed loop:**
  - compile: {fresh registry / failed}
  - index check: {fresh / stale}
  - lint: {clean / N diagnostics}
  - couple: {clean / drift surfaced}
**Lifecycle:** {N specs across <statuses>}  (from registry status-report)

Next: run `/init` to load full session context.
```

Do not invent counts. Only report values that came back from a
`spec-spine` subcommand.

## Rules

- The build target is `cargo build --release -p spec-spine-cli`. The
  loop runs through `target/release/spec-spine`, never `npx spec-spine`.
- Halt on first failure. Do not silently continue past a missing
  prerequisite or a failing gate.
- Never parse `.derived/**/*.json` directly in any verification step.
  Use the `spec-spine` subcommands.
- Idempotent: safe to re-run. Cargo skips up-to-date crates; `compile`
  is deterministic.
