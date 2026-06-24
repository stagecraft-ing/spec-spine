---
id: configuration
title: Configuration, Hooks, and Customization
sidebar_position: 8
---

# Configuration, Hooks, and Customization

The files that make the loop work: `settings.json`, `.mcp.json`, `AGENTS.md`, a
local CI command, and an optional pre-commit hook.

## `settings.json`

Controls permissions and hooks. The kit ships a working template in
`kit/settings.json`.

```json
{
  "permissions": {
    "allow": ["Bash(spec-spine *)", "Bash(make *)", "..."],
    "deny": ["Bash(npm publish*)", "Bash(gh repo delete *)", "..."]
  },
  "hooks": {
    "SessionStart": [],
    "PostToolUse": [],
    "PreToolUse": [],
    "Stop": []
  }
}
```

Never commit secrets in this file.

### Permissions

`allow` whitelists tool invocations the agent may run without asking; `deny`
blocks destructive operations (publishing, deleting repos, creating releases).
Rewrite both lists for your tool paths.

### Hooks

The hooks are the deterministic safety net. They call the `spec-spine` CLI
directly, so they work in any repo that has it on `PATH`.

| Hook | Matcher | What it does |
|---|---|---|
| `SessionStart` | startup/resume/clear/compact | Recompiles the registry and reports registry + index freshness. |
| `PostToolUse` | `Edit\|Write` | After a `spec.md` edit, recompiles; after any hashed-input edit, runs `index check`. |
| `PreToolUse` | `Bash` | Intercepts `gh pr create`, runs the coupling gate, and blocks the PR if it fails with no `Spec-Drift-Waiver` in the body. |
| `Stop` | `*` | If the index is stale (and no rebase/merge is in progress), regenerates it via `spec-spine index`. |

To adapt: keep all four if you use spec-spine. Adjust the `PostToolUse` `case`
globs to match your `spec-spine.toml [index] extra_hashed_inputs`, and the
waiver keyword if you changed it.

## `.mcp.json`

Declares Model Context Protocol servers. The kit ships an empty template:

```json
{ "mcpServers": {} }
```

Add your own servers if you have any.

## `AGENTS.md`

Two jobs: its `## New Sessions` section is the protocol `/init` executes, and its
"Available Agents" / "Available Commands" sections document what is on hand.
Rewrite the New Sessions section for your repo (see
[Session init](./session-init.md)).

## A local CI command

`/validate-and-fix` and `/ship` expect one command that runs the same gate set as
CI. A common convention is three Make targets:

- **`make setup`**: install spec-spine, compile the registry, build the index.
- **`make ci`**: your full local gate (the governance verbs plus build,
  type-check, lint, tests).
- **`make pr-prep`**: refresh the index, then run `spec-spine couple` against
  `origin/main`.

The names are a contract with the skills; keep them, or rename and update the
skill references. Every `make ci` must include the four spec-spine verbs
(`compile`, `lint --fail-on-warn`, `index check`, `couple`).

## Optional pre-commit hook

An opt-in git hook that refuses a commit when spec-spine is missing or the index
is stale:

```bash
git config core.hooksPath .githooks      # enable
git config --unset core.hooksPath        # disable
git commit --no-verify                    # emergency bypass
```

Keep the staleness gate if you use spec-spine. The hook is read-only: it never
mutates the working tree.
