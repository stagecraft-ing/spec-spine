# spec-spine constitution

Durable principles that govern this corpus. This document is **tier 2**: it is
subordinate to the bootstrap spec (`specs/000-spec-spine-bootstrap/spec.md`),
whose `unamendable` anchors it may not contradict, and it governs all ordinary
specs (`001`+).

**Normative hierarchy (highest wins):**

1. `specs/000-spec-spine-bootstrap/spec.md` — the bootstrap spec. Non-overridable.
2. `standards/spec/constitution.md` — this document.
3. `standards/spec/contract.md` — a normative summary of the bootstrap spec.
4. Ordinary specs (`001`+) — feature-level claims within this envelope.

When two specs conflict, resolve in this order, then by the typed authority graph.

---

## I. Markdown-only authored truth

Authored truth lives only in markdown with YAML frontmatter. There is no
authoritative hand-authored JSON or YAML data file. If a fact governs the system,
it is written in a `spec.md` (or a `standards/` document), never in a derived
artifact. *(Bootstrap anchor: `markdown-truth-boundary`.)*

## II. Compiler-owned JSON machine truth

All machine-consumable truth is emitted by the compiler/indexer into the derived
output tree, and is read only through a typed consumer (the `spec-spine` binary
or the `spec-spine-core` library). Hand-editing a derived artifact is a workflow
violation, and ad-hoc parsing of one (`jq`/`awk`/`sed`) is equally forbidden:
typed reads make schema drift fail at the deserializer, with a clean error,
instead of silently somewhere downstream. *(Bootstrap anchors: `json-truth-boundary`.)*

## III. Spec-first development

A change to behavior begins with a change to a spec. The spec defines the
territory (the units it owns) and the relationships (the typed edges) before the
code is written. The coupling gate enforces this at PR time: a claimed code unit
that changes without its owning `spec.md` changing — or vice versa — refuses the
merge. The escape valve is a named, scoped waiver recorded in the PR body, never
a silent edit to an owner spec.

## IV. Determinism and validation

Every artifact-producing function is a pure function of `(config, file
contents)`; the same inputs produce byte-identical output. Validation is
mechanical: the compiler reports violations and sets a pass/fail flag; the lint
reports conformance warnings; the coupling gate reports drift. No artifact
carries an ambient clock or environment read except the excluded `builtAt` field.
*(Bootstrap anchor: `determinism-requirement`.)*

## V. Legacy as evidence

Code that predates a governing spec is not a violation to be erased — it is
evidence. A spec that claims authority over pre-existing code declares
`origin.retroactive: true` to record that it holds authority it has had since
before the graph existed, rather than masquerading as a fresh `establishes`
claim. History is queryable: "who established this unit" and "who currently owns
it" are different questions, and an amendment patches its predecessor in place
rather than blowing away its history.

---

## Amendment

This constitution may be amended by an ordinary spec that `amends` it and is
approved, **provided** the amendment does not contradict a `specs/000`
`unamendable` anchor. The bootstrap spec's freeze surface is the hard boundary;
everything else in this document is revisable through the normal governed flow.
