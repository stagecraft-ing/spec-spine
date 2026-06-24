---
id: install
title: Install the Kit
sidebar_position: 2
---

# Install the Kit

Two steps: install the spec-spine substrate, then copy the kit into your
repository and customize it.

## 1. Install spec-spine

The kit drives the `spec-spine` CLI, so install it first by whichever method
fits your environment (full detail on the
[installation page](../getting-started/installation.md)):

```bash
cargo install spec-spine-cli            # from crates.io (needs a Rust toolchain)
npm i -D spec-spine                      # in a TS/JS repo (prebuilt binary)
pip install spec-spine                   # or: uvx spec-spine  (Python repo)
```

Verify with `spec-spine --version`. If your repository has no spec corpus yet,
run `spec-spine init` to scaffold `spec-spine.toml`, `standards/`, a first spec,
and the three core rules.

## 2. Copy the kit

The kit is the [`kit/`](https://github.com/stagecraft-ing/spec-spine/tree/main/kit)
directory in the spec-spine repository. Copy its `.claude/` tree into your
repository root, plus the config templates if you do not already have them:

```bash
# from a checkout (or download) of the spec-spine repo:
cp -r path/to/spec-spine/kit/.claude     .
cp    path/to/spec-spine/kit/AGENTS.md   .   # if you have none
cp    path/to/spec-spine/kit/settings.json .claude/settings.json
cp    path/to/spec-spine/kit/.mcp.json   .   # if you have none
```

| Item | Verdict | What you do |
|---|---|---|
| `.claude/skills/` (10) | Portable / Adaptable | Copy. `init`, `setup`, `ship`, `validate-and-fix` are adaptable: point them at your install method and gate. |
| `.claude/agents/` (4) | Portable | Copy architect, explorer, implementer, reviewer as-is. |
| `.claude/rules/` (3) | Portable | Copy, or skip if `spec-spine init` already scaffolded them. |
| `AGENTS.md` | Adaptable | Rewrite the "New Sessions" section for your repo (see [Session init](./session-init.md)). |
| `settings.json` | Adaptable | Rewrite the permission allow-list; keep the hooks (see [Configuration](./configuration.md)). |
| `.mcp.json` | Adaptable | Empty by default; declare your own MCP servers if you have any. |

## 3. Customize

Replace every `<bracketed>` placeholder the kit ships with:

- In `AGENTS.md`: the project name, your source directories, and the parallel
  reads your init should perform.
- In `settings.json`: the `permissions.allow` entries for your tools, and the
  hashed-input globs in the `PostToolUse` hook so they match your
  `spec-spine.toml [index] extra_hashed_inputs`.
- In the adaptable skills: the install command (`/setup`) and any `<your build
  command>` / `<your test command>` placeholders (`/validate-and-fix`,
  `/implement-plan`).

## 4. First run

```bash
/setup        # installs spec-spine if needed, verifies the governed loop
/init         # loads context via the AGENTS.md New Sessions protocol
```

If both produce a clean summary, the kit is operational. Continue to
[Session init](./session-init.md), or jump to
[Adopt in your repo](./adopt-in-your-repo.md) for the full step-by-step.
