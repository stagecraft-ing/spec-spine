---
id: "007-distribution"
title: "Distribution: npm binary shim + the release pipeline"
status: approved
kind: "tooling"
created: "2026-06-09"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "001-compile-registry"
establishes:
  - "npm/"
  - "install.sh"
  - ".github/workflows/release.yml"
  - "docs/releasing.md"
summary: >
  How the spec-spine CLI reaches users who do not have a Rust toolchain. Two
  channels already existed but were unspecced: the tag-gated GitHub release
  pipeline (release.yml) that builds the five per-triple archives, and install.sh
  (curl | sh) that consumes them. This spec adopts both as governed territory and
  adds a third channel: an npm binary-distribution shim under npm/ so a TS/JS repo
  can `npm i -D spec-spine` and get the CLI with no Rust. The shim is a launcher,
  not a native addon: the main package's bin resolves a per-triple platform
  package (optionalDependencies, os/cpu-gated) and exec's the prebuilt binary,
  forwarding argv and exit code. No postinstall, no network at install, no archive
  extraction; it works under `npm ci --ignore-scripts` and offline. Version-locked
  to the binary release tag (npm 0.1.0 ships the v0.1.0 assets).
---

# 007: Distribution

## 1. Purpose

`cargo install spec-spine-cli` and `curl | sh` (install.sh) already put the
`spec-spine` binary on a machine, but a TypeScript/JS team's reflex is
`npm i -D <tool>`, and they will not install a Rust toolchain to lint a spec
corpus. This spec gives that audience a first-class path and, while doing so,
brings the previously-unspecced release machinery (release.yml, install.sh, the
release runbook) under the same authority ledger that governs the rest of the
library.

The framing is deliberate and load-bearing: this is a **binary-distribution
shim, not a napi / N-API binding.** No Rust is compiled into a Node addon, and
the engine is never called from JS. The shim ships the existing prebuilt
`spec-spine` CLI through npm and exec's it as a child process. The stable surface
that bindings wrap remains the Rust library API (spec 000, docs/bindings-plan.md),
untouched by this work.

## 2. Territory

- **`npm/`** (directory subtree): the entire npm shim. The main package
  (`package.json`, the `bin/` launcher, the `lib/` resolution logic), the
  publish-time platform-package generator (`scripts/`), and the tests. Generated
  platform packages and the prebuilt binaries they carry are **not** committed
  (they are assembled at publish time from release artifacts), so they are not
  part of this territory.
- **`install.sh`**: the `curl | sh` installer that detects platform/arch,
  downloads the matching release archive and its `.sha256` sidecar, verifies the
  checksum, and drops the binary on `PATH`.
- **`.github/workflows/release.yml`**: the tag-gated pipeline that builds the
  five per-triple archives, publishes the GitHub Release and the crates, and (new
  in this spec) publishes the npm packages.
- **`docs/releasing.md`**: the maintainer release runbook.

The platform map (`npm/`), the install.sh detection table, and the release.yml
build matrix are **one fact in three places**; this spec makes that shared fact
its responsibility so a drift in any one of them is a governed change.

## 3. Behavior

### 3.1 The pattern: optional-dependencies platform packages

The shim follows the esbuild / `@biomejs/biome` / turbo model:

- A single main package, **`spec-spine`**, whose
  `bin: { "spec-spine": "bin/spec-spine.js" }` is a tiny Node launcher.
- One **platform package per supported triple**
  (`@spec-spine/cli-<os>-<cpu>`), each carrying exactly one prebuilt binary and
  declaring `os` / `cpu` so npm installs only the matching one and skips the rest.
- The platform packages are listed as **`optionalDependencies`** of the main
  package, version-locked (§3.5).

There is **no `postinstall` script**, no network access at install time, and no
archive extraction. Installation is npm placing the one matching platform package
into `node_modules`; the binary is already a plain file inside it. This is what
lets the shim work under `npm ci --ignore-scripts` and fully offline (from a
warm cache or a private mirror), the failure modes that sink download-on-install
shims.

### 3.2 Platform map (the five triples)

The map is derived from `release.yml`'s build matrix and **must match it
exactly**, including the in-archive binary name:

