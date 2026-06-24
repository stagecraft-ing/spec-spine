---
name: init
description: Initialize a session by executing the cross-agent New Sessions protocol declared in AGENTS.md.
---

# /init: session bootstrap

Thin dispatcher. The canonical protocol lives in **`AGENTS.md` § New Sessions**
under the AAIF/Linux Foundation cross-agent standard (the same file is read by
Claude Code, Codex CLI, Cursor, Copilot, and any future agent).

## What to do

1. Read `AGENTS.md`: the section from `## New Sessions` inclusive to the next
   `## ` heading exclusive. That section is the step list.
2. Execute the protocol described there, using parallel tool calls wherever it
   says "dispatch simultaneously".
3. Emit the structured summary the protocol prescribes (the
   `## initialized: <your-project>` block).

This dispatcher deliberately does not duplicate the step list: `AGENTS.md` is
the single source of truth. Evolve the protocol by editing `AGENTS.md`, never
this file, so every agent stays in sync.

The protocol performs its governed reads through the installed `spec-spine`
binary on your `PATH`. If `spec-spine` is not found, run `/setup` first.
