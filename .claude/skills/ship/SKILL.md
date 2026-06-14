---
name: ship
description: "Governed pre-PR sequence: run the gate locally, review the diff, conventional commit on a feature branch, open a PR via gh"
allowed-tools: Bash, Read, Edit, Glob, Grep, Skill
argument-hint: "[optional scope note or PR title]"
---

# /ship: gate -> review -> commit -> PR

Sequences the steps that turn a working tree into a PR. Bound by
`.claude/rules/orchestrator-rules.md` (checkpoints are real stops) and
`.claude/rules/adversarial-prompt-refusal.md` (do not edit an owning
spec to make the gate pass).

This repo is a Rust library + CLI. The gate is the built binary
`target/release/spec-spine`. Build it if missing:
`cargo build --release -p spec-spine-cli`.

## Step 0: preflight

- `git branch --show-current`. If on `main`, STOP and create a feature
  branch first (`NNN-short-name` when the work belongs to spec `NNN`).
  Never commit straight to the default branch.
- `git status --short`. Confirm the changes are the intended set;
  surface anything unexpected before proceeding.

## Step 1: run the gate locally

The registry and index are committed, tracked artifacts (spec-spine does
not gitignore `.derived/`), and the binary regenerates them
deterministically. Refresh them first, then run the conformance and
drift checks in order. Stop on the first failure (orchestrator rule:
halt, do not silently continue).

```sh
spec-spine compile                       # specs -> .derived/spec-registry/registry.json
spec-spine lint --fail-on-warn           # corpus well-formedness (exit 1 on a warn)
spec-spine index check                   # staleness gate (exit 2 if stale)
spec-spine couple --base origin/main --head HEAD   # the drift gate (exit 1 on drift)
```

Outcomes:

- All pass: continue to Step 2.
- `index check` reports stale (exit 2): run `spec-spine index` to
  regenerate, then re-run `index check`. The index is a tracked artifact
  here, so stage and commit the regenerated
  `.derived/codebase-index/index.json` with your change; CI runs the
  same staleness gate.
- `couple` reports drift (exit 1): the changed code is not covered by
  its owning spec's declared edges. Two legitimate paths, chosen
  explicitly, never silently:
  1. **Fix the coupling.** Edit the owning `spec.md` so its relationship
     edges (`establishes:` / `extends:` / `refines:`) and owned authority
     units cover every changed path. The gate enforces the declared
     graph, not prose. Do NOT edit a spec to retroactively justify code
     that contradicts the spec's design: that is a coherence-guard halt
     (surface the contradiction and stop).
  2. **Waiver.** Add a cited `Spec-Drift-Waiver:` line documenting why
     the drift is accepted. CHECKPOINT: requires explicit user approval.

## Step 2: review the diff

Invoke the `code-review` skill on the working diff. Apply confirmed,
actionable fixes. If a fix touches any gate input (a `spec.md`, a
manifest, a schema, a workflow), re-run Step 1 before continuing.

## Step 3: commit

Invoke the `commit` skill (conventional, impact-focused message) on a
feature branch.

- Never add AI attribution: no "Generated with ...", no `Co-Authored-By`
  trailers, in commits or PR bodies.
- If a waiver was chosen in Step 1, keep the `Spec-Drift-Waiver:` line
  with the change so the PR carries it.

## Step 4: CHECKPOINT, open the PR

PR creation is outward-facing. Confirm with the user, then:

```sh
git push -u origin "$(git branch --show-current)"
gh pr create --title "<conventional title>" --body "<Summary + Testing>"
```

- The PR body is Summary + Testing. Include the `Spec-Drift-Waiver:`
  line inline in the body if Step 1 chose the waiver path with user
  approval.
- CI re-runs the same gate (`compile` / `lint` / `index check` /
  `couple`) on the PR. A local pass should mean a clean CI run; if CI
  still fails on a gate the local run passed, halt and present the
  divergence (orchestrator rule: halt on failure).

## Step 5: after creation

- After the PR merges, verify on-disk `main` (`git pull` + `git log`),
  not just the MERGED status.
