---
id: "022-keypath-section-anchors"
title: "Bounded keypath section anchors for first-party structured config"
status: draft
kind: "tooling"
created: "2026-06-14"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "004-codebase-index"
  - "005-coupling-gate"
extends:
  - spec: "004-codebase-index"
    nature: additive
    paths:
      - "crates/spec-spine-core/src/sections.rs"
summary: >
  Widens the section-anchor grammar (`sections.rs`, spec 004 §3.3) so a
  `{ kind: section, file, anchor }` unit can name a bounded mapping keypath
  inside a first-party structured-config file: a workflow YAML
  (`on`, `permissions`, `env`, `jobs.<name>`, `jobs.<name>.permissions`), a
  `Cargo.toml` table path (`workspace`, `package.metadata.oap`), or a
  `package.json` member (`scripts`, `dependencies`). The anchor field is
  already a free string (`Unit::Section`), so this is a permissive-payload
  additive change with no frontmatter or JSON-schema edit, mirroring spec 017.
  The motivating cases are ASI security boundaries: `permissions:`
  (GITHUB_TOKEN scope) and `on`/`on.merge_group` (the trigger and merge-queue
  contract). Section ownership of those keys means any token-scope escalation
  or trigger change forces the owning spec's review. A hard file-eligibility
  predicate keeps the grammar pointed at surfaces spec-spine has a first-party
  reason to parse and off foreign tool schemas (deny.toml, Helm values, k8s
  manifests), which their owners rev independently. This retires the
  `# region:`-in-structured-config escape hatch for the eligible surfaces: a
  structural keypath is computed from the document tree and cannot silently
  drift from the block it claims, the way a hand-placed region marker can.
---

# 022: bounded keypath section anchors for first-party structured config

## 1. Purpose

`sections.rs` resolves a named section to a `LineSpan` for four file shapes:
Makefile targets, Markdown heading slugs, `region:` comment markers, and CI
`jobs.<name>` blocks (bare job key only). For structured-config files this
leaves exactly two ways to own a sub-file slice, and both are inadequate for
the security-relevant cases:

1. **Whole-file** ownership. Too coarse: a spec that owns a workflow only to
   govern its `permissions:` block is forced to review every unrelated edit to
   that workflow, and the coupling gate cannot distinguish a token-scope
   escalation from a step rename.
2. **Injected `# region:` markers.** An escape hatch, and a drifting one. A
   region marker is a hand-placed comment that lives independently of the block
   it wraps; moving or deleting the `# endregion` line silently un-claims the
   slice while the governed config stays put. The anchor and the thing it
   anchors are not the same object.

The two motivating boundaries are agentic-security (ASI) surfaces:

- **`permissions:`** in a workflow is the GITHUB_TOKEN scope, the security
  boundary for what a CI run (and any agent acting through it) may do. Making
  it section-ownable means a spec can claim it directly, so any scope
  escalation, top-level or per-job, fails the coupling gate against the owning
  spec rather than passing as an ordinary workflow tweak.
- **`on`** (and `on.merge_group`) is the trigger and merge-queue contract.
  Owning it at section granularity forces review when *what causes CI to run*
  changes.

A structural keypath anchor is computed from the document tree on every index
build, so unlike a region marker it cannot drift from the block it names. This
spec adds that grammar for the surfaces spec-spine has a first-party reason to
parse, and deliberately withholds it from foreign tool configs.

## 2. Territory

The section enumerator and resolver in
`crates/spec-spine-core/src/sections.rs` (`enumerate_sections`,
`resolve_section`, and the per-shape dispatchers). This extends spec 004's
section-anchor grammar (the `sections.rs` header cites spec 004 §3.3) and is
consumed transparently by the spec 005 coupling gate, which already calls
`resolve_section` / `enumerate_sections`; the gate gains keypath attribution
with no call-site change.

`Unit::Section { file, anchor }` already carries a free-string `anchor`
(`crates/spec-spine-types/src/unit.rs`). The grammar widens which `anchor`
strings resolve; it adds no field and no kind. As with spec 017, the registry
and index schemas are permissive on the unit payload, so there is no
schema-file edit. A resolved keypath produces the same `ResolvedLocation`
(a `LineSpan`) any existing section produces.

## 3. Behavior

### 3.1 Grammar: the bounded keypath

A section `anchor` on an eligible structured-config file (§3.2) MAY be a
**dotted keypath** of mapping or table keys: `permissions`,
`jobs.build.permissions`, `package.metadata.oap`, `scripts`. The grammar is
deliberately bounded:

