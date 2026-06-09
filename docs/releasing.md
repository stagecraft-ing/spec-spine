# Releasing spec-spine (maintainers)

> Two distribution paths ship together: **crates.io** (the library + CLI, and the
> path that unblocks bindings) and **prebuilt binaries** (the `curl | sh` install
> path). This is the maintainer runbook. Adopters do not read this; they read
> [adoption-guide.md](adoption-guide.md).

## Pre-flight

- [ ] Working tree clean and green: `cargo test --workspace --locked`.
- [ ] Self-governance green: `spec-spine compile && spec-spine index check &&
      spec-spine lint --fail-on-warn && spec-spine couple --base origin/main --head HEAD`.
- [ ] Versions bumped consistently (workspace `version` in the root `Cargo.toml`;
      schema-version constants in `spec-spine-types` per
      [schema-versioning.md](schema-versioning.md) if the schema changed).
- [ ] `cargo package --workspace --locked` succeeds (it cross-verifies every
      crate from its packaged sources, in dependency order: the same check CI can
      run).

## 1. crates.io: publish in dependency order

The crates depend on each other, so they must be published **leaf-first**. A
later crate can only publish once its dependency is live on the index:

```sh
cargo publish -p spec-spine-types     # 1. the leaf (DTOs, Config, schemas, Error)
cargo publish -p spec-spine-core      # 2. the engine (depends on types)
cargo publish -p spec-spine-cli       # 3. the binary (depends on core + types)
```

> **Order is load-bearing.** `cargo package -p spec-spine-core` fails with
> "no matching package named `spec-spine-types`" until `types` is on the index;
> that is expected, not a defect. Publish `types`, let the index update, then
> `core`, then `cli`. `cargo install spec-spine-cli` must then yield a working
> `spec-spine` binary.

All three crates are publish-clean by construction: full metadata
(`license = "Apache-2.0"`, `repository`, `homepage`, `description`, `keywords`,
`categories`), per-crate `README.md`, internal deps carry both `version` and
`path` (never path-only), and **no `publish = false`** on any shipped crate.

## 2. Prebuilt binaries: push a tag

The release workflow (`.github/workflows/release.yml`) is tag-gated:

```sh
git tag v0.1.0
git push origin v0.1.0
```

That builds a per-triple archive (with a `.sha256` sidecar) for all five
supported targets, `x86_64`/`aarch64` `apple-darwin`, `x86_64`/`aarch64`
`unknown-linux-gnu`, `x86_64-pc-windows-msvc`, each natively on a matching
GitHub-hosted runner, and attaches them to the GitHub Release.
[`install.sh`](../install.sh) (`curl | sh`) consumes those assets.

Per-archive CycloneDX SBOM is deferred for v1 (low value/time ratio); add to the
release workflow on request.

## 3. Determinism gate

`.github/workflows/determinism.yml` proves the emitted `registry.json` and
`index.json` (including tree-sitter symbol line-spans) are **byte-identical
across all five triples**: the empirical backstop for the
"identical-on-every-triple" claim, beyond merely pinning the grammars exact. Keep
it green; a span drift on any platform fails this gate.

## Schema / version bumps

Follow [schema-versioning.md](schema-versioning.md): MINOR = additive only;
MAJOR = breaking (loaders reject an unknown MAJOR). Bump the schema-version
constants and the embedded schemas together so the conformance test (which fails
the **build** on mismatch) stays the source of truth.
