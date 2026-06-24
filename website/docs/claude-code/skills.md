---
id: skills
title: Skills Reference
sidebar_position: 5
---

# Skills Reference

The kit ships 10 skills under `kit/.claude/skills/`. Each is a `SKILL.md` in its
own folder. Six are portable as-is; four are adaptable (they reference your
install method, gate, or build commands).

| Skill | Verdict | What it does |
|---|---|---|
| `cleanup` | Portable | Dead-code and duplicate detection with categorized recommendations. |
| `code-review` | Portable | Review the working diff for correctness bugs and spec drift, emit an evidence line. |
| `commit` | Portable | Create a git commit with an impact-focused conventional message. |
| `implement-plan` | Portable | Execute a plan file step by step with progress tracking and checkpoints. |
| `refactor-claude-md` | Portable | Tighten and restructure a bloated `CLAUDE.md`, extracting path-scoped rules. |
| `research` | Portable | Deep research with parallel sub-agents and a filesystem artifact protocol. |
| `init` | Adaptable | Run the `AGENTS.md` New Sessions protocol. Adapt via your `AGENTS.md`. |
| `setup` | Adaptable | One-time setup: install spec-spine, verify the governed loop. |
| `validate-and-fix` | Adaptable | Run your local CI loop and fix findings in severity order. |
| `ship` | Adaptable | Gate, review, commit on a feature branch, open a PR. |

## Portable skills

- **`cleanup`** finds dead code and duplication, classifies each finding, and
  refuses to remove spec-owned paths without checking `spec-spine registry show`.
- **`code-review`** runs decorrelated finders over the diff and verifies each
  finding, ending in a `Local-Review-Evidence:` line that `/ship` can carry.
- **`commit`** writes a conventional, impact-focused commit message; it never
  adds AI attribution trailers.
- **`implement-plan`** drives a plan file through a status state machine with
  task-list and midpoint checkpoints.
- **`refactor-claude-md`** restructures an overgrown `CLAUDE.md` into a lean core
  plus path-scoped rules.
- **`research`** fans out parallel sub-agents and synthesizes a cited report.

## Adaptable skills

- **`init`** is a thin dispatcher over your `AGENTS.md` protocol; see
  [Session init](./session-init.md).
- **`setup`** installs spec-spine and smoke-tests `compile`, `index check`,
  `lint`, and `couple`. Point step 1 at your preferred install method.
- **`validate-and-fix`** runs your CI composite (commonly `make ci`), categorizes
  findings (CRITICAL / HIGH / MEDIUM / LOW), fixes them in phases, and supports
  parallel fix agents. Add your own post-feature quality checklist.
- **`ship`** runs the gate (`compile`, `lint`, `index check`, `couple`), invokes
  `/code-review` then `/commit`, and opens a PR at a checkpoint. Adapt the gate
  commands and waiver keyword to your repo.

## Not shipped

`shepherd-prs` (post-merge PR shepherding) is wired to a specific PR/queue setup
and is omitted. Recreate it for your flow if you need post-merge automation.
