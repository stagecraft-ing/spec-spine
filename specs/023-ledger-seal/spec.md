---
id: "023-ledger-seal"
title: "Ledger Seal (signed, reproducible attestation over the spec corpus)"
status: approved
created: "2026-06-16"
authors: ["The spec-spine Authors"]
kind: tooling
implementation: complete
risk: medium
summary: >
  spec-spine self-describes as a typed, hash-verifiable authority ledger over a
  markdown spec corpus. A hash-verifiable ledger is an attestation missing only
  two things: a signature and a verify. This spec completes that sentence. It
  adds a CorpusAttestation, a deterministic, pure-function payload over the
  inputs manifest, the registry hash, and the compile/lint (and optionally
  coupling) verdicts, plus a detached Ed25519 seal over its hash, and a two-mode
  verify-attestation (recompute, which any third party can run with no key, and
  signature, which is opt-in). The reproducible recompute mode is the property a
  run certificate can never have, because runs are side-effecting; this object is
  the signed, archival, independently-verifiable form of a verdict spec-spine
  already computes and then throws away into a CI log. It is deliberately NOT
  called a certificate: that word stays with run-provenance (signed testimony of
  a non-reproducible event). This is an attestation (recomputable truth).
depends_on:
  - "001-compile-registry"      # emits registry.json (the hashed ledger)
  - "003-conformance-lint"      # the lint verdict
  - "004-codebase-index"        # the index_hash recorded under --with-coupling
  - "005-coupling-gate"         # the in-sync verdict (FR-002, optional scope)
establishes:
  - "crates/spec-spine-types/src/attest.rs"
  - "crates/spec-spine-core/src/attest.rs"
  - "crates/spec-spine-core/tests/attest.rs"
  - "crates/spec-spine-cli/src/seal.rs"
  - "crates/spec-spine-cli/src/cmd_attest.rs"
  - "crates/spec-spine-cli/src/verify_attestation.rs"
extends:
  # Additive surface in files other specs established: the public re-exports, the
  # CLI dispatch frame, and the CLI manifest (the Ed25519 dependency).
  - { spec: "000-spec-spine-bootstrap", unit: "crates/spec-spine-types/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-core/src/lib.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/src/main.rs", nature: additive }
  - { spec: "001-compile-registry", unit: "crates/spec-spine-cli/Cargo.toml", nature: additive }
references:
  # The (config, file contents) pure-function claim this capability rests on.
  - { unit: { kind: file, path: "docs/design/00-architecture.md" }, role: context }
---

# 023: Ledger Seal

