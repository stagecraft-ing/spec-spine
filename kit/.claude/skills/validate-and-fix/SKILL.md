---
name: validate-and-fix
description: Run the project's local CI loop and automatically fix discovered issues using concurrent agents.
allowed-tools: Bash, Agent, Read, Edit, Glob, Grep
---

# Validate and Fix

Run the local CI loop and automatically fix what it surfaces. Your repo should
expose one command that runs the same gate set as CI (commonly `make ci`): if it
passes locally, CI passes too. Adapt the commands below to your repo.

## 1. Run the local CI loop

Invoke your CI composite (e.g. `make ci`) from the repo root. The Makefile (or
its equivalent) is the single source of truth for what CI validates: do not
rediscover validation commands by grepping manifests.

Every spec-spine adopter's CI runs the four governance verbs in order (commonly
wrapped in a `spine` target that mirrors your spec-spine workflow):

```bash
spec-spine compile                       # compile the spec registry
spec-spine lint --fail-on-warn           # corpus conformance (a warning is a failure)
spec-spine index check                   # codebase index staleness gate
spec-spine couple --base origin/main     # spec/code coupling gate
```

Alongside those run your language gates (build, type-check, lint, tests).
Capture full output (file paths, line numbers, messages) and categorize:

- **CRITICAL**: security issues, breaking changes, data-loss risk, coupling-gate
  failure (an owned path changed without its owning spec).
- **HIGH**: functionality bugs, test failures, build breaks, index staleness.
- **MEDIUM**: `spec-spine lint` warnings (the gate runs `--fail-on-warn`), type
  errors, lint-rule violations.
- **LOW**: formatting, minor optimizations.

If a check is missing, add it to the CI composite and the relevant workflow in
the same change. Never introduce a new validation via a one-off script.

## 2. Strategic fix execution

- **Phase 1, safe quick wins**: LOW/MEDIUM findings that cannot break anything.
  Verify each by re-running the narrowest affected target.
- **Phase 2, functionality fixes**: HIGH findings one at a time; re-run the
  affected target after each.
- **Phase 3, critical issues**: handle CRITICAL findings with explicit user
  confirmation and a plan first. Coupling failures need judgement: refusing the
  destructive step is sometimes the right answer (see
  `.claude/rules/adversarial-prompt-refusal.md`).
- **Phase 4, verification**: re-run the full CI composite end to end.

## 3. Error handling

- **Rollback**: `git stash push -m "pre-validate-and-fix"` before any change;
  offer instant rollback if a fix regresses.
- **Partial success**: continue past a fix that fails; separate successes from
  failures; give manual instructions for what you could not fix.
- **Governed reads**: read `.derived/**` only through `spec-spine` verbs, never
  `python`/`jq`/`awk`/`sed` (see `.claude/rules/governed-artifact-reads.md`).

## 4. Parallel execution

Launch multiple agents concurrently for independent, parallelizable fixes:

- Put multiple Agent calls in a SINGLE message ONLY when the tasks are truly
  independent (fixes in different packages, non-overlapping spec edits).
- Keep cross-interface or ordered changes sequential.
- Each agent owns non-overlapping files and verifies its own fix before
  reporting complete.

## 5. Final verification

- Re-run the full CI composite to confirm a clean local pass.
- Confirm no new issues were introduced.
- Summary: `Fixed X/Y issues, Z require manual intervention. CI: {PASS|FAIL}`.

## Substrate notes

- `spec-spine lint` runs with `--fail-on-warn`: a warning is a failure.
- The coupling gate compares `HEAD` against `origin/main`; if `origin/main` is
  not fetched the gate cannot run (`git fetch origin main` first).
- The codebase index hashes more than `spec.md`: its inputs are configured in
  `spec-spine.toml [index] extra_hashed_inputs` plus the manifests it discovers.
  Editing a hashed input without regenerating the index fails the staleness
  check.
- If `.claude/settings.json` or `.mcp.json` are hashed inputs they are hashed
  byte-for-byte: editor reformatting trips the staleness gate even when the JSON
  is semantically unchanged. Edit in place, do not reformat.

## Add your own quality checklist

The mechanized gates above catch what CI can enforce. Layer a project-specific,
post-feature checklist on top (framework invariants, route and DTO alignment,
auth scoping, env-var coverage, and so on) and run it after feature work. Keep
that checklist in your own repo: it is intentionally not shipped with the kit,
because it is specific to your stack.
