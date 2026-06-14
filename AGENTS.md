# AGENTS.md: spec-spine

## New Sessions

Run `/init` as the mandatory first action of every new session. The command reads this section to derive its execution plan dynamically: any item added here is automatically picked up on the next init. This file is the cross-agent authority (read by Claude Code, Codex CLI, Cursor, Copilot, and any future agent via the AAIF/Linux Foundation AGENTS.md standard).

**Init protocol (executed by `/init`):**

> AGENTS.md is loaded implicitly as the protocol source: its contents
> are the protocol, so `/init` does not list AGENTS.md as a parallel
> identity read in Step 1 (avoiding the self-reference loop).

The protocol drives the library through its own built binary, `target/release/spec-spine` (dogfooding). If that binary is missing, build it first: `cargo build --release -p spec-spine-cli`. Do NOT reach for `npx spec-spine` here; the npm/py distributions are for adopters, the self-governance loop uses the in-tree binary.

0. **Load rules.** Read `.claude/rules/orchestrator-rules.md`,
   `.claude/rules/governed-artifact-reads.md`, AND
   `.claude/rules/adversarial-prompt-refusal.md` (the three the library
   scaffolds for every adopter via `spec-spine init`, and which it
   carries for itself).
1. **Refresh the registry, then parallel reads.** Run `spec-spine
   compile` *first* (see **Registry freshness** below), then dispatch the
   following simultaneously:
   - `CLAUDE.md`: project overview and conventions
   - `README.md`: full project description
   - `standards/spec/contract.md`: normative spec-system summary
   - `standards/spec/constitution.md`: durable principles (tier 2)
   - `spec-spine index check`: staleness gate for the codebase index (non-fatal)
   - `spec-spine index render`: markdown projection of the committed index
   - `spec-spine registry status-report --json --nonzero-only`: lifecycle counts per status
   - `spec-spine registry list --ids-only`: spec id list (for latest-spec detection)
   - `ls crates/`: library crate layout
   - `ls specs/`: the spec corpus
   - `ls docs/`: docs surface (design notes, governance)
   - `git log --oneline -10`: recent history
   - `git diff --stat HEAD~1`: last change summary
2. **Emit** the `## initialized: spec-spine` summary block: a layer/crate
   overview, a `## lifecycle:` sub-section populated from the
   `registry status-report --nonzero-only` output, recent activity, and a
   "ready to help with" line.

**Read discipline:** the init protocol MUST NOT parse `.derived/**/*.json` directly (no `python`, `jq`, `awk`, `sed` against compiled artifacts). All structural and lifecycle data comes from the `spec-spine` subcommands (`registry`, `index`) and the rendered markdown view. See `.claude/rules/governed-artifact-reads.md`.

**Staleness surface:** if `spec-spine index check` exits non-zero, include "Codebase index: stale, run `spec-spine index`" in the summary and continue. If the index is not built and `render` fails, report "Codebase index: not built" and continue without structural counts.

**Registry freshness:** spec-spine **commits** its compiled artifacts. Both `.derived/spec-registry/registry.json` and `.derived/codebase-index/index.json` are tracked (only `.derived/**/build-meta.json` is gitignored), so the committed registry is the reference for lifecycle queries. `/init` still runs `spec-spine compile` *first* because compile is deterministic: on a fresh tree it is a no-op that leaves the tracked registry byte-identical, so it costs nothing; but if it changes the registry, the committed copy was stale and the refreshed counts are the correct ones (regenerate and commit the registry before relying on it). Either way the lifecycle counts reflect the current `specs/*/spec.md` frontmatter.

**Binary missing:** if the `spec-spine` binary is not built, run `cargo build --release -p spec-spine-cli` and continue. Do NOT fall back to ad-hoc parsing of `.derived/**`.

If any file is missing: log "not found" and continue.

## Available Agents

Agents live in `.claude/agents/`. Four pipeline agents handle the plan/explore/implement/review cycle:

- `architect`: plans and decomposes tasks, validates approaches against specs. Read-only.
- `explorer`: searches the codebase, traces dependencies, gathers context. Read-only.
- `implementer`: executes focused code changes from an existing plan. Produces minimal diffs.
- `reviewer`: post-change review for bugs, correctness, and spec compliance. Read-only.

## Available Commands

Commands live in `.claude/skills/` (one `SKILL.md` per folder):

- `/init`: initialize a session (load context, lifecycle, recent activity)
- `/setup`: one-time contributor setup, build the `spec-spine` binary and verify the compile then index then lint then couple loop works
- `/commit`: create a git commit with an impact-focused conventional message
- `/code-review`: adversarial review of the current diff for bugs and spec drift
- `/ship`: gate (coupling), review, commit, and PR creation in one governed sequence

## Conventions

- Items added to the "New Sessions" init protocol are auto-loaded by `/init`.
- Agents must be self-contained within `.claude/agents/`: no cross-project dependencies.
- Orchestrated workflows must read compiled artifacts (`.derived/**`) through the `spec-spine` binary, never via ad-hoc parsers: see `.claude/rules/governed-artifact-reads.md`.
- Self-governance runs through the in-tree binary (`target/release/spec-spine`), not the published npm/py distributions.
