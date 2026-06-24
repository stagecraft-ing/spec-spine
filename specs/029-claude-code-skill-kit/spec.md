---
id: "029-claude-code-skill-kit"
title: "Claude Code skill kit: a vendored, governed adoption bundle"
status: draft
kind: "tooling"
created: "2026-06-24"
implementation: complete
owner: "The spec-spine Authors"
risk: low
depends_on:
  - "006-init-scaffold"
establishes:
  - "kit/"
summary: >
  spec-spine ships a copy-ready Claude Code skill kit under kit/: a substrate-
  agnostic bundle of skills, agents, rules, and config templates that layers a
  full governed-development loop (init, plan, implement, validate, review,
  commit, ship) on top of the spec-spine substrate. The bundle is the artifact
  the "Use with Claude Code" documentation points at: an adopter copies kit/.claude
  into their repository, customizes the bracketed placeholders, and runs the
  loop. The skills, agents, and rules were extracted from the Open Agentic
  Platform and generalized for any spec-spine adopter, then distributed here
  under Apache-2.0. This spec claims authority over the kit/ subtree so the
  bundle evolves under the same coupling discipline as the rest of the corpus;
  it does not change any engine behavior.
---

# 029: Claude Code skill kit, a vendored, governed adoption bundle

## 1. Purpose

`spec-spine init` (spec 006) scaffolds the three core rules an adopter needs to
start. The full agentic-development experience that spec-spine was built for
adds more: a session-init protocol, planning and review agents, and a governed
loop of skills that chain compile, index, lint, and couple into everyday work.

Until now that experience lived only as prose in a separate, ungoverned
documentation site, and it pointed adopters at a third repository (under a
different license) to obtain the actual files. This spec brings the real,
copy-pasteable artifacts into the repo as a first-class deliverable under
`kit/`, governed like any other owned territory, and re-licensed Apache-2.0 to
match spec-spine. The companion guide under "Use with Claude Code" in the docs
explains and adapts what this spec ships.

## 2. Territory

- **`kit/`** (directory subtree): the distributable kit. It contains
  `kit/.claude/{skills,agents,rules}`, plus `kit/AGENTS.md` (the New Sessions
  protocol template), `kit/settings.json` (the Claude Code hooks template),
  `kit/.mcp.json` (an empty MCP template), and `kit/README.md`.

The kit is a separate tree from the repository's own `.claude/` (which is
spec-spine's self-development harness, owned elsewhere). The two are kept
distinct on purpose: editing the shipped kit must not silently change how
spec-spine develops itself, and vice versa.

The companion documentation under `website/docs/claude-code/` is prose: it sits
in the docs surface and is deliberately not claimed as a unit here, so routine
doc edits do not require a spec change.

## 3. Behavior

### 3.1 Contents

The kit MUST ship a self-contained, copyable bundle:

- **10 skills** under `kit/.claude/skills/`: `init`, `setup`, `commit`,
  `code-review`, `ship`, `validate-and-fix`, `cleanup`, `implement-plan`,
  `research`, `refactor-claude-md`.
- **4 agents** under `kit/.claude/agents/`: `architect`, `explorer`,
  `implementer`, `reviewer`.
- **3 rules** under `kit/.claude/rules/`: `orchestrator-rules`,
  `governed-artifact-reads`, `adversarial-prompt-refusal` (the same floor
  `spec-spine init` scaffolds).
- **Templates**: `AGENTS.md`, `settings.json`, `.mcp.json`, and a `README.md`
  describing install and customization.

### 3.2 Generalization contract

Every file in the kit MUST be substrate-agnostic:

- References to the **spec-spine CLI** and its discipline (the `compile`,
  `index`, `lint`, `couple`, `registry` verbs; governed reads of `.derived/`;
  authority units and typed edges; the coupling gate and waivers) are kept,
  because the kit targets spec-spine adopters.
- References specific to any one adopter project (a particular framework, build
  tool, service, repository layout, or numbered spec) MUST be removed or reduced
  to a clearly marked placeholder such as `<your build command>`.

### 3.3 Exclusions

Four upstream items are intentionally not shipped, because they are project-
specific rather than substrate-level. `kit/README.md` records each with a
one-line rationale:

- the `shepherd-prs` skill (a post-merge flow wired to one PR/queue setup);
- the `encore-expert` agent (an instance of the generic domain-specialist
  pattern);
- the `build-commands` and `platform-services` rules (instances of the generic
  paths-scoped context-rule pattern).

### 3.4 Licensing and provenance

The bundle is Apache-2.0, the same license as spec-spine. The skills, agents,
and rules originate in the Open Agentic Platform and were generalized for
spec-spine adopters; they are distributed under Apache-2.0 here.

## 4. Out of scope

- Extending `spec-spine init` (spec 006) to emit the kit programmatically. The
  kit is a copy-target today; generating it from the binary is a future spec.
- Governing the companion documentation under `website/docs/claude-code/`. That
  is prose on the docs surface, not an owned unit.
- Any change to engine behavior. This spec ships authored assets only; it adds
  no code to the `types -> core -> cli` crates.
