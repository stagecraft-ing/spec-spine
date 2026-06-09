# spec-spine: the concept

> The origin story and the model behind the crate. This is the *why*; for the
> *how*, see [adoption-guide.md](adoption-guide.md) (using it),
> [api.md](api.md) (the library), and
> [design/00-architecture.md](design/00-architecture.md) (the design). Where this
> document describes the full vision, the README's capability table marks what v1
> actually ships.

**A typed, hash-verifiable ledger of who-owns-what, sitting underneath a codebase
so that many agents can work in parallel without trampling each other.**

What looks from the outside like a pile of markdown is a contract surface with
formal ownership semantics. The contract is the substrate, a deterministic
compiler enforces its shape, a typed read layer enforces how it is read, gates
enforce its truthfulness at PR time, and a refusal rule enforces its
truthfulness at prompt time. Remove any one layer and the others stop being
sufficient.

---

## The problem it solves

Unconstrained agentic output is unprocessable. A human will not review every
line an agent produces, and pretending otherwise just moves the bottleneck back
to the human. The only honest move is to stop reviewing output and start
constraining intent.

Intent becomes a requirement. The requirement defines a spec. The spec is law.

Everything downstream (the compiler, the registries, the coupling gate, the
refusal rule) is mechanical enforcement of that law. The human writes the
contract once; the machinery enforces it on every diff, forever. This is what
lets one person sit at the helm of a development effort and steer it without the
structure becoming incoherent or drifting from the original intent. The human
authors the law; the agents comply with it; the spine makes non-compliance
impossible to merge.

Treat all agentic output as hostile by default. Agents earn passage by surviving
the gates, not by appealing to trust. When the work is large enough to need many
of them, pit them against each other: divide the territory, type the boundaries,
let the spine arbitrate. Parallel agents do not need to cooperate; cooperation is
a property of the substrate, not a virtue the agents have to share.

---

## The core idea

Every piece of work in a repository, every feature, every refactor, every
infrastructure change, is anchored to a small markdown document that declares its
territory. The territory is not a vague description; it is an explicit list of
code paths plus a set of *typed relationships* to the other documents that have
touched those paths before. Together those documents form a graph, and the graph
is the source of truth about who is allowed to change what.

### What ties the documents together: typed edges

A document does not just claim "I exist." It declares, in machine-readable
frontmatter, its relationships to the rest of the corpus:

- **establishes**: I am the document that first brought this code into being.
- **extends**: I add surface to a predecessor's territory without disturbing it.
- **refines**: I tighten behavior on a specific aspect.
- **supersedes**: I replace this predecessor, partially or fully.
- **amends**: I patch a predecessor in place (clarification, correction, restriction).
- **co_authority**: I share a path with another document on a named section.
- **constrains**: I assert an invariant that everyone else must respect.
- **references**: I point at another document without claiming authority over it.

These edges turn the corpus from a flat pile of claims into a directed graph.
Authority over any given path is *derived* by walking the graph, not declared
directly. "Who currently has authority over file X, function Y?" is a query
against the graph, not a guess.

### Why parallel work becomes safe

Three properties fall out of this design:

1. **Disjoint territory is provably disjoint.** Two agents working on documents
   that establish or refine non-overlapping paths cannot collide by
   construction. The graph tells them so before either edits a line.
2. **Shared territory is typed, not undefined.** When two documents touch the
   same path, say a project-wide build file where many features add targets, they
   declare co-authority section-by-section, with named anchors. The collision
   becomes a structured merge, not a free-for-all.
3. **History is queryable.** "Who established this file" and "who currently has
   authority over it" are different questions. An amendment does not blow away its
   predecessor; it patches it in place, and consumers see the patched view. Two
   agents can refine different aspects of the same predecessor in parallel without
   one having to wait for the other.

### The four guardrails

Sitting on top of the graph:

1. **A deterministic compiler** reads the markdown corpus and emits a frozen JSON
   registry. The same committed inputs must produce byte-identical output. Two
   agents producing changes independently produce diffable,
   mechanically-mergeable registries: there is no interpretation drift at merge
   time, and staleness is detectable by content-hash comparison alone.
2. **A coupling gate at PR time** cross-references every modified code path
   against the graph. If an agent changes a path without changing the document
   that has authority over it, or vice versa, CI refuses the merge. This is the
   mechanism that catches silent drift before it lands.
3. **Typed reads, or nothing.** Reads of the compiled JSON go through the typed
   query layer (the library API and the `spec-spine registry` subcommands), never
   ad-hoc parsing. Ad-hoc parsing over compiled JSON would let an agent silently
   encode schema assumptions; schema drift would then fail loudly somewhere else,
   not at the read. Typed reads make drift fail at the deserializer, with a clean
   error. A shipped agent rule (`governed-artifact-reads`) encodes this as a
   workflow constraint.
