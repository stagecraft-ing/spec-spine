# 01: Partial supersession (structured `supersedes`)

**Status:** decided and shipped. Option A was implemented as spec
`019-structured-partial-supersedes` (approved, complete). This document is the
historical design analysis that preceded the decision; the analysis and
algorithm sketch below match what was built, but the "open question" and staged
recommendation framing is superseded (see the postscript at the end).
**Origin:** the Phase-0 OAP corpus dry-run (read-only, throwaway config). Five
specs author `supersedes` items in a structured `{ spec, scope: partial, unit }`
form that this library's grammar does not accept. Unlike the other dry-run gaps
(013 declared passthrough, 014 `paths:` sugar, 015 `unit:` wrapper + `n/a`
alias, 016 short-id resolution), this one is **not** normalize-away sugar: it
changes the meaning of the authority graph and the coupling gate's transfer
rule. The amendments README Tripwire is explicit -- *"surface rather than widen
grammar"* -- so it is captured here for a deliberate call rather than chased
into the parser.

This document is decision-support: it states the gap, recaps the current model
exactly, defines what "partial" must mean, lays out the options with trade-offs,
and gives a recommendation. It deliberately does **not** change any code or
grammar.

---

## 1. The observed shape

The five specs write (paraphrased, OAP literals removed):

```yaml
supersedes:
  - { spec: "042-old-thing", scope: partial, unit: "src/foo/bar.rs" }
```

Read in English: *"this spec takes over spec 042's authority over
`src/foo/bar.rs` specifically; 042 keeps everything else it owns."* Contrast
the full form this library accepts today:

```yaml
supersedes: ["042-old-thing"]   # takes over ALL of 042's authority
```

`scope: partial` with a `unit:` is a **scoped authority transfer**. `scope:
full` (or the bare string) is the whole-spec transfer already implemented.

## 2. Why this is semantics, not sugar

Every amendment 013-016 shares one property: the new spelling **normalizes away
at parse time** to an existing wire shape, so `registry.json`, its schema, and
every consumer are untouched (the byte-equivalence golden is each spec's
acceptance test). Partial supersession cannot do that. There is no existing wire
shape that means "transfer authority over unit U only" -- the registry models
`supersedes` as a flat `Vec<String>` of predecessor ids, with no place to record
*which* of the predecessor's units transferred. Accepting the grammar therefore
forces a choice that the sugar amendments never had to make: either change the
registry schema and the gate algorithm to carry the scope, or discard the scope
(which would be silently wrong -- see Option C).

## 3. The current model (exactly as built)

This is the single most battle-tested algorithm in the port; it is described in
`docs/design/00-architecture.md` §2.4 and lives in `crates/spec-spine-core/src/
couple.rs`. The relevant pieces:

- **Grammar / registry.** `supersedes: Vec<String>` -- a list of predecessor
  spec ids. The architecture edge table (§2.1) already labels the edge
  *"replaces a predecessor (partial/full); inherits current authority"*, so
  "partial" was a **named-but-unbuilt** capability at the time of this analysis,
  not a foreign concept. (Both forms are now implemented as of spec 019; this
  section describes the pre-019 baseline.)
- **`build_superseders`** (`couple.rs`): folds the registry into a
  `predecessor_id -> { superseder_ids }` map. The key is a bare id; there is no
  slot for a unit qualifier.
- **`owners_for_path` step 2** (the transfer): for every candidate owner of a
  changed path, *all* of that owner's transitive superseders are added as
  legitimate owners of that path, with the same span. The transfer is keyed only
  on the predecessor id, so it moves the predecessor's **entire** authority
  surface to the superseder. (Transfer is additive: the predecessor is not
  removed as an owner.)

Partial supersession is precisely a change to that step 2: the transfer must
fire **only** for paths that resolve to the named `unit`, not for everything the
predecessor owns.

## 4. What "partial" must mean (the semantics to get right)

If spec B declares `supersedes: [{ spec: A, scope: partial, unit: U }]`:

1. B becomes a legitimate owner of the paths that `U` resolves to (exactly the
   span semantics units already have).
2. A's authority over everything **other than** `U` is unaffected.
3. Open sub-question: does A **retain** authority over `U` (transfer is
   additive, mirroring the current full-supersedes behavior where the
   predecessor is not removed), or does A **lose** it (true hand-off)? The
   current full-supersedes is additive (predecessor kept). The most consistent
   choice is additive here too -- B *gains* authority over U, A is not stripped
   -- which also keeps the gate's "any one owner clears" rule monotonic. A true
   exclusive hand-off would be a larger change (the first place the graph ever
   *removes* an owner) and should be out of scope for a first cut.
4. Chains and the transitive closure (`transitive_superseders`) must respect the
   scope: a partial transfer of U does not give B's *own* superseders authority
   over A's other units.

## 5. Options

### Option A -- Implement structured `supersedes` faithfully

Widen the grammar to `supersedes: Vec<SupersedeItem>` where an item is **either**
a bare string (full, unchanged) **or** `{ spec, scope?, unit? }`. Wire the scope
through `build_superseders` and `owners_for_path` step 2 so a partial item
transfers authority only for its `unit`.

