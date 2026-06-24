---
id: adoption-guide
title: Adoption Guide
sidebar_position: 5
---

# Adoption Guide

This guide explains how to take a conventional repository from zero to spec-governed. There are no source edits to the library required; every project-specific assumption is a configurable knob.

:::tip Driving spec-spine with Claude Code
To run the governed workflow with an AI agent, see [Use with Claude Code](claude-code/overview.md): a ready-to-copy kit of skills, agents, and rules that chains the gate into everyday development.
:::

The process involves four steps: **install**, **init**, **annotate**, and **wire CI**.

## 1. Install

Choose the distribution channel that fits your stack. See the [Installation](getting-started/installation.md) page for details.

```bash
# Example: using cargo
cargo install spec-spine-cli --version 0.8.0 --locked
```

## 2. Scaffold the corpus

Run the initialization command at your repository root:

```bash
spec-spine init
```

This creates your `spec-spine.toml` configuration, the `standards/` directory containing constitutional templates, and your first tier-1 spec at `specs/000-bootstrap/spec.md`.

Compile and lint the empty corpus to ensure everything is well-formed:

```bash
spec-spine compile
spec-spine lint
```

## 3. Annotate manifests and code

You must link your codebase to the specs. There are three linkage directions, and the coupling gate joins all three.

### Manifest key (Crate/Package to Spec)

Add the metadata key to your package manifests.

**Cargo (`Cargo.toml`):**
```toml
[package.metadata.spec-spine]
spec = "000-bootstrap"
```

**npm (`package.json`):**
```json
{
  "spec-spine": {
    "spec": "000-bootstrap"
  }
}
```

### Comment header (File to Spec)

Add a doc-comment at the root of a file to declare its owning spec.

```rust
// Spec: specs/000-bootstrap/spec.md
```

### Spec edges (Spec to Code)

In your markdown specs (`specs/*/spec.md`), declare the units the spec owns using frontmatter edges:

```yaml
---
status: approved
establishes:
  - src/core/
extends:
  - src/api.rs
---
```

After annotating, build the codebase index and commit the `.derived/` directory (excluding `build-meta.json`):

```bash
spec-spine index
git add .derived/
```

## 4. Wire CI

The coupling gate runs at PR time and refuses a changed, owned path whose owning spec was not also edited.

Add a workflow to your CI system. Here is an example for GitHub Actions:

```yaml
name: spec-spine
on: pull_request
permissions:
  contents: read
jobs:
  govern:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - run: curl -fsSL https://raw.githubusercontent.com/stagecraft-ing/spec-spine/main/install.sh | sh
      - run: spec-spine compile
      - run: spec-spine index check
      - run: spec-spine lint --fail-on-warn
      - name: Coupling gate
        env:
          PR_BODY: ${{ github.event.pull_request.body }}
        run: |
          set -euo pipefail
          printf '%s' "${PR_BODY:-}" > /tmp/pr-body.txt
          spec-spine couple \
            --base "${{ github.event.pull_request.base.sha }}" \
            --head HEAD \
            --pr-body /tmp/pr-body.txt
```

### Handling Waivers

When drift is deliberate (e.g., a cross-cutting refactor or dependency bump), add a waiver to the PR body:

```text
Spec-Drift-Waiver: mechanical refactor of helper function
```

This downgrades the coupling gate violation from an error to a warning, allowing the PR to merge while recording the waiver in the ledger.

## OAP-style Adopters

If your repository requires domain-specific output (e.g., compliance reports, factory artifacts), you should adopt spec-spine as a generic core and build an **overlay crate**. Do not fork the core library. See [Extending and Overlays](extending-and-overlays.md) for the overlay contract.
