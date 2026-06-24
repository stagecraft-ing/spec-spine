---
id: schema-and-versioning
title: Schema and Versioning
sidebar_position: 8
---

# Schema and Versioning

The registry and index schemas are **library-owned** and decoupled from any consumer's history. They start fresh at `0.1.0`.

## The Versioned Artifacts

| Artifact | Field | Current Value |
|---|---|---|
| Registry shards (`spec-registry/by-spec/<id>.json`) | `specVersion` | `1.0.0` |
| Index shards (`codebase-index/...`) | `schemaVersion` | `1.0.0` |
| `build-meta.json` | `schemaVersion` | `0.1.0` |
| `spec-spine.toml` | `config_version` | `0.1.0` |

Each shard carries its artifact's schema version. A reader gates the MAJOR version per shard.

## MINOR vs MAJOR Policy

The version line is SemVer over the *schema*, independent of the toolchain's own crate version.

- **MINOR bump (Additive):** Old readers keep working. This includes new optional fields, new enum variants, or new unit kinds.
- **MAJOR bump (Breaking):** Old readers must refuse. This includes removed/renamed fields, retyped fields, or changed semantics.

*(Note: Pre-1.0 (`0.x`), a MINOR bump is permitted to make a breaking change. Pin the toolchain version to avoid issues.)*

## How Loaders React

The `load_registry` and `load_index` functions enforce the policy at read time. They **reject an unknown MAJOR**:

1. If the version is not SemVer, return an error.
2. If the MAJOR version differs from the build's MAJOR, return an error.
3. Otherwise, load the artifact.

A build understands **its own MAJOR line only**. Within a MAJOR, MINOR differences are ignored by older readers.

## `deny_unknown_fields`

Both the `Config` (`spec-spine.toml`) and the artifact DTOs use `deny_unknown_fields`.

- **Upside:** A misspelled config knob or a field from a newer schema is a loud error, never a silently ignored setting.
- **Consequence:** An older pinned binary will error on a newer config or artifact. This tells you to upgrade the binary rather than letting a newer config quietly do nothing.

## How Adopters Pin

To ensure reproducible governance, pin your binary version:

- **crates.io:** `cargo install spec-spine-cli --version =X.Y.Z --locked`
- **Prebuilt binary:** `SPEC_SPINE_VERSION=vX.Y.Z install.sh`

Keep the binary version and the committed artifacts in lockstep.