- **Registry/schema.** A union entry (string | object). Critically, keeping the
  bare-string form for full supersession means **every existing registry stays
  byte-identical** -- only specs that actually use the partial form emit the
  object. This is an additive MINOR schema bump (like 012's `sliceHashes` and
  013's value widening), and preserves determinism for all current corpora.
- **Algorithm.** `build_superseders` gains a per-edge optional unit; step 2's
  transfer becomes conditional on the candidate path matching that unit. This is
  a change to the most-tested function in the gate, so it needs fresh
  golden/property tests: partial transfer fires for U, does **not** fire for the
  predecessor's other paths, and chains stay scoped.
- **Cost / risk.** Highest of the options. Touches `couple.rs` (gate core),
  `edges.rs`/`registry.rs` (grammar + wire types, spec 000 territory), the JSON
  Schema artifact, and the index pre-flattening if it materializes supersedes
  units. Deserves the review attention 009 got (it changes gate semantics).
- **Fit.** Clean: it builds the capability the architecture already names.

### Option B -- Re-express partial supersession with an existing edge

Partial supersession of unit U is *arguably* one of the edges the grammar
already has. The adopter rewrites the 5 specs:

- `extends: [{ spec: A, unit: U }]` -- "B adds authority over U on top of A." But
  `extends` does **not** imply A ever held U as a supersedable surface, and it
  carries an "additive, non-disturbing" nuance that may misdescribe a takeover.
- `co_authority` -- only fits if U is a named section and shared ownership (not
  takeover) is intended.

- **Cost.** No library change; adopter migrates 5 specs at refactor time.
- **Risk.** Semantic drift: none of the existing edges means *"B takes over A's
  authority over U"* exactly. `extends` is the closest and is probably
  *good enough* for the gate's purpose (it makes B a legitimate owner of U), but
  it loses the lifecycle intent ("this was superseded") that a reader/registry
  query would want. Acceptable if the intent is purely about clearing the gate.

### Option C -- Accept the grammar, treat `scope: partial` as full (lossy)

Parse the structured form but drop the scope, transferring the whole spec.
**Rejected.** It silently grants B authority over *all* of A -- the opposite of
what the author wrote -- and would clear gate violations it should not. A
correctness hazard masquerading as compatibility.

### Option D -- Reject with a precise diagnostic, document the migration

Keep the grammar as-is; when a `supersedes` item is a structured object rather
than a string, emit a clear, dedicated error (a new V-code) whose message names
this document and the two interim paths (rewrite as `extends` per Option B, or
wait for Option A). This is the status quo's behavior made *intentional and
legible* instead of a generic malformed-frontmatter error.

- **Cost.** Tiny (a parse-time branch + message). Keeps the library generic.
- **Risk.** None to the gate. Pushes the decision to the adopter per-spec.

## 6. The "second adopter" test

The Tripwire asks: *would a second adopter want this, or is it OAP-specific?*
Partial supersession passes -- it is a generic lifecycle pattern (one spec takes
over part of another's surface), and the architecture **already reserves it**
("partial/full" in the §2.1 edge table). So Option A is not grammar emulation;
it is finishing a designed capability. The hesitation is **evidence volume**:
five specs in one pre-adoption corpus is thin grounds for changing the gate's
core algorithm. The genericity is real; the demand signal is not yet strong.

## 7. Recommendation

**Short term: Option D** (reject with a precise, document-linked V-code). It is
cheap, keeps the library generic, turns a confusing parse failure into a clear
signpost, and unblocks nothing it should not. Pair it with a note in the
adopter's migration guide that the 5 specs choose per case between *Option B*
(rewrite as `extends` when the intent is just "B now owns U") and *waiting for
Option A* (when the supersession lifecycle intent matters).

**When evidence crosses a threshold (a second adopter, or the lifecycle intent
proves load-bearing): Option A**, implemented as a deliberate `017` with the
union-schema shape in §5 (full stays a bare string -> all current registries
byte-identical), additive transfer semantics (§4.3), scoped chains, and the
009-level review the gate-core change warrants. The schema and algorithm sketch
here are meant to make that spec fast to write when the time comes.

**Not recommended:** Option C (silently wrong), and rushing Option A on
five-spec evidence (changing the most battle-tested function in the port for a
demand signal this small).

## 8. Open questions for the maintainer

1. **Additive vs exclusive transfer (§4.3):** for a first cut, is partial
   supersession additive (B gains U, A keeps it -- consistent with today's
   full-supersedes) acceptable, deferring true exclusive hand-off?
2. **Interim path:** do you want Option D's dedicated V-code filed now as a
   small spec (it is arguably sugar-adjacent: a better error, no semantic
   change), or left until the OAP refactor actually hits the wall?
3. **Threshold for Option A:** what is the trigger -- a second adopter, a count
   of partial-supersedes specs, or an explicit product decision -- that promotes
   this from "reserved" to "build it"?
4. **`scope: full` spelling:** if A is built, do we also accept an explicit
   `{ spec, scope: full }` object (symmetry), or keep full as bare-string-only
   to guarantee registry stability?

## 9. Outcome (postscript)

The decision was taken and **Option A shipped directly as spec
`019-structured-partial-supersedes`** (approved, complete), not the staged
Option-D-then-A path recommended in §7, and not under the `017` number used as a
placeholder above (`017` became the unrelated directory/crate/module-units
spec). The implementation matches the §5 sketch:

- **Wire format** (`spec-spine-types/src/edges.rs`): `supersedes` is a
  `Vec<SupersedeItem>`. A bare predecessor id and an explicit
  `{ spec, scope: full }` both normalize to `SupersedeItem::Full`, so existing
  registries stay byte-identical (answering open question 4: symmetry accepted,
  full canonicalizes to the bare form). A partial item is
  `{ spec, scope: partial, unit, note?, rationale? }` -> `SupersedeItem::Scoped`.
- **Transfer semantics:** additive (open question 1) -- `build_superseders` in
  `couple.rs` branches on `SupersedeItem::is_full()`, so a partial item transfers
  only the scoped unit's authority rather than the predecessor's whole surface.

This document is retained as the design analysis that justified that shape; the
analysis stands, only its "open question" framing is historical.
