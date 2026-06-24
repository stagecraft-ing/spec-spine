---
id: releasing
title: Releasing
sidebar_position: 9
---

# Releasing (Maintainer Runbook)

This document is the runbook for project maintainers. Adopters do not need to read this.

Four distribution paths ship together: **crates.io**, **prebuilt binaries**, **npm**, and **PyPI**.

## Pre-flight

1. Ensure the working tree is clean and tests pass (`cargo test --workspace --locked`).
2. Verify self-governance (`spec-spine compile && spec-spine index check && spec-spine lint --fail-on-warn && spec-spine couple --base origin/main --head HEAD`).
3. Bump versions consistently using `scripts/bump_version.py <version>`.
4. Ensure `cargo package --workspace --locked` succeeds.

## 1. crates.io: Publish in dependency order

Crates must be published **leaf-first**:

```bash
cargo publish -p spec-spine-types
cargo publish -p spec-spine-core
cargo publish -p spec-spine-cli
```

Wait for the index to update between publishes.

## 2. Prebuilt binaries: Push a tag

The release workflow is tag-gated.

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

This builds per-triple archives (with `.sha256` sidecars) for five supported targets and attaches them to the GitHub Release. It also generates a CycloneDX SBOM and a SLSA build-provenance attestation.

## 3. npm: The binary-distribution shim

The same `v*` tag drives the `publish-npm` job. It does not rebuild Rust; it repackages the archives as npm packages.

- A main package `spec-spine` with a tiny launcher.
- Five platform packages `@spec-spine/cli-<os>-<cpu>` listed as `optionalDependencies`.

The job requires the `NPM_TOKEN` secret.

## 4. PyPI: The wheel shim

The `publish-pypi` job repackages the archives as five platform wheels and one sdist under the `spec-spine` project.

The job requires the repository variable `PYPI_TRUSTED_PUBLISHING=true` and uses OIDC Trusted Publishing.

## 5. Determinism Gate

The `.github/workflows/determinism.yml` workflow proves that the emitted `registry.json` and `index.json` are byte-identical across four triples. Keep this gate green; a span drift on any platform fails it.
