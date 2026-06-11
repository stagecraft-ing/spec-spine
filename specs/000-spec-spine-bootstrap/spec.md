---
id: "000-spec-spine-bootstrap"
title: "Bootstrap spec system (markdown → compiled JSON authority ledger)"
status: approved
kind: "constitutional-bootstrap"
created: "2026-06-08"
authors:
  - "The spec-spine Authors"
origin:
  retroactive: true
unamendable:
  - "markdown-truth-boundary"
  - "json-truth-boundary"
  - "determinism-requirement"
  - "directory-name-equals-id"
  - "typed-authority-graph"
  - "refusal-rule"
summary: >
  Foundational contract for spec-spine. Authored truth lives only in markdown
  (with YAML frontmatter); machine-consumable truth is compiler-emitted JSON
  only; the corpus compiles in full from day one; every artifact-producing step
  is a pure, deterministic function of (config, file contents); and a typed
  authority graph, not a flat pile of claims, governs who may change what.
  This spec defines what a spec IS. It is tier-1: its `unamendable` anchors are
  non-overridable, and it bootstraps the corpus by declaring authority it has
  held since before the graph existed (`origin.retroactive`).
---

# 000: Bootstrap spec system

This is the spec that defines what a spec is. It bootstraps the corpus: it was
authored by hand before the compiler existed, the compiler is built to satisfy
it, and once built the compiler compiles this spec along with the rest. It sits
at the top of the constitutional hierarchy (see `standards/spec/constitution.md`,
which is subordinate to this document).

## 1. The authoring/derived boundary

There are exactly two kinds of truth in a spec-spine'd repository.

- **Authored truth** lives only in markdown (`specs/NNN-slug/spec.md`), with YAML
  frontmatter blocks permitted inside the markdown. Humans (and authorized
  agents) write authored truth. *(anchor: `markdown-truth-boundary`)*
- **Machine-consumable truth** is emitted only by the compiler/indexer, as JSON,
  into the derived output tree (`layout.derived_dir`, default `.derived/`). No
  hand-authored JSON is authoritative; no consumer may treat hand-edited JSON as
  truth. *(anchor: `json-truth-boundary`)*

Corollary: **typed reads or nothing.** Compiled JSON is read only through a
typed consumer (the `spec-spine` binary or the `spec-spine-core` library). Ad-hoc
parsing of compiled JSON (`jq`/`awk`/`sed`/hand-rolled readers) is a workflow
violation: it silently encodes schema assumptions that then fail far from the
read instead of at the deserializer.

## 2. Identity: directory name equals id

A spec's directory under `layout.specs_dir` is named exactly `NNN-slug`, where
`NNN` is a three-digit zero-padded ordinal and `slug` is a kebab-case name. The
spec's `id` frontmatter field MUST equal that directory name. The numeric prefix
`NNN` is unique across the corpus. *(anchor: `directory-name-equals-id`)*

## 3. Frontmatter grammar

Every `spec.md` begins with a YAML frontmatter block delimited by `---` fences.

**Required keys** (absence is a compile error): `id`, `title`, `status`,
`created`, `summary`.

- `status` ∈ { `draft`, `approved`, `superseded`, `retired` }.
  - `superseded` requires `superseded_by` resolving to an existing id.
  - `retired` requires `retirement_rationale`.
- `created` is an ISO date (`YYYY-MM-DD`).

**Optional descriptive keys**: `authors`, `owner`, `risk`
(`low`/`medium`/`high`/`critical`), `depends_on`, `code_aliases`,
`feature_branch`, `implementation` (`pending`/`in-progress`/`complete`/`n-a`/
`deferred`), and the two opt-in categorical taxonomies `domain` and `kind`
(each validated against its configured allowlist only when that allowlist is
non-empty; otherwise free-text, see the constitution).

**Bootstrap marker**: `origin.retroactive: true` declares authority held since
before the graph existed. Without it, a pre-graph spec would read as a fresh
`establishes` claim and the history would be wrong. Reserved for genuine
foundational bootstrap (this spec uses it).

**Freeze surface**: `unamendable` is a list of anchors that no amendment may
alter. The `unamendable` list in *this* frontmatter is the authoritative freeze
surface of the system.

**Unknown keys** fall into `extra_frontmatter` (scalars and string-lists only),
capped, so downstream layers can carry their own metadata through the compiler
without forking the types. Adopters may promote keys into the recognized set via
`frontmatter.extra_known_keys`.

## 4. The typed authority graph

A spec does not merely claim "I exist." It declares, in frontmatter, **typed
edges** to the rest of the corpus and the **authority units** it owns. Authority
over any unit is *derived by walking the graph*, never declared directly.
*(anchor: `typed-authority-graph`)*

### 4.1 Edges: eight types, seven ownership-bearing

