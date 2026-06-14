# Releasing spec-spine (maintainers)

> Four distribution paths ship together: **crates.io** (the library + CLI, and
> the path that unblocks bindings), **prebuilt binaries** (the `curl | sh` install
> path), **npm** (the same prebuilt binaries, repackaged so a TS/JS repo can
> `npm i -D spec-spine`; spec 007), and **PyPI** (the same binaries again, as
> platform wheels so a Python team can `uvx spec-spine`; spec 008). This is the
> maintainer runbook. Adopters do not read this; they read
> [adoption-guide.md](adoption-guide.md).

## Pre-flight

- [ ] Working tree clean and green: `cargo test --workspace --locked`.
- [ ] Self-governance green: `spec-spine compile && spec-spine index check &&
      spec-spine lint --fail-on-warn && spec-spine couple --base origin/main --head HEAD`.
- [ ] If the spec-020 derived-artifact merge driver is enabled on this clone
      (`./.githooks/enable-merge-driver.sh`), the binary is current
      (`cargo build --release --locked`) before merging the release-prep branch:
      the driver regenerates the committed artifacts by shelling out to
      `spec-spine compile && index`.
- [ ] Versions bumped consistently with `scripts/bump_version.py <version>`,
      then `scripts/bump_version.py --check <version>` is green. The script sets
      all three shims in lockstep: workspace `version` in the root `Cargo.toml`;
      `version` + the `optionalDependencies` pins in `npm/package.json` (the
      release workflow re-locks these to the tag via `--write-main`, but they
      should match in source); `version` in `py/pyproject.toml` (the workflow
      verifies this against the tag and fails loudly on a mismatch, the asymmetry
      that let v0.2.0's PyPI publish fail while npm/crates shipped). Schema-version
      constants in `spec-spine-types` are decoupled; bump them separately per
      [schema-versioning.md](schema-versioning.md) only if the schema changed.
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
git tag vX.Y.Z
git push origin vX.Y.Z
```

That builds a per-triple archive (with a `.sha256` sidecar) for all five
supported targets, `x86_64`/`aarch64` `apple-darwin`, `x86_64`/`aarch64`
`unknown-linux-gnu`, `x86_64-pc-windows-msvc`, each on a GitHub-hosted runner
(`x86_64-apple-darwin` cross-compiles on the Apple Silicon runner), and attaches
them to the GitHub Release.
[`install.sh`](../install.sh) (`curl | sh`) consumes those assets.

### Supply-chain artifacts (spec 021)

Each of the five archives ships with two supply-chain artifacts, generated in the
same `build` matrix (no second Rust build):

- a **per-target CycloneDX SBOM** (`spec-spine-<tag>-<triple>.cdx.json`) produced
  by `anchore/sbom-action` (syft) from the committed `Cargo.lock`, attached to
  the GitHub Release as a separate asset. A fail-closed check refuses to ship an
  SBOM with zero components.
- a **SLSA build-provenance attestation** over the archive, recorded in GitHub's
  attestation store via `actions/attest-build-provenance` (no long-lived token).

Verify a downloaded archive:

```sh
gh attestation verify spec-spine-<tag>-<triple>.tar.gz --repo bartekus/spec-spine
```

These steps run only on a `v*` tag (the release workflow is tag-gated), so they
are not exercised by PR CI; if the pinned action versions move, sanity-check them
before the next release.

## 3. npm: the binary-distribution shim (spec 007)

The same `v*` tag drives the `publish-npm` job. It does **not** rebuild Rust: it
downloads the build matrix's archives and repackages them as npm packages, then
publishes them. It is a binary shim (a launcher that exec's the prebuilt binary),
**not** a native addon.

The shape (esbuild / biome / turbo model):

- a main package **`spec-spine`** whose `bin` is a tiny launcher
  (`npm/bin/spec-spine.js`);
- five **platform packages** `@spec-spine/cli-<os>-<cpu>`, each carrying one
  prebuilt binary, `os`/`cpu`-gated and listed as `optionalDependencies` of the
  main package, **version-locked** to the tag (npm `X.Y.Z` ⇔ `vX.Y.Z` assets).

There is **no `postinstall`**, so it installs under `npm ci --ignore-scripts` and
offline. The `npm/scripts/generate-platform-packages.js` generator assembles the
platform packages from the archives at publish time; binaries and generated
packages are never committed.

The job is **idempotent** (`npm view` precheck skips versions already live) and
**gated on the `NPM_TOKEN` secret** (absent the token it is a clean no-op, the
same posture as the crates.io token). **First-time human setup (once):**

```sh
# 1. Create the @spec-spine org on npm (Settings → Add Organization), so the
#    scoped platform packages @spec-spine/cli-* can be published.
# 2. Create an automation access token (npmjs.com → Access Tokens → Granular/
#    Automation; bypasses 2FA for CI) with publish rights to `spec-spine` and the
#    @spec-spine scope.
# 3. Add it as the repo secret NPM_TOKEN (Settings → Secrets → Actions).
```

Once `NPM_TOKEN` is set, every `v*` tag publishes npm alongside crates.io and the
GitHub Release. Verify a release with:

```sh
npm view spec-spine@<version> version          # main package live
npm view @spec-spine/cli-darwin-arm64@<version> version
npx spec-spine@<version> --version             # end-to-end smoke
```

Local dry-run before tagging (no publish, no network): from `npm/`, run
`npm test` (the platform-mapping unit test) and `npm run smoke` (builds the host
binary, packs + installs both packages into a throwaway project, and runs
`spec-spine --version` through the launcher).

## 4. PyPI: the wheel shim (spec 008)

The same `v*` tag drives the `publish-pypi` job. Like npm, it does **not**
rebuild Rust: it downloads the build matrix's archives and repackages them, here
as **five platform wheels + one sdist** under a single `spec-spine` PyPI
project. The wheel platform tag is the selector (the Python analogue of npm's
`os`/`cpu` fields): pip/uv install only the wheel matching the host, and each
wheel carries its prebuilt binary in the `*.data/scripts/` directory so the
binary lands directly on `PATH` as the `spec-spine` command. There is no Python
in the run path, no postinstall, and no network at install. Unsupported hosts
(musl/Alpine, win-arm64, 32-bit) match no wheel and fall to the sdist, whose
`spec-spine` entry point refuses clearly and points at
`cargo install spec-spine-cli`.

The `py/scripts/generate_wheels.py` generator assembles the wheels from the
archives at publish time; binaries, wheels, and the sdist are never committed.

The job is **idempotent** (`skip-existing: true` makes re-running a tag skip
artifacts already on PyPI) and **gated on the repository variable
`PYPI_TRUSTED_PUBLISHING`** (a variable, not a secret: Trusted Publishing has no
token to detect). Absent or not `'true'`, the job is a clean no-op, the same
posture as `NPM_TOKEN`. **First-time human setup (once):**

```sh
# 1. On PyPI, register this repo as a Trusted Publisher for the `spec-spine`
#    project (for the not-yet-existing project: Account → Publishing → "Add a
#    new pending publisher"): owner bartekus, repo spec-spine, workflow
#    release.yml, environment pypi. The first publish then creates the project.
# 2. Create the matching `pypi` environment in this repo
#    (Settings → Environments → New environment).
# 3. Set the repository variable PYPI_TRUSTED_PUBLISHING=true
#    (Settings → Secrets and variables → Actions → Variables).
```

Publishing uses OIDC Trusted Publishing with PEP 740 attestations, so no
long-lived token lives in the repo. If OIDC is ever unavailable, the documented
fallback is a `PYPI_API_TOKEN` secret + `twine upload --skip-existing`; see the
comment block at the bottom of `release.yml`.

Once the variable is set, every `v*` tag publishes PyPI alongside npm, crates.io,
and the GitHub Release. Verify a release with:

```sh
pip index versions spec-spine            # versions live on PyPI
uvx spec-spine@<version> --version       # end-to-end smoke through a wheel
```

Local dry-run before tagging (no publish): from `py/`, run
`PYTHONPATH=src python3 -m unittest discover -s test` (the platform-mapping unit
test) and `./scripts/smoke_test.sh` (builds the host binary, generates the host
platform wheel, installs it into a throwaway env with uv or pip, and runs
`spec-spine --version` through the installed binary).

## 5. Determinism gate

`.github/workflows/determinism.yml` proves the emitted `registry.json` and
`index.json` (including tree-sitter symbol line-spans) are **byte-identical
across four triples** (`x86_64-apple-darwin` is intentionally omitted: its two
dimensions, x86_64 and apple-darwin, are each proven by another leg, and the
deprecated Intel macOS runner queues badly): the empirical backstop for the
"identical-on-every-triple" claim, beyond merely pinning the grammars exact. The
release workflow still builds and ships all five archives. Keep this gate green;
a span drift on any platform fails it.

## Schema / version bumps

Follow [schema-versioning.md](schema-versioning.md): MINOR = additive only;
MAJOR = breaking (loaders reject an unknown MAJOR). Bump the schema-version
constants and the embedded schemas together so the conformance test (which fails
the **build** on mismatch) stays the source of truth.
