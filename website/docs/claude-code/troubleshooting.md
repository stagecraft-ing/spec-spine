---
id: troubleshooting
title: Troubleshooting and FAQ
sidebar_position: 10
---

# Troubleshooting and FAQ

Common issues when running the kit, with causes and fixes.

## `spec-spine compile` fails

**Symptom:** compile exits with an error about invalid frontmatter, missing
required fields, or relationship-graph violations.

**Cause:** a `specs/*/spec.md` file has invalid YAML frontmatter (missing `id` /
`title` / `status`, an invalid edge type, or a reference to a spec id that does
not exist).

**Fix:** read the error (it names the file and line), check the
[Edges and units](../concepts/edges-and-units.md) grammar, fix the YAML, re-run.

## "Index is stale"

**Symptom:** `spec-spine index check` exits non-zero; the SessionStart hook
reports `codebase index: STALE`.

**Cause:** a hashed input changed since the last index build. The content hash
covers everything in `spec-spine.toml [index] extra_hashed_inputs` plus the
always-hashed core (spec files and discovered manifests). Common triggers: a
`spec.md` edit, a manifest change, a `.claude/**` edit, a workflow change, or an
edit to `spec-spine.toml` itself.

**Fix:**
```bash
spec-spine index
git add .derived/codebase-index/   # if .derived/ is committed in your repo
```

## "Coupling gate failed"

**Symptom:** `spec-spine couple` exits non-zero, reporting drift on one or more
owned paths.

**Cause:** you changed a path that a spec owns (via its `establishes` /
`extends` / `refines` / `co_authority` edges) without the owning `spec.md` being
in the same diff, or vice versa. A path is cleared only when an owner's `spec.md`
also changed.

**Fix (preferred):** include the owning `spec.md` in the change so the spec and
its code move together; if a new path should be owned, add the appropriate edge
to a spec. **Fix (waiver):** include a `Spec-Drift-Waiver` block in the PR body
at creation time (requires explicit user approval; the agent never adds it
alone). Paths on the bypass floor (`.github/**`, `docs/**`, lockfiles) and any
`[coupling] bypass_prefixes` in `spec-spine.toml` are exempt; unowned paths clear
automatically.

## "pr-prep regenerated `.derived/`"

**Symptom:** after the pre-PR gate, `git status` shows changes under `.derived/`.

**Cause:** the index was stale and got regenerated.

**Fix:** stage and commit (or amend) the regenerated index. This is expected,
not an error; the PreToolUse hook blocks PR creation while it is uncommitted.

## `/init` reports "Codebase index: not built"

**Symptom:** the init summary omits structural counts.

**Cause:** `spec-spine index` has never run on this clone, or the shards were
deleted.

**Fix:** `spec-spine index` (or `/setup` for a full bootstrap).

## `/setup` fails installing spec-spine

**Symptom:** setup fails at the install step.

**Cause:** the chosen install method's prerequisite is missing (a Rust
toolchain for `cargo install`, Node for `npm`, Python for `pip`).

**Fix:** install the prerequisite, or switch to a prebuilt method (`npm i -D
spec-spine`, `pip install spec-spine`), then re-run.

## Agent violates a checkpoint

**Symptom:** the agent opens a PR or pushes without waiting for confirmation.

**Cause:** `orchestrator-rules` is not being loaded.

**Fix:** verify `.claude/rules/orchestrator-rules.md` exists and that your
`AGENTS.md` New Sessions protocol loads it in Step 0.

## A declared MCP server will not connect

**Symptom:** the agent cannot reach a server declared in `.mcp.json`.

**Cause:** the binary at the declared path is missing or not executable.

**Fix:** build or install it, or remove the entry if you do not need it (the kit
ships an empty `.mcp.json`).
