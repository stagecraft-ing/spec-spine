---
id: installation
title: Installation
sidebar_position: 1
---

# Installation

spec-spine is distributed as a single multi-call binary. It is available via four distribution channels: crates.io (for Rust users), a prebuilt binary installer script, an npm package, and a PyPI package. All channels ship version **0.8.0**.

## crates.io (Rust)

If you have a Rust toolchain installed, `cargo install` is the recommended path. It builds the binary from source and places it on your path.

```bash
cargo install spec-spine-cli --version 0.8.0 --locked
```

## Prebuilt binary (Shell script)

If you do not have a Rust toolchain, you can install the prebuilt binary using the shell installer. The script detects your platform and architecture, downloads the matching release archive, verifies its checksum, and installs the binary.

```bash
curl -fsSL https://raw.githubusercontent.com/stagecraft-ing/spec-spine/main/install.sh | sh
```

:::note
The install script currently points to `stagecraft-ing/spec-spine`. Verify the URL against the canonical repository.
:::

You can pin a specific version and target directory using environment variables:

```bash
SPEC_SPINE_VERSION=v0.8.0 SPEC_SPINE_BIN_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/stagecraft-ing/spec-spine/main/install.sh | sh
```

## npm (TypeScript/JavaScript)

For TypeScript or JavaScript repositories, you can install spec-spine as a development dependency. The npm package ships a prebuilt binary through a small launcher; it does not require a Rust toolchain.

```bash
npm i -D spec-spine
```

You can then run the binary via `npx`:

```bash
npx spec-spine --version
```

## PyPI (Python)

For Python repositories, spec-spine is available as a PyPI package. It ships as platform wheels containing the prebuilt binary, placing it directly on your path.

```bash
uvx spec-spine
# or
pip install spec-spine
```

## Verifying the installation

Regardless of the channel you chose, you should now have the `spec-spine` binary available.

```bash
spec-spine --version
```

If the command prints the version, you are ready to proceed to the [Quickstart](quickstart.md).