Filed off the OAP ADR 0002 boundary analysis. ADR 0002 confirmed the run
certificate is run-provenance (OAP's factory-engine domain), not spec-spine's
authority-ledger domain. The same analysis surfaced a distinct object it had
conflated: an attestation over the spec corpus state itself, which is native to
spec-spine and not addressed by the run cert. This spec owns that object.

## 1. Purpose

spec-spine is a hash-verifiable authority ledger. Today the verdict it computes
(compiles clean, lint-clean, coupling-green, at registry hash Y) is ephemeral: it
passes in CI and evaporates into a log. A live gate proves the present and forces
re-execution every PR, which is correct for CI. It cannot let a future party
trust a prior clean state without re-running the gate, which is what handoff and
audit need.

The corpus attestation is the frozen, signed snapshot of that verdict. The
producer of a verdict is the natural signer of it, so no one downstream has to
re-run or re-trust the computation, and, uniquely, anyone can, because spec-spine
is a pure function of `(config, file contents)` and the attestation is therefore
reproducible bit-for-bit. That makes the corpus link the most verifiable link in
any audit chain that contains it: a live gate proves the present; a sealed
attestation lets a future steward trust the past without re-trusting the prover.

A ledger without a seal decays the same way a covenant without a gate decays: the
moment it is handed off, "this corpus is governed and consistent" becomes a claim
no one can check after the fact. This spec is the seal.

This capability adds no new ledger semantics: every verdict it freezes is one
spec-spine already computes (`compile`, `lint`, `couple`). It is the signing and
the verify, nothing more.

## 2. Territory

This spec establishes the attestation surface across the three-crate layout, and
additively extends three existing files it must touch, honouring FR-003's "the
pure core gains no key handling":

- `crates/spec-spine-types/src/attest.rs`: the plain-data DTOs
  (`CorpusAttestation`, the verdict structs, `LedgerSeal`) and
  `ATTESTATION_SCHEMA_VERSION`. No crypto, no clock: the type substrate stays
  key-free.
- `crates/spec-spine-core/src/attest.rs`: the pure `CorpusAttestation` builder
  and the `--recompute` verifier, alongside the existing compile/lint/couple
  engine. No key, no clock, no env.
- `crates/spec-spine-cli/src/seal.rs`: the detached Ed25519 seal (`--sign`) and
  its signature check, a post-pass over an already-emitted hash. The only place
  spec-spine handles a key, kept out of the core.
- `crates/spec-spine-cli/src/cmd_attest.rs` and
  `crates/spec-spine-cli/src/verify_attestation.rs`: the `attest` and
  `verify-attestation` verb wiring (recompute delegates to core; the signature
  check uses the seal module).
- `crates/spec-spine-core/tests/attest.rs`: determinism, recompute, and
  coupling-scope acceptance tests.

It additively extends (no behavior change to the owner) the public re-export
surfaces (`spec-spine-types` and `spec-spine-core` `lib.rs`), the CLI dispatch
frame (`main.rs`), and the CLI manifest (`Cargo.toml`, for the Ed25519
dependency). The schema-version axis for the attestation is its own
(`ATTESTATION_SCHEMA_VERSION`), independent of the registry and index lines.

## 3. Object shape

```
CorpusAttestation            # pure function of (config, file contents).
                             # No clock, no env, no key. Reproducible.
  schemaVersion
  tool:        { name: "spec-spine", version: "<x.y.z>" }   # reproducibility anchor (FR-005)
  inputsManifestHash:  sha256 over what spec-spine read (the registry's input hash)
  registryHash:        sha256(canonical registry.json)
  verdicts:
    compile:  { ok: true }
    lint:     { ok: true, findingsHash: ... }
    couple:   { ok: true, indexHash: ..., joinHash: ... }   # present iff --with-coupling (FR-002)

# Emitted alongside, never inside, the attestation:
attestationHash:  sha256(canonical(CorpusAttestation))      # the chain handle a consumer references
LedgerSeal                   # detached. Produced only by --sign (FR-003).
  alg:        ed25519
  keyId
  signedAt                   # the timestamp lives HERE, out of the pure payload
  sig:        ed25519(attestationHash)   # over the 32-byte hash, lowercase hex
```

`inputsManifestHash` is content-derived rather than a git SHA so the payload
stays git-agnostic (the manifest is exactly what the pure function already
reads). `signedAt` and `keyId` live in the detached seal, so the attested fact
stays reproducible while the act of attesting carries its own non-reproducible
identity and time.

## 4. Functional requirements

- **FR-001 (corpus attestation, pure).** `spec-spine attest` emits a
  `CorpusAttestation` over the inputs manifest, the registry hash, and the
  compile and lint verdicts. It is a pure function of the same
  `(config, file contents)` the compiler consumes: no clock, no env, no key.
  Re-running `attest` on an unchanged corpus at the same tool version yields a
  byte-identical payload.
- **FR-002 (optional coupling scope).** `attest --with-coupling` additionally
  records the in-sync verdict against named `indexHash` and `joinHash`. Without
  the flag the attestation covers spec-corpus state only (the cleanest,
  code-independent claim). With it, the attestation covers
  specs-and-code-in-sync (the stronger handoff claim, dependent on the code
  tree); `ok` is true iff every claimed unit resolves with no blocking resolver
  diagnostic. Both are first-class and distinguishable by the presence of the
  coupling block; the scope is never silently widened.
- **FR-003 (detached seal: signing is a separable wrapper).** `attest --sign`
  produces, alongside the attestation, a detached Ed25519 signature over
  `attestationHash`. The compiler core gains no key handling: signing is a
  post-pass over an already-emitted hash, in the CLI. `keyId` and `signedAt` live
  in the seal envelope, never in the payload.
- **FR-004 (two-mode verify).** `spec-spine verify-attestation` supports two
  independent modes. `--recompute` re-reads the corpus, recompiles, and checks
  the emitted hashes against the attestation: no key, no signature, offline,
  runnable by any third party. `--signature` checks the detached seal against a
  supplied public key: offline, opt-in. The recompute mode is the property the
  run certificate structurally cannot have; it is the load-bearing reason this
  object exists.
- **FR-005 (version-aware verification).** Recompute is reproducible only under
  the tool version that produced the attestation. `verify-attestation
  --recompute` reads `tool.version` and either recomputes under that pinned
  version or reports a version mismatch as a distinct, named outcome: never a
  false content mismatch, never a skip-as-pass.
- **FR-006 (degraded-mode visibility).** A mode that cannot run (no key for
  `--signature`, unreadable corpus for `--recompute`, no mode selected) fails
  visibly with reason. Skip-as-pass is forbidden, consistent with the ledger's
  fail-closed posture.

## 5. Acceptance criteria

- **AC-1.** `attest` run twice on an unchanged corpus at one tool version
  produces byte-identical attestation payloads.
- **AC-2.** `verify-attestation --recompute` accepts an attestation whose corpus
  is unchanged; a single spec-file edit flips it to a named mismatch that cites
  the changed input.
- **AC-3.** `attest --with-coupling` records the in-sync verdict; removing a
  claimed code unit fails the coupling block while the registry/lint block still
  verifies: the two scopes are independently checkable.
- **AC-4.** `attest --sign` then `verify-attestation --signature` round-trips
  against the public key; a single tampered payload byte fails the signature; an
  unsigned attestation still passes `--recompute` (signing is pure-upside, never
  a gate on truth).
- **AC-5.** Verifying an attestation produced by a different tool version yields
  the named version-mismatch outcome, not a false content mismatch.

## 6. Out of scope

- **The run certificate and its chain edge.** OAP's factory-engine run cert may
  reference this attestation by hash via an additive
  `corpus_binding.corpus_attestation_hash` field. That field, and the
  read-not-recompute invariant that protects the boundary, are an OAP-side change
  owned by **OAP spec 218-run-cert-corpus-binding**, not this spec. Stated here
  only to mark the seam: spec-spine computes the attestation and publishes its
  hash; the consumer reads that hash and never recomputes it. If a consumer
  recomputes corpus state to obtain the hash, it has absorbed the compiler and
  the boundary is gone. That invariant is enforced as a gate on the consumer side
  (OAP spec 218), where it lives.
- **Signer identity / key provisioning.** The seal needs a key; where an
  operator's or tenant's key comes from is unspecified here and is non-blocking,
  because an unsigned attestation is already reproducibly true. (Contrast the run
  cert, whose value collapses without a signer; here the signer is pure upside.)
- **Transparency log, countersignature, external timestamping.** A detached
  Ed25519 seal is the floor; richer trust roots are later and additive.
- **An embedded JSON Schema + conformance test for the attestation.** The
  payload is canonical-JSON and version-stamped (`ATTESTATION_SCHEMA_VERSION`)
  and proven deterministic by test; a formal schema artifact mirroring the
  registry/index conformance gate is a later, additive step.

## 7. Naming (deliberate)

This object is an attestation (recomputable truth) and is deliberately not called
a certificate. "Certificate" stays with run-provenance: signed testimony of a
non-reproducible event. Keeping the words apart is what prevents the two objects
from reconflating, which is precisely the conflation ADR 0002's analysis had to
untangle.

## 8. Sequencing

Independent of the run-cert distribution work (OAP ADR 0002 / OAP spec 219). This
capability rides the npm pin every tenant already has, adds no release matrix,
and touches none of OAP's overlay, so it neither blocks nor is blocked by OAP's
spec-spine minimization. It is a generic ledger capability (sign your own
verdict) carrying none of the OWASP / stage-id / pipeline-state baggage that is
OAP overlay territory (see `docs/design/00-architecture.md`, "Deliberately
dropped from the generic core").

Order within the spec: `attest` (pure, FR-001/002) first, with standalone value
and no key story; then `--sign` and `verify-attestation` (FR-003/004) as the
separable wrapper; then the optional coupling scope. The run-cert chain edge
(out of scope above) lands as a small additive change in OAP's corpus (spec 218)
whenever this lands.
