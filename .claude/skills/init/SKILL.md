---
name: init
description: Initialize a spec-spine session by executing the cross-agent New Sessions protocol declared in AGENTS.md.
---

# /init: session bootstrap

Thin Claude-Code dispatcher. The canonical protocol lives in
**`AGENTS.md` § New Sessions** under the AAIF/Linux Foundation
cross-agent standard.

## What to do

1. Read `AGENTS.md`: the section from `## New Sessions` inclusive to
   the next `## ` heading exclusive. That section is the step list.
2. Execute the protocol described there using Claude Code's parallel
   tool calls where the protocol says "dispatch simultaneously".
3. Emit the structured summary the protocol prescribes (the
   `## initialized: spec-spine` block).

This dispatcher does not duplicate the step list. AGENTS.md is the
single source of truth read by Claude Code, Codex CLI, Cursor,
Copilot, and any future agent.

The protocol drives the library through its own built binary
(`target/release/spec-spine`), not the published npm/py distributions.
If the binary is missing, build it with
`cargo build --release -p spec-spine-cli` and continue.
