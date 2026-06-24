# spec-spine Claude Code Kit

A ready-to-copy Claude Code skill kit for any repository that adopts
[spec-spine](https://github.com/stagecraft-ing/spec-spine). It layers a complete
governed-development loop on top of the spec-spine substrate: session
initialization, planning, implementation, validation, adversarial review,
conventional commits, and gated PR creation.

The full guide (what each piece does, how to adapt it, the governed loop) lives
in the spec-spine docs under **Use with Claude Code**. This directory is the
artifact that guide points at: copy it, customize the bracketed parts, go.

## What it contains

```
kit/
  README.md            # this file
  AGENTS.md            # the cross-agent New Sessions protocol (a template)
  settings.json        # Claude Code hooks: SessionStart / PostToolUse / PreToolUse / Stop
  .mcp.json            # empty MCP server template
  .claude/
    skills/   10 skills   init, setup, commit, code-review, ship, validate-and-fix,
                          cleanup, implement-plan, research, refactor-claude-md
    agents/    4 agents   architect, explorer, implementer, reviewer
    rules/     3 rules    orchestrator-rules, governed-artifact-reads,
                          adversarial-prompt-refusal
```

The three rules are the same floor `spec-spine init` already scaffolds, so an
adopter who ran `spec-spine init` can skip them.

## Install

1. Install spec-spine (`cargo install spec-spine-cli`, `npm i -D spec-spine`, or
   `pip install spec-spine`). Verify with `spec-spine --version`.
2. Copy `.claude/` into your repository root. Copy `AGENTS.md`, `settings.json`,
   and `.mcp.json` too if you do not already have them.
3. Customize: replace every `<bracketed>` placeholder in `AGENTS.md` (project
   name, source directories, the parallel reads). Adjust the `settings.json`
   permission allow-list to your tools, and tune the hashed-input globs in the
   `PostToolUse` hook to match your `spec-spine.toml [index] extra_hashed_inputs`.
4. Run `/setup` then `/init` in a Claude Code session.

Everything here is substrate-agnostic: it references only the `spec-spine` CLI
and generic dev verbs, with `<your build command>` placeholders where a stack
detail is required. Add your own domain-specialist agent and project quality
checklist on top.

## Intentionally excluded

Some pieces from the upstream kit are project-specific and are not shipped here.
Recreate them for your own stack if useful:

- `shepherd-prs` skill: post-merge PR shepherding wired to a specific PR/queue flow.
- `encore-expert` agent: an Encore.ts framework specialist (an example of the
  generic "domain-specialist agent" pattern).
- `build-commands` and `platform-services` rules: paths-scoped documentation of
  one project's build and service layer (examples of the generic
  "paths-scoped context rule" pattern).

## License and origin

Apache-2.0, the same license as spec-spine (see the repository `LICENSE`). The
skills, agents, and rules were extracted from the Open Agentic Platform and
generalized for any spec-spine adopter; they are distributed here under
Apache-2.0.