- **Mapping keys only.** No sequence indexing (`jobs.build.steps.0` MUST NOT
  resolve). An array and its elements are owned by their parent key or by the
  whole file, never by index.
- **Bounded depth**, per format (§3.3). Deep paths beyond the bound MUST NOT
  resolve; they fall through to I-006 (§3.5), not to a partial match.
- **No wildcards, no globs.** A keypath names exactly one block.

Back-compat: the existing bare `jobs.<name>` behavior is preserved. A single
bare segment that names a job (e.g. `build`) MUST continue to resolve to that
job's block, in addition to the qualified `jobs.build`. Existing specs that
anchor a bare job name keep working unchanged.

### 3.2 File eligibility: the ownership boundary (hard)

Keypath resolution is enabled only for files spec-spine has a first-party
reason to parse:

- **Workflow YAML**: a path under `.github/workflows/` ending `.yml` / `.yaml`.
  (CI that spec-spine ships and governs.)
- **`Cargo.toml`** (any path with that basename). (A manifest spec-spine
  already reads for the package inventory and the spec-metadata key.)
- **`package.json`** (any path with that basename). (Likewise.)

For any other structured file (`deny.toml`, a Helm `values.yaml`, a k8s
manifest, an arbitrary `.json`), a dotted anchor MUST NOT receive keypath
treatment. Those files retain whole-file ownership and, where a comment syntax
exists, `region:` markers. The rationale is load-bearing: spec-spine must not
bind to a schema it does not own. `deny.toml` is cargo-deny's settings file;
`values.yaml` is Helm's; a manifest under `k8s/` is Kubernetes'. Those tools
own and rev their internal structure, and a spec-spine keypath into them
(`deny.toml#advisories`) would couple the gate to a third-party schema that can
change without notice. Whole-file coupling stays legitimate everywhere, because
it only diffs the path and never parses contents.

### 3.3 Resolution (per format)

All three resolvers return inclusive 1-based `LineSpan`s aligned with
`git diff -U0` hunk ranges, consistent with the existing dispatchers.

- **Workflow YAML** (indentation-aware structural scan, generalizing the
  current `ci_job_sections`). `enumerate_sections` yields: every top-level
  mapping key (`on`, `permissions`, `env`, `concurrency`, `defaults`, `jobs`,
  and any other top-level key); each `jobs.<name>` block; one nested level
  under a job (`jobs.<name>.<key>`, e.g. `jobs.build.permissions`); and a bare
  `<name>` alias for each job (back-compat). A block's span runs from its key
  line to the last line before the next key at the same or shallower indent.
  Maximum depth 3 (`jobs.<name>.<key>`). The scan stays hand-rolled (not
  `serde_yaml`, which does not expose line spans), matching the existing code.
- **`Cargo.toml`** (span-aware document model via the `toml` crate, which
  retains source spans through `toml_edit`). Resolve a table keypath
  (`[workspace]`, `[workspace.package]`, `[package.metadata.oap]`) and a
  top-level dotted key to the span from its header (or key) line through the
  last line before the next sibling-or-shallower table. Maximum depth 4 to
  cover `package.metadata.oap`.
- **`package.json`** (bounded brace-depth line scan). Locate the member named
  by the keypath (top level plus one nested level, e.g. `scripts`,
  `dependencies`) and span from its `"key":` line to the matching close. JSON
  has no comment syntax, so there is no region-marker fallback for it.
  Maximum depth 2.

### 3.4 Staleness

A keypath anchor carries a span, so its backing config file folds into the
index content hash: a line shift that moves the claimed block stales the index
and must be recommitted, exactly as for `section` and `symbol` units
(spec 017 §3.3). This is the property that makes the keypath superior to a
region marker, which the indexer cannot detect drifting because the marker
moves with the edit.

### 3.5 Diagnostics

An anchor that does not resolve, whether an unknown keypath, an over-deep
keypath, a sequence index, or any keypath on an ineligible file (§3.2), fires
the existing **I-006** (`"spec '{spec_id}' section unit '{anchor}' not found in
{file}"`, `index.rs`). No new diagnostic code is introduced; the keypath grammar
only widens the set of anchors that *do* resolve. (See Design decision D3 on
whether an ineligible-file keypath warrants a more specific message.)

### 3.6 Tests (minimum)

- **Grammar / back-compat**: in a workflow fixture, `build`, `jobs.build`, and
  `jobs.build.permissions` all resolve; top-level `on`, `permissions`, `env`
  resolve; `on.merge_group` resolves.
- **Cargo.toml**: `workspace`, `workspace.package`, `dependencies`, and
  `package.metadata.oap` resolve to their table spans.