| `process.platform` | `process.arch` | platform package | release triple | in-archive binary |
|---|---|---|---|---|
| `darwin` | `arm64` | `@spec-spine/cli-darwin-arm64` | `aarch64-apple-darwin` | `spec-spine` |
| `darwin` | `x64` | `@spec-spine/cli-darwin-x64` | `x86_64-apple-darwin` | `spec-spine` |
| `linux` | `x64` | `@spec-spine/cli-linux-x64` | `x86_64-unknown-linux-gnu` | `spec-spine` |
| `linux` | `arm64` | `@spec-spine/cli-linux-arm64` | `aarch64-unknown-linux-gnu` | `spec-spine` |
| `win32` | `x64` | `@spec-spine/cli-win32-x64` | `x86_64-pc-windows-msvc` | `spec-spine.exe` |

The Linux binaries are **glibc** (the `-gnu` triples), not musl. Alpine / musl
hosts are unsupported by the shim and must use `cargo install spec-spine-cli` or
a glibc-based image (§3.4).

### 3.3 The launcher

`bin/spec-spine.js` is a pure translation layer with no logic of its own beyond
resolution and process forwarding:

1. Map `(process.platform, process.arch)` to a platform-package name and binary
   name via the table in §3.2 (the mapping lives in `lib/`, is a pure function of
   its inputs, and is unit-tested with no I/O).
2. Resolve the installed platform package's binary with `require.resolve`.
3. `execFileSync` the binary with `process.argv.slice(2)` and `stdio: "inherit"`,
   then exit with the child's exit code (and surface a terminating signal as a
   non-zero exit).

The launcher adds no flags, rewrites no arguments, and prints nothing on the
success path: `spec-spine <args>` through npm is behaviorally identical to the
native binary.

### 3.4 Unsupported hosts fail clearly

On any host without a matching prebuilt binary, the launcher exits non-zero with
a message that names the host and points at the source-build escape hatch. This
covers, at minimum:

- triples with no archive (`win32-arm64`, `linux` 32-bit, …): the mapping
  function itself refuses them;
- **musl / Alpine** Linux: detected (the glibc runtime is absent) and refused
  even though `linux-<arch>` is otherwise supported, because the glibc binary
  would not run;
- a supported triple whose optional platform package is **missing** (installed
  with `--no-optional`, or its install failed): `require.resolve` fails and the
  launcher reports how to recover.

Every such message recommends `cargo install spec-spine-cli` (and, for
Alpine/musl, a glibc-based image). The README states the glibc constraint up
front so Alpine CI reaches for `cargo install` or a `-gnu` image from the start.

### 3.5 Version lock

The npm package version **equals** the binary release tag it ships: npm `0.1.0`
carries the `v0.1.0` archives, and the main package pins each
`optionalDependencies` entry to that exact version. There is no floating range,
so a given `spec-spine` npm version always exec's the binary built from the
matching tag. The lock is enforced at publish time by the generator, which
stamps every platform package with the release version and fails on a mismatch.

### 3.6 Release integration

`release.yml`, on a `v*` tag, additionally publishes the npm packages:

- it is **idempotent** (a version already live on npm is skipped, matching the
  crates.io job), so re-running a tag is safe;
- it is **gated on an `NPM_TOKEN` secret**: absent the token the job is a no-op,
  never a failure. The token must be an npm **automation** token (it bypasses the
  2FA one-time-password that CI cannot supply);
- scoped packages default to **restricted** on npm, so each platform package
  carries `publishConfig.access: public` (and the job also passes
  `--access public`) to publish publicly;
- the platform packages are generated from the same per-triple archives the build
  matrix already produced (no second Rust build), then published, then the main
  package. The generator extracts each binary tolerant of archive member layout:
  the release tarballs store members `./`-prefixed, which a strict `tar` (GNU tar
  on the Linux runner) will not match by bare name, so it extracts the whole
  archive and reads the binary by its known path rather than selecting a member.

The first npm publish and the `NPM_TOKEN` (and the one-time creation of the
`@spec-spine` org) are left to a human, the same posture as the crates.io token.
The runbook step lives in `docs/releasing.md`.

## 4. Out of scope

- **No napi / native binding.** No Node addon, no `napi-rs` / `neon`, no calling
  the engine in-process from JS. That path is design-only in
  `docs/bindings-plan.md` and wraps the Rust library API, not this CLI shim.
- **No actual `npm publish` from the build session.** The pipeline is made
  publish-ready; the human performs the first publish.
- **Migrating any existing repo** (e.g. template-encore) onto the shim.
- **Windows-on-ARM, musl, and 32-bit** prebuilt binaries: out of the v1 triple
  set; those hosts use `cargo install` (§3.4). Adding a triple is an additive
  change to the §3.2 map and the release matrix together.
