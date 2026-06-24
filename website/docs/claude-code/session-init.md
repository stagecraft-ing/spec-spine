---
id: session-init
title: Session Init and Context Loading
sidebar_position: 3
---

# Session Init and Context Loading

Every session begins with `/init`. This skill reads the `AGENTS.md` "New
Sessions" protocol and executes it, loading the context an agent needs to work
effectively in the repository.

## The `/init` skill

`/init` is a thin dispatcher. Its `SKILL.md` contains only three instructions:

1. Read `AGENTS.md`, specifically the section from `## New Sessions` to the next
   `## ` heading.
2. Execute the protocol described there, using parallel tool calls where it says
   "dispatch simultaneously".
3. Emit the structured summary the protocol prescribes.

The skill does not duplicate the step list. `AGENTS.md` is the single source of
truth, read by Claude Code, Codex CLI, Cursor, Copilot, and any future agent.
Evolve the protocol by editing `AGENTS.md`, never the skill.

## The AGENTS.md "New Sessions" protocol

The template the kit ships defines this flow:

### Step 0: Load rules

Read the three core rules every orchestrated workflow depends on:
`orchestrator-rules`, `governed-artifact-reads`, `adversarial-prompt-refusal`.

### Step 1: Refresh the registry, then parallel reads

Run `spec-spine compile` first (so lifecycle counts reflect current spec
frontmatter), then dispatch simultaneously:

- `CLAUDE.md` (project overview and conventions)
- `README.md` (full project description)
- your contract and constitution documents under `standards/`
- `spec-spine index check` (staleness gate, non-fatal)
- `spec-spine registry status-report --json --nonzero-only` (lifecycle counts)
- `spec-spine registry list --ids-only` (spec id list)
- directory listings for your source surfaces
- `git log --oneline -10` and `git diff --stat HEAD~1`

### Step 2: Emit the initialized summary

A structured `## initialized: <project-name>` block: a layer overview, recent
activity, a "ready to help with" line, and a `## lifecycle:` sub-section from the
status report.

## The SessionStart freshness hook

The `settings.json` SessionStart hook fires on every new session and resume. It
reports spec-registry and codebase-index freshness, so the agent knows
immediately whether the index is stale before `/init` even runs:

```
[session-freshness] spec registry: recompiled fresh; codebase index: fresh
```

## Read discipline

Compiled artifacts under `.derived/**` must be read through the consumer CLI
(`spec-spine registry`, `spec-spine index`), never via `jq`, `awk`, `sed`, or
`python` against the JSON shards. This is enforced by the
[`governed-artifact-reads`](./rules.md) rule and is load-bearing: the shard
format is an implementation detail that can change between spec-spine versions.

## Staleness handling

- **Index stale** (`spec-spine index check` exits non-zero): include "Codebase
  index: stale, run `spec-spine index`" in the summary and continue.
- **Index not built**: report "Codebase index: not built" and continue without
  structural counts.

Neither is fatal. The protocol is designed to succeed even on a partially-set-up
clone, surfacing what is missing without halting.

## Adapting for your repository

Write an `AGENTS.md` `## New Sessions` section listing the reads your repo needs.
Keep the structure (load rules, compile, parallel reads, emit summary), and
replace the example reads with your equivalents. The `/init` skill needs no
change: it reads whatever protocol you define.
