---
id: "008-python-distribution"
title: "Distribution: uvx/PyPI wheel shim"
status: approved
kind: "tooling"
created: "2026-06-11"
implementation: complete
owner: "The spec-spine Authors"
depends_on:
  - "007-distribution"
establishes:
  - "py/"
  - "py/scripts/generate_wheels.py"
extends:
  # 007 establishes release.yml; 008 adds a `publish-pypi` job to it. This is the
  # same additive file-level relationship 002 has to 001's lib.rs: 007 remains the
  # owner, 008 is an additive extender. Authority is scoped to the publish-pypi
  # job -- 008 does not touch publish/publish-crates/publish-npm.
  - spec: "007-distribution"
    unit: { kind: file, path: ".github/workflows/release.yml" }
    nature: additive
summary: >
  The Python parallel of 007's npm shim: a uvx/PyPI channel so a Python or
  polyglot team can `uvx spec-spine ...` (or `uv tool install spec-spine`) and
  get the CLI with no Rust toolchain. Where npm uses a main launcher package +
  five os/cpu-gated platform packages, Python collapses the same idea into one
  PyPI project + five per-platform wheels (+ one sdist): the wheel platform tag
  IS the selector, and the prebuilt binary ships in the wheel's data/scripts dir
  so pip/uv place it directly on PATH. There is no Python in the run path, no
  postinstall, no network at install, and no archive extraction at install: it
  works offline and under --no-build-isolation. Unsupported hosts (musl/Alpine,
  win-arm64, 32-bit) match no wheel and fall to the sdist, whose only artifact is
  a `spec-spine` console entry point that names the host and points at
  `cargo install spec-spine-cli` -- exact parity with 007 §3.4. Wheels are
  assembled from the same release archives the build job already produces (no
  second Rust build), are byte-reproducible, and are version-locked to the tag.
---

# 008: Distribution — uvx/PyPI wheel shim

## 1. Purpose

007 gave the `npm i -D spec-spine` audience a first-class, Rust-free path. The
Python and polyglot audience has the same reflex spelled `uvx` / `pipx` /
`uv tool install`, and the same refusal to install a Rust toolchain to lint a
spec corpus. This spec gives that audience the same first-class path, reusing
007's machinery rather than duplicating it: the release archives, the platform
map, the version-lock discipline, and the "absent setup = clean no-op" publish
posture all carry over. It adds one channel (`py/` + a `publish-pypi` job); it
does not change how the binary is built or how 007's channels behave.

## 2. Territory

This spec establishes `py/` (the Python distribution channel) and extends
007 §3.6 additively with a `publish-pypi` job in `release.yml`. The `py/`
subtree deliberately mirrors `npm/`:

    py/
      pyproject.toml              # the sdist project (the refusal fallback)
      src/spec_spine/
        __init__.py               # version via importlib.metadata
        platform_map.py           # the five triples <-> wheel tags (mirrors npm/lib/platform.js)
        _refuse.py                # sdist console entry point (mirrors the npm unsupported-host message)
      scripts/
        generate_wheels.py        # wheel assembler (mirrors scripts/generate-platform-packages.js)
        smoke_test.sh             # install-and-run check (mirrors npm/smoke-test.sh)
      test/test_platform_map.py   # asserts the triples equal release.yml's matrix
      README.md
      LICENSE                     # copied from repo root at build time

## 3. Behavior

### 3.1 The pattern: per-platform wheels, not download-on-first-use

npm selects the right binary with `optionalDependencies` plus `os`/`cpu` fields.
Python's equivalent selector is the **wheel platform tag**: pip/uv resolve the
one wheel whose tag matches the host and ignore the rest. So the npm "main
package + five platform packages" shape collapses into **one PyPI project + five
wheels (+ one sdist)**. Each wheel is `py3-none-<platform>`, `Root-Is-Purelib:
false`, and carries exactly one prebuilt binary in its
`spec_spine-<v>.data/scripts/` directory; on install pip/uv drop that file into
the environment's scripts dir as the `spec-spine` executable. `uvx spec-spine`
then runs the native binary directly.

This is a faithful translation of 007 §3.1's non-goals, not the download-shim
pattern: **no network at install, no archive extraction at install, no
postinstall, no Python interpreter on the run path**. It works offline, under
`uv tool install --offline`, and the run path is the binary itself (faster and
more robust than a Python launcher that fetches from GitHub on first use, which
would reintroduce every failure mode 007 §3.1 exists to avoid).

### 3.2 Platform map (the same five triples)

The wheel selector is the same five triples 007 §3.2 governs, each paired with
its wheel platform tag:

