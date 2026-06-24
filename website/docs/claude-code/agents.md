---
id: agents
title: Agents Reference
sidebar_position: 6
---

# Agents Reference

The kit ships 4 agents under `kit/.claude/agents/`. They handle the
plan / explore / implement / review cycle. All are portable.

| Agent | Mutation | Role |
|---|---|---|
| `architect` | Read-only | Plans and decomposes tasks; validates approaches against the spec corpus; produces structured work plans. |
| `explorer` | Read-only | Investigates the codebase, traces dependencies, gathers context, answers how-things-work questions. |
| `implementer` | Writes | Executes focused changes from an existing plan, keeping diffs minimal and verifying as it goes. |
| `reviewer` | Read-only | Reviews changes for bugs, correctness, performance, and spec compliance. |

The three read-only agents (architect, explorer, reviewer) gather context and
judgement without touching files; only the implementer mutates, and only from an
approved plan. Each references the `spec-spine` CLI for spec-aware context and
defers spec/code disagreement to the [`adversarial-prompt-refusal`](./rules.md)
rule.

## Add a domain specialist

The upstream source also carried a framework-specialist agent (a read-only agent
that loads a framework's reference docs and enforces its pattern constraints).
That is an instance of a generic, reusable pattern rather than a portable file,
so it is not shipped. To add one for your stack:

1. Create `.claude/agents/<your-framework>-expert.md`.
2. Give it read-only tools.
3. List the reference docs it should load.
4. Document the pattern constraints it enforces and its output format.

The pattern (a specialist that loads docs, examines current state, and proposes
implementations within hard constraints) is fully generic.
