---
id: "009-coupling-floor-claim-precedence"
title: "Coupling gate: explicit unit claims take precedence over the bypass floor"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "005-coupling-gate"
amends:
  - "005-coupling-gate"
extends:
  - { spec: "005-coupling-gate", unit: "crates/spec-spine-core/src/couple.rs", nature: additive }
  - { spec: "005-coupling-gate", unit: "crates/spec-spine-core/tests/couple.rs", nature: additive }
  # The 005 §3.5 auto-waiver pre-filter must see the same claim-aware bypass
  # verdict as the gate (is_bypassed_path gains the index parameter), so the
  # CLI wiring and its e2e contract file are touched too. Both additive.
  - { spec: "005-coupling-gate", unit: "crates/spec-spine-cli/src/cmd_couple.rs", nature: additive }
  - { spec: "005-coupling-gate", unit: "crates/spec-spine-cli/tests/couple.rs", nature: additive }
summary: >
  Amends 005 §3.5: a path covered by an explicit unit claim (a file, section,
  or symbol declared in some spec's ownership-bearing edge and resolved into
  the index) is NEVER bypassed, even when it matches the hardcoded floor or an
  adopter's additive bypass list. Implicit path-level ownership (manifest
  floors, comment headers) keeps deferring to bypass. Closes the silent-
  weakening hole where an adopter who governs workflow YAML or specific docs
  by explicit claim would have those claims ignored because `.github/` and
  `docs/` sit on the immutable floor.
---

# 009: Explicit unit claims take precedence over the bypass floor

## 1. Purpose

Spec 005 §3.5 makes the bypass floor absolute: `config.coupling.bypass_prefixes`
is additive-only and "cannot remove a floor entry". That is the right contract
for *unclaimed* prose and scaffolding -- but it silently disables the gate for
an adopter whose specs **explicitly claim** units beneath a floor prefix.

The motivating adopter is OAP, where workflow YAML is governed truth: specs
declare `co_authority` over `jobs.<name>` sections of `.github/workflows/*.yml`
(OAP spec 118 / 152 lineage), and this repo's own corpus already does the same
thing -- specs 007/008 declare `establishes` / `extends` units on
`.github/workflows/release.yml`, which today the floor renders unenforceable.
A gate that ignores an explicit, resolved authority claim is worse than one
that lacks the claim: the corpus *says* the path is governed and the gate
silently disagrees.

The fix is a precedence rule, not a removable floor. The floor stays immutable
as a *default*; an explicit claim overrides it for exactly the claimed surface.

## 2. Territory

`spec-spine-core`'s `couple.rs` (the bypass decision inside the pure engine)
and its tests; plus the 005 §3.5 auto-waiver pre-filter in `cmd_couple.rs`,
whose `is_bypassed_path` verdict becomes claim-aware (it reads the committed
index) so a claim-overridden floor path cannot slip past the manifest-only
candidate check into a mechanical waiver. No user-facing CLI surface change;
no config knob change; no schema change.

## 3. Behavior

### 3.1 The precedence rule

For a changed path `P`:

1. Compute `explicitly_claimed(P)`: true iff the committed index contains at
   least one **resolved unit** (kind `file`, `section`, or `symbol`, from any
   ownership-bearing edge: `establishes` / `extends` / `refines` /
   `co_authority` / `constrains` / `supersedes`-transferred) whose location
   file equals `P` (file units with a trailing-slash directory path match `P`
   by directory prefix, per 004 §3.3).
2. If `explicitly_claimed(P)`, the bypass check is **skipped entirely**: `P`
   proceeds to owner derivation and clearance exactly as a non-bypassed path
   (005 §3.2–§3.5). Violations emit `C-001` as usual.
3. Otherwise the effective bypass set (floor ∪ config additions) applies
   unchanged.

### 3.2 What does NOT override bypass

**Implicit path-level ownership does not override.** The two path-level
linkage sources from 004 §3.2 -- manifest metadata (`[package.metadata.<ns>]
.spec`, the whole-crate coverage floor) and `// Spec:` comment headers -- keep
deferring to bypass. This preserves the floor's purpose and this repo's own
dogfood config: `**/README.md` stays bypassed even though every crate README
sits inside a manifest-floored directory. The dividing line is intent: an
explicit unit in spec frontmatter is an author saying *this exact surface is
governed*; a crate floor is a blanket safety net.

### 3.3 Precedence over adopter additions too

The rule overrides the **entire** effective bypass set, including
`config.coupling.bypass_prefixes` entries, not just the hardcoded floor. An
adopter who both bypasses a pattern and explicitly claims a unit under it has
stated two intents; the specific one (the claim) wins. This needs no new
config: un-claiming the unit restores the bypass.

### 3.4 Effect on this repo's own corpus

Once this lands, `.github/workflows/release.yml` stops being bypassed (specs
007/008 claim it explicitly), so future `release.yml` edits MUST be
accompanied by an edit to 007 or 008 (or a waiver). This is the intended
behavior and serves as the live acceptance test: the dogfood corpus is the
first adopter of the rule.

### 3.5 Determinism and report shape

`couple_with` stays a pure function of `(config, registry, index, diff,
waiver)`. The `CoupleReport` gains no new fields; a path un-bypassed by this
rule is indistinguishable in the report from any other evaluated path. (A
debug-level note in human output -- "bypass overridden by explicit claim from
<spec-id>" -- is RECOMMENDED for the violation rendering, not required.)

### 3.6 Tests (minimum)

- Floor path with an explicit file unit claim → evaluated, drifts without the
  owning spec in the diff, clears with it.
- Floor path with only manifest-floor ownership → still bypassed.
- Adopter-added bypass (`**/README.md`) with an explicit claim on one specific
  README → that README evaluated; sibling READMEs still bypassed.
- Directory-form file unit (`py/` style) claiming under a floor prefix →
  subtree evaluated.
- Section unit on a workflow `jobs.<name>` under `.github/` → evaluated.

## 4. Out of scope

Removing or configuring floor entries (the floor stays immutable; this spec
makes it *yield to claims*, not *editable*). Constraint-specific exit codes or
evaluation semantics for `constrains` beyond its existing ownership-bearing
role. Diff-side section attribution changes (the hunk-to-span matching of 005
§3.2 is untouched -- only the bypass short-circuit moves).