- **package.json**: `scripts` and `dependencies` resolve to their member spans.
- **Boundary (hard)**: a dotted anchor on `deny.toml` and on a non-workflow
  `values.yaml` does NOT resolve (I-006), proving the eligibility predicate.
- **Bounds**: a sequence index (`jobs.build.steps.0`) and an over-deep keypath
  do not resolve.

## 4. Out of scope

- Sequence/array indexing and arbitrary depth beyond the per-format bounds.
- Keypath anchoring into foreign tool configs (the §3.2 boundary): they stay
  whole-file or region-marker.
- Routing non-workflow / foreign YAML to `region_sections` (today `.yml`
  dispatches to the job scanner, so foreign YAML gets no region parsing at all).
  Whether to give foreign structured config a `region:` fallback is a separate
  question recorded as Design decision D4, not built here.
- Any registry or index JSON-schema text change (the `anchor` payload is a free
  string; permissive-additive like spec 017).
- Manifest formats beyond `Cargo.toml` / `package.json` (e.g.
  `pnpm-workspace.yaml`, `pyproject.toml`): widen the eligibility allowlist on
  demand under a follow-up.

## 5. Design decisions (for review, before implementation)

These are the choices that shape the implementation; flagged for sign-off.

- **D1 - Eligibility is a hard code predicate, not a soft convention
  (recommended).** §3.2 makes ineligibility a non-resolution in code, so a spec
  *cannot* keypath into `deny.toml` even by trying. The alternative is to
  resolve keypaths for all `.yml`/`.toml`/`.json` and enforce the boundary by
  review only. Hard is the ASI-defensible choice and matches the 2026-06-14
  ownership-boundary ruling.
- **D2 - INDEX_SCHEMA_VERSION bump?** Spec 017 bumped 0.2.0 -> 0.3.0 because it
  added unit *kinds* to the payload. This spec adds no kind, field, or shape:
  existing corpora hash identically; only new specs that use keypath anchors
  produce new (same-shaped) resolutions. So a bump is arguably unnecessary.
  Recommendation: no bump, document the capability in the spec; or a 0.3.0 ->
  0.4.0 minor purely as a capability marker. Pick one.
- **D3 - Ineligible-file keypath diagnostic.** §3.5 reuses I-006. An author who
  writes `deny.toml#advisories` gets "not found", which is true but may read as
  a typo rather than "this file is not keypath-eligible." Option: a distinct
  message (still I-006, or a new I-010) naming the boundary. Recommendation:
  keep I-006, refine the message text only.
- **D4 - Region fallback for foreign YAML.** OAP's vendored gate parses
  `# region:` markers in Helm/k8s YAML; the spec-spine library currently does
  not (`.yml` -> job scanner, never `region_sections`). Folding that in would
  complete the "first-party gets keypaths, foreign gets region markers" story
  but expands this spec. Recommendation: defer to a focused follow-up; keep 022
  about the keypath grammar only.
- **D5 - Depth bounds.** YAML depth 3 (`jobs.<name>.<key>`), TOML depth 4
  (`package.metadata.oap`), JSON depth 2. Confirm these cover the corpus's real
  claims without inviting deep structural coupling.

## 6. Resolved decisions (as implemented)

The implementation in `crates/spec-spine-core/src/sections.rs` resolves the §5
decisions as follows:

- **D1: hard predicate.** `is_workflow_path` plus a `Cargo.toml` / `package.json`
  basename match gate keypath resolution; an ineligible file never reaches a
  keypath resolver. Adopted as recommended.
- **D2: no schema bump.** No `INDEX_SCHEMA_VERSION` change. The `anchor` payload
  is unchanged (a free string), existing corpora hash identically, and only new
  keypath anchors produce new (same-shaped) resolutions. The capability is
  documented here rather than versioned.
- **D3: I-006 unchanged.** An ineligible-file or unresolved keypath fires the
  existing I-006 with its current message, per §3.5. A boundary-naming message
  refinement was not pursued and remains a possible follow-up.
- **D4: foreign-YAML region fallback deferred.** Non-workflow YAML keeps the
  legacy bare-`jobs.<name>` scanner (no keypaths, no region markers), unchanged
  from before this spec. Folding region markers into foreign YAML stays out of
  scope.
- **D5: bounds shipped.** Workflow YAML resolves depth 1, depth 2
  (`parent.child`, covering `on.merge_group` and `jobs.<name>`), and depth 3
  under a job only (`jobs.<name>.<key>`); `Cargo.toml` tables to depth 4;
  `package.json` members to depth 2. Sequence indices and over-deep keypaths do
  not resolve.
