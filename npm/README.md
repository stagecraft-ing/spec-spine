# spec-spine (npm)

**The [`spec-spine`](https://github.com/bartekus/spec-spine) CLI, distributed as a
prebuilt binary through npm. No Rust toolchain required.**

spec-spine turns a markdown spec corpus into a deterministic, hash-verifiable
authority ledger and refuses code that drifts from its owning spec at PR time.
This package lets a TypeScript/JS repo install and run the CLI like any other dev
tool.

```sh
npm i -D spec-spine
npx spec-spine --version
```

```sh
npx spec-spine init                                  # scaffold a spec corpus
npx spec-spine compile                               # specs/*/spec.md -> registry.json
npx spec-spine index                                 # codebase index
npx spec-spine lint --fail-on-warn                   # corpus conformance
npx spec-spine couple --base origin/main --head HEAD # the PR-time drift gate
```

Add it to your CI / package scripts like any other binary CLI:

```json
{
  "scripts": {
    "spec:check": "spec-spine compile && spec-spine index check && spec-spine lint --fail-on-warn"
  }
}
```

## How it works

This is a **binary-distribution shim, not a native (napi) addon**. The package's
`bin` is a tiny Node launcher that resolves the one prebuilt binary matching your
platform and runs it as a child process, forwarding arguments and the exit code.

The binaries ship as **optional dependencies** (`@spec-spine/cli-<os>-<cpu>`),
each gated by npm's `os`/`cpu` fields so only the one for your machine is
installed. There is **no `postinstall` script, no network access at install time,
and no archive extraction**, so it works under `npm ci --ignore-scripts` and
fully offline (from a warm cache or a private mirror).

## Supported platforms

| OS | Arch | Notes |
|---|---|---|
| macOS | arm64, x64 | |
| Linux | x64, arm64 | **glibc only** (see below) |
| Windows | x64 | |

**Linux binaries are glibc, not musl.** On **Alpine** or any musl host the
launcher refuses with a clear message: use a glibc-based image, or install from
source with `cargo install spec-spine-cli`. The same applies to any platform
without a prebuilt binary (Windows on ARM, 32-bit, …).

## Versioning

The npm version equals the binary release tag it ships: `spec-spine@0.1.0` runs
the binary built from `v0.1.0`. There is no floating range, so a pinned npm
version always runs a known binary.

## Alternatives (no npm)

```sh
cargo install spec-spine-cli                                          # from crates.io
curl -fsSL https://raw.githubusercontent.com/bartekus/spec-spine/main/install.sh | sh
```

## License

Apache-2.0. See the [repository](https://github.com/bartekus/spec-spine) for full
documentation, the Rust library API, and the design docs.