4. **A refusal rule at prompt time** prevents an agent from "resolving" a
   coupling-gate failure by quietly editing the contract to match the code it just
   wrote. The agent must surface the contradiction and let a human (or another
   agent with explicit authority) decide. Without this, the long-running failure
   mode is an agent erasing the contract to keep going. The coupling gate is the
   PR-time defense; the refusal rule is the prompt-time defense; together they
   sandwich the failure mode.

---

## The constitutional layer: three tiers of authority

Not every document is equal. Conflicts resolve in a fixed order (highest wins):

1. **The bootstrap spec**: the spec that defines what a spec is. It bootstraps
   the corpus; its invariants are non-overridable.
2. **The constitution**: durable principles (markdown-only authored truth,
   compiler-owned JSON, spec-first development, determinism,
   legacy-as-evidence). Subordinate to the bootstrap spec where they differ.
3. **Ordinary specs**: feature-level claims operating within the constitutional
   envelope.

---

## Authority is over units, not just files

The graph expresses ownership at finer granularity than "file." A unit can be:

- a **file** (the default),
- a **section** within a file, a named anchor such as a particular build target
  or a Markdown heading,
- a **symbol**, a function, a type, an exported binding (resolved via
  tree-sitter).

Section-scoped co-authority is the property that makes the canonical hard case
tractable: a project-wide build file where many features each add targets.
Co-authority is section-scoped, not file-scoped.

---

## Two registries, two directions

- **The spec-as-source view**: what does each spec say? For each spec: its
  status, its relationships, the paths it claims. Read through the registry query
  layer (`list`, `show`, `status-report`, relationship and authority queries).
- **The code-as-source view**: for each path / section / symbol in the repo,
  which spec(s) currently claim authority over it? Built by the codebase indexer
  (`spec-spine index`), with `index check` detecting staleness by content hash.

They are inverses. The coupling gate joins them at PR time and refuses the merge
if they disagree.

### How code connects back to specs

Each compilation unit declares its owning spec in its manifest, via a single
configurable metadata key: for Rust, a key under `[package.metadata.<ns>]`; for
an npm package, a top-level `"<ns>"` object; analogous conventions for other
ecosystems. The indexer walks the tree, hashes those manifests along with the
spec files, and builds the inverse map. A query layer backs the
`authorities(unit)` function, *who currently owns this unit?*, for both the
coupling gate and any other consumer that needs the same answer.

---

## The pre-merge gate chain

Not a single check, a chain:

- **A local refresh step** rebuilds the codebase index and runs the coupling
  check against the merge base. These are the two checks that fail first in CI
  when forgotten.
- **CI: the spec/code coupling check**, the absolute floor.
- **CI: the conformance lint**, corpus well-formedness, fail-on-warn.
- **CI: index freshness**, the committed index must match current inputs.

### The waiver: the gate's escape valve, itself governed

The coupling gate would be tyranny without an escape valve, and the escape valve
itself has to be in the ledger:

- **A named waiver** declared in the PR body: explicit, scoped, citing the reason
  it applies. The blessed path for legitimate consolidated changes, for example a
  dependency refresh that touches many owned paths. Never amend an owner spec just
  to satisfy a mechanical refresh; waive instead.
- **Amends-aware coupling**: an amendment to a predecessor's paths is recognized
  as legitimate authority, not drift.

---

## Lifecycle and the retroactive bootstrap

Specs carry a `status`: `draft`, `approved`, `superseded`, `retired`. A
superseded spec retains its `establishes` history but loses current authority;
the superseding spec inherits it.

For specs whose claimed paths predate the relationship graph, frontmatter carries
`origin: retroactive: true`. Without it, every pre-graph spec would look like a
fresh `establishes` claim and the history would be wrong. This is the bootstrap
marker for "I am declaring authority I have held since before the graph existed,"
and it is reserved for genuine foundational bootstrap.

---

## What you get

Put the pieces together and the corpus stops behaving like documentation and
starts behaving like a typed, hash-verifiable, append-only ledger of
who-owns-what:

- Agents query the ledger to find their territory.
- Agents edit code and specs; the compiler re-mints the ledger.
- Agents open a PR; the coupling gate verifies code and ledger agree.
- Agents face a contradiction; the refusal rule stops them from rewriting the
  ledger to escape.
- A human authorizes a waiver; the waiver is itself recorded and citable.

This is **L4 first-class agentic software engineering**: not AI-assisted coding,
not copilots, not humans-in-the-loop on every diff, but a delegated execution
model where the human governs the contract and the contract governs the work.
