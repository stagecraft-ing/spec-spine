---
id: adopt-in-your-repo
title: Adopt in Your Repo
sidebar_position: 9
---

# Adopt in Your Repo

A numbered path from a generic repository to a governed one. Each step builds on
the previous; the end state is a working governed-development loop.

## Step 1: Install spec-spine

```bash
cargo install spec-spine-cli   # or npm i -D spec-spine, or pip install spec-spine
```

Verify with `spec-spine --version`.

## Step 2: Configure `spec-spine.toml`

Run `spec-spine init` to scaffold it, or create it by hand at the repo root.
Declare your namespace, domain and kind taxonomies, layout, the index hashed
inputs, and the coupling bypass and waiver keyword. The
[Configuration reference](../configuration.md) covers every key.

## Step 3: Create your first spec

```bash
mkdir -p specs/001-initial-governance
```

Write `specs/001-initial-governance/spec.md` with YAML frontmatter declaring the
authority relationships for your core files. See
[Edges and units](../concepts/edges-and-units.md) for the grammar.

## Step 4: Copy the kit

Copy `kit/.claude/` into your repo root (see [Install the kit](./install.md)).
Add `AGENTS.md`, `settings.json`, and `.mcp.json` if you do not have them.

## Step 5: Write the `AGENTS.md` New Sessions protocol

Fill in the bracketed placeholders the template ships: the project name, your
source directories, and the parallel reads your init should perform. Keep the
structure (load rules, compile, parallel reads, emit summary).

## Step 6: Provide the build targets

Add `setup`, `ci`, and `pr-prep` (or your equivalents) so `/setup`,
`/validate-and-fix`, and `/ship` have something to call. See
[Configuration](./configuration.md#a-local-ci-command).

## Step 7: Run the loop

```bash
/setup        # verify the bootstrap
/init         # load context
```

Then make a small change on a feature branch and run `/ship` end to end. If the
gate passes (or you fix coupling / apply a waiver), `/code-review` produces
findings, `/commit` writes the message, and the PR is created, the loop is
operational.

## Adapt the project-specific pieces

The kit is substrate-agnostic, but a real adoption layers your own stack on top.
Recreate these for your repository (the kit ships the patterns, not the content):

| Piece | What to do |
|---|---|
| Domain-specialist agent | Add `.claude/agents/<framework>-expert.md` if a stack benefits from one (see [Agents](./agents.md)). |
| Paths-scoped context rule | Add `.claude/rules/<area>.md` with `paths:` frontmatter documenting a directory's conventions (see [Rules](./rules.md)). |
| Quality checklist | Layer a post-feature checklist (framework invariants, route/DTO alignment, auth scoping, env coverage) into `/validate-and-fix`; keep it in your repo. |
| `spec-spine.toml` taxonomies | Replace example domain and kind enums with your own. |
| Permission allow-list | Rewrite `settings.json` `permissions.allow` for your tools. |
| MCP servers | Declare your own in `.mcp.json`, or leave it empty. |
| Post-merge automation | Recreate a `shepherd-prs`-style flow for your PR/queue setup if you want it. |

The general test: if a file references a concrete framework, service, path, or a
specific spec id, generalize it or replace it with your equivalent.