| Edge | Ownership? | Meaning |
|---|---|---|
| `establishes` | yes | first brings a unit into being (historical origin) |
| `extends` | yes | adds surface to a predecessor without disturbing it |
| `refines` | yes | tightens behavior on a named aspect |
| `supersedes` | yes | replaces a predecessor; inherits its current authority |
| `amends` | yes | patches a predecessor in place; co-authority over its `spec.md` |
| `co_authority` | yes | shares a path on a named section with another spec |
| `constrains` | yes | asserts an invariant others must respect |
| `references` | **no** | points without claiming authority (gate ignores it) |

`origin` is a bootstrap marker, **not** an edge. The graph has eight edge types.

### 4.2 Authority units: file, section, symbol (v1)

Ownership resolves at three granularities, declared via a `unit:` on an edge:

- **file**: `{ kind: file, path }` (a bare string is shorthand for this; a
  trailing-slash path denotes the directory subtree).
- **section**: `{ kind: section, file, anchor }` (a Makefile target, a Markdown
  heading slug, a `region:` marker, a CI `jobs.<name>`).
- **symbol**: `{ kind: symbol, id }` (a function/type/export, resolved by the
  indexer; Rust and TypeScript in v1).

(`crate`, `module`, `directory` are reserved for additive future minors.)

### 4.3 Resolution and amends-awareness

"Who currently owns unit X" is a derived query over the graph, computed by the
indexer at index time, not a runtime guess. A `supersedes` edge transfers a
predecessor's current authority to the superseding spec; `establishes` records
historical origin. An `amends` edge grants the amender co-authority over the
predecessor's `spec.md`, but only expands an already-firing owner set; it never
silently enrolls a new owner.

## 5. The compiled artifacts

- **Registry** (`registry.json`, spec-as-source): the output of `compile`: for
  each spec, its status, relationships, claimed units, and a validation report.
- **Codebase index** (`index.json`, code-as-source): the output of `index`: for
  each path/section/symbol, which spec(s) currently claim it, plus a content-hash
  for staleness detection.

They are inverses. The coupling gate joins them at PR time and refuses the merge
if they disagree. Both are compiler-owned JSON (§1).

## 6. Determinism

Every artifact-producing function is a pure function of `(config, file
contents)`: the same committed inputs MUST produce byte-identical output. No
ambient clock or environment reads enter an artifact; the sole exception is a
wall-clock `builtAt` field in `build-meta.json`, which is excluded from every
determinism and golden check. *(anchor: `determinism-requirement`)*

Determinism is what makes the ledger a ledger: two agents producing changes
independently produce diffable, mechanically-mergeable artifacts, and staleness
is detectable by content-hash comparison alone.

**Staleness completeness corollary.** Because staleness is detected by
content-hash comparison alone, that hash MUST fold *every* input that can change
the resolved artifact: not only the spec/manifest/config corpus, but the source
files whose `symbol`/`section` spans the index resolved. A blind spot in the hash
is a silent correctness hole: an input could drift while the artifact reads
fresh, and the coupling gate would then match diffs against stale spans. The
hash's input set is therefore a closed function of what the artifact actually
depends on.

## 7. The guardrails

1. A **deterministic compiler** mints the registry (§5, §6).
2. A **coupling gate** at PR time refuses code/spec drift.
3. **Typed reads or nothing** for compiled JSON (§1 corollary).
4. A **refusal rule** at prompt time stops an agent from "resolving" a coupling
   failure by quietly editing the contract to match the code it just wrote. The
   agent MUST surface the contradiction and let a human (or an agent with
   explicit authority) decide. *(anchor: `refusal-rule`)*

The coupling gate is the PR-time defense; the refusal rule is the prompt-time
defense. Together they sandwich the failure mode of an agent erasing the contract
to keep going.

## 8. Bootstrap order

1. This spec (`000`) is authored by hand.
2. `spec-spine-types` implements its frontmatter grammar, config, and DTOs.
3. The compiler is built to satisfy this spec.
4. Once built, the compiler compiles this spec and the rest of the corpus; the
   repo governs itself (dogfood).

## 9. Status

This is a `constitutional-bootstrap` spec. Its `unamendable` anchors
(§frontmatter) are frozen: no amendment may alter the authoring/derived boundary,
the identity rule, the typed-authority-graph principle, the determinism
requirement, or the refusal rule. Amendments may add surface elsewhere.

## Amendments received

**Amendment 2026-06-11 (record: specs 004/005 dependency-only cutover).**
The `[coupling]` config table this spec's types crate carries gains one
additive key: `auto_waive_dependency_only` (bool, default `false`) — the
opt-in switch for spec 005 §3.5's mechanical dependency-only auto-waiver.
No unamendable anchor is touched; `deny_unknown_fields` keeps a misspelled
knob a loud config error.
