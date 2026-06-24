---
id: overview
title: Overview and Philosophy
sidebar_position: 1
---

# Use with Claude Code

This section is an adoption guide. It teaches you how to drive spec-spine with
[Claude Code](https://claude.ai/code) using a ready-made kit: a complete set of
skills, agents, and rules that chain into a single governed development loop.

The kit lives in this repository under [`kit/`](https://github.com/stagecraft-ing/spec-spine/tree/main/kit).
It is Apache-2.0, substrate-agnostic, and built to be copied into any repository
that adopts spec-spine. These pages explain what each piece does and how to
adapt it; the files themselves are in `kit/.claude/`.

## What you are adopting

On top of the spec-spine substrate, the kit adds:

- **10 skills** that cover the development lifecycle: session init, planning,
  implementation, validation, adversarial review, conventional commits, and
  gated PR creation.
- **4 agents** for the plan / explore / implement / review cycle.
- **3 rules** that constrain how every workflow reads artifacts and resolves
  spec/code disagreement (the same floor `spec-spine init` scaffolds).
- **Config templates**: `AGENTS.md`, `settings.json` (hooks), `.mcp.json`.

The kit is not a monolith. Every piece carries a portability verdict so you know
what to copy as-is, what to adapt, and what is an example of a pattern you
recreate for your own stack.

## Why spec-spine is the substrate

[spec-spine](../getting-started/installation.md) is a typed, hash-verifiable
authority ledger over a markdown spec corpus. It gives the kit three things to
build on:

1. **The compiler** turns `specs/NNN-slug/spec.md` files into a governed
   registry of authority relationships.
2. **The indexer** produces a structural view of the codebase with per-shard
   staleness detection driven by content hashing.
3. **The [coupling gate](../concepts/coupling-gate.md)** refuses code that drifts
   from its owning spec at PR time.

Several skills (`/init`, `/setup`, `/ship`) depend directly on the spec-spine
CLI and its governed-read discipline. The three rules are scaffolded by
`spec-spine init`, so installing spec-spine gives you the rule floor for free.

## Portability taxonomy

Every skill, agent, and rule carries one of three verdicts:

| Verdict | Meaning | Action |
|---|---|---|
| **Portable** | Works in any repository without modification | Copy as-is |
| **Adaptable** | Needs light configuration (a build command, a path) | Copy and edit |
| **Project-Specific** | Wired to one stack (a framework, a service, specific spec IDs) | Skip, or replace with your equivalent |

The kit ships only the Portable and Adaptable pieces. The Project-Specific
examples from the upstream source (a framework specialist agent, paths-scoped
service rules, a post-merge PR flow) are documented as patterns to recreate, not
shipped. See [Adopt in your repo](./adopt-in-your-repo.md).

## Next step

Continue to [Install the kit](./install.md).
