---
name: code-review
description: "Review the current diff for correctness bugs and spec drift, then emit an evidence-oriented findings list"
allowed-tools: Read, Grep, Glob, Bash(git status:*), Bash(git diff:*), Bash(git log:*), Bash(git show:*), Bash(git rev-parse:*), Bash(spec-spine:*)
argument-hint: "[scope] - e.g. \"branch\", \"working tree\", \"crates/spec-spine-core\""
---

# /code-review: correctness + spec drift

Reviews the current diff against two questions: does the change have
correctness or edge-case bugs, and does it still match its owning
spec's contract. Output is an evidence-oriented findings list, each
line citing `file:line`. Read-only: no files are modified unless the
user asks for a fix afterward.

This repo is a Rust library + CLI. The gate is the built binary
`target/release/spec-spine` (build with
`cargo build --release -p spec-spine-cli` if missing).

## Step 0: scope the diff

```sh
git status --short && git diff --stat && git log --oneline -10
git diff origin/main...HEAD --stat   # committed delta
git diff HEAD --stat                 # uncommitted delta
```

Note which classes changed: Rust source (`crates/**/*.rs`, `Cargo.toml`),
specs (`specs/**/spec.md`), schemas/standards (`standards/**`), docs
(`docs/**`, `*.md`), scripts/packaging (`scripts/**`, `npm/**`, `py/**`),
workflows (`.github/**`).

## Step 1: corpus stays green

The change must not leave the spine red. Run the gate and capture the
exact outputs as evidence:

```sh
spec-spine compile
spec-spine lint --fail-on-warn       # corpus well-formedness
spec-spine index check               # staleness (exit 2 if stale)
spec-spine couple --base origin/main --head HEAD   # drift gate (exit 1 on drift)
```

- A `couple` failure is the headline finding: cite the file the gate
  named and the owning spec whose declared edges fail to cover it.
- A `lint` or `index check` failure is a corpus finding: cite the
  diagnostic verbatim.

## Step 2: spec-contract match

For each changed source file, confirm the change is consistent with the
contract of its owning spec rather than only with the gate's mechanical
pass. Useful reads (governed, via the binary, not ad-hoc JSON parsing):

```sh
spec-spine registry show <spec-id>           # the owning spec's declared surface
spec-spine registry relationships <spec-id>  # its typed edges
```

Flag drift where code does something the spec's narrative or owned
authority units do not describe, even if `couple` happens to pass
(e.g. the edge is over-broad). Cite the spec section and the `file:line`.

## Step 3: correctness pass

Read the changed source and look for, with a `file:line` and a
one-sentence evidence claim for each:

- Logic and edge-case bugs (off-by-one, unhandled `None`/`Err`, empty
  input, boundary values).
- Determinism hazards: unsorted map/set iteration, locale- or
  platform-dependent behavior, unstable ordering in emitted JSON or
  hashes (this repo's core promise is byte-identical output).
- Error-path correctness: does the change return the right exit-code
  class (1 validation/drift, 2 stale, 3 I/O/parse/schema/config)?
- Hygiene: stray debug prints, commented-out code, dead branches.

## Step 4: findings report

```
## Review: <scope>
Base: origin/main | Head: <branch> | Files: <n> | +<a>/-<d>
Gate: compile <ok|FAIL> | lint <ok|FAIL> | index check <ok|stale> | couple <ok|drift>

### Findings (severity-ordered)
- [CORRECTNESS|SPEC-DRIFT|GATE|HYGIENE] <claim> at `file:line`
  Evidence: <one sentence, cited>
  Fix: <specific recommendation>

### Clean
- <dimensions checked with nothing found>
```

If nothing is found, say so plainly and report the gate as the
evidence. To proceed with fixes, the user names the findings to apply.