| target        | rust triple                    | wheel platform tag        |
| ------------- | ------------------------------ | ------------------------- |
| darwin-arm64  | aarch64-apple-darwin           | macosx_11_0_arm64         |
| darwin-x64    | x86_64-apple-darwin            | macosx_10_12_x86_64       |
| linux-x64     | x86_64-unknown-linux-gnu       | manylinux_2_17_x86_64     |
| linux-arm64   | aarch64-unknown-linux-gnu      | manylinux_2_17_aarch64    |
| win32-x64     | x86_64-pc-windows-msvc         | win_amd64                 |

This is now **the one fact in four places**: npm/lib/platform.js, install.sh,
release.yml's build matrix, and py/src/spec_spine/platform_map.py. The fourth
copy is kept honest by `py/test/test_platform_map.py`, which asserts the table
equals release.yml's matrix; a triple added to the release without updating the
Python map fails that test. Linux wheels target **glibc** (manylinux_2_17 ==
glibc 2.17); musl is out of scope by the same decision as 007 (see §3.4).

### 3.3 The launcher (there isn't one)

npm needs a launcher because npm cannot itself put a native binary on PATH; the
JS `bin` resolves the platform package and exec's the binary. A Python wheel can
put a binary on PATH directly (the `*.data/scripts/` convention), so the launcher
disappears: the binary IS the installed `spec-spine` command. The launcher
contract from 007 §3.3 (forward argv, forward exit code, surface signals, nothing
on the success path) is satisfied trivially because nothing intermediates — the
process the user invokes is the binary.

### 3.4 Unsupported hosts fail clearly

A host with no matching wheel (musl/Alpine, win-arm64, 32-bit, anything off the
five) gets no platform wheel, so pip/uv fall back to the **sdist**. The sdist
carries no binary and builds no Rust; its only artifact is a `spec-spine` console
entry point (`spec_spine._refuse:main`) that names the host, explains there is no
prebuilt binary for it, and points at the source build:

    cargo install spec-spine-cli

This is the exact posture of 007 §3.4 / npm's unsupported-host message. The
sdist is reached two ways — no matching wheel, or an explicit `--no-binary` — and
the message covers both (musl gets an extra Alpine hint; an explicit `--no-binary`
on a supported host gets a "reinstall allowing wheels" hint). It exits non-zero.

### 3.5 Version lock

The wheels and sdist are version-locked to the binary release tag: the v0.1.0
tag ships 0.1.0 artifacts. `generate_wheels.py` takes `--version` from the tag
(`v0.1.0` -> `0.1.0`) and, in CI, runs against the committed `py/` so a tag that
disagrees with the project's declared version is caught rather than published.
There is no second source of truth for the version: `__init__.py` reads it from
installed metadata, and the sdist's pyproject is the only declared copy.

### 3.6 Release integration

`release.yml` gains a `publish-pypi` job parallel to `publish-npm` (007 §3.6):

- It **reuses the build job's archives** (`download-artifact` of `archive-*`),
  exactly like publish-npm — there is no second Rust build for Python.
- It runs `generate_wheels.py --archives <dist> --build-sdist` to assemble the
  five wheels and the sdist, then uploads them.
- It is **idempotent**: re-running a tag skips artifacts already on PyPI
  (`--skip-existing`), the same property publish-npm gets from its `npm view`
  precheck.
- It is a **clean no-op until a human does the one-time setup**, the same posture
  as publish-npm's NPM_TOKEN gate. The recommended gate is a repository variable
  (`vars.PYPI_TRUSTED_PUBLISHING == 'true'`) flipped on once the PyPI project and
  its Trusted Publisher are created; absent that, the job is a skip, never a red
  X. The first publish, the project creation, and the Trusted Publisher config
  are a human's job (docs/releasing.md), mirroring the npm org-creation note.
- Publishing uses **PyPI Trusted Publishing (OIDC)** with **PEP 740
  attestations**, so no long-lived token lives in the repo and each artifact is
  signed by the workflow that built it. A `PYPI_API_TOKEN`-gated `twine upload
  --skip-existing` is the documented fallback for environments without OIDC.

## 4. Out of scope

- **musl / Alpine wheels.** glibc-only, same decision as 007; musl hosts get the
  sdist refusal with an Alpine hint. (If musl is ever added, it is one new triple
  in all four map copies + one release-matrix entry, and this spec amends.)
- **win-arm64, 32-bit.** Off the five triples; sdist refusal.
- **A pure-Python reimplementation or a download-on-first-use shim.** Explicitly
  rejected: it reintroduces the install/run-time network dependency and the loss
  of offline + `--ignore-scripts`-equivalent behavior that 007 §3.1 forbids.
- **maturin-built wheels.** A valid alternative (maturin can emit `bin` wheels),
  but it adds a second cargo build per target and diverges from the repo's
  build -> archives -> packagers shape; the archive-reuse generator is preferred
  for that reason. Recorded here so the choice is not relitigated silently.
- **An enterprise/private index.** The channel targets public PyPI; a private
  index is a deployment concern, not a property of this shim.
