[![CI](https://github.com/bartekus/spec-spine/actions/workflows/ci.yml/badge.svg)](https://github.com/bartekus/spec-spine/actions/workflows/ci.yml)
&nbsp;License: Apache-2.0

# spec-spine

**A typed, hash-verifiable authority ledger over a markdown spec corpus.**
Installable Rust library + CLI; API-first, binding-ready, deterministic.

spec-spine turns a markdown spec corpus into a governed, hash-verifiable
authority ledger and **refuses code that drifts from its owning spec** at PR
time. Each `specs/NNN-slug/spec.md` declares, in YAML frontmatter, typed edges to
other specs and the authority units it owns (**file / section / symbol**). Two
deterministic views are emitted and joined by a coupling gate:

- **`registry.json`**: the *spec-as-source* view (the compiler's output).
- **`index.json`**: the *code-as-source* view (the indexer's output), with a
  content-hash staleness mechanism.

Every artifact-producing function is a **pure function of `(config, file
contents)`**: same inputs, byte-identical output, on every platform.

---

## Install

```sh
cargo install spec-spine-cli                                           # from crates.io
# or, no Rust toolchain:
curl -fsSL https://raw.githubusercontent.com/bartekus/spec-spine/main/install.sh | sh
# or, in a TS/JS repo (prebuilt binary, no Rust toolchain):
npm i -D spec-spine
# or, in a Python repo (prebuilt wheel, no Rust toolchain):
uvx spec-spine                 # or: pip install spec-spine
# or, from this checkout:
cargo install --path crates/spec-spine-cli
```

Each yields a `spec-spine` binary (on your `PATH`, or via `npx spec-spine` for the
npm install). The npm and PyPI packages ship the prebuilt binary per platform;
their Linux binaries are glibc (Alpine/musl use `cargo install`). See
[npm/](npm/) and [py/](py/).

## Quickstart

```sh
spec-spine init             # scaffold spec-spine.toml, standards/, specs/000, agent rules
spec-spine compile          # specs/*/spec.md -> .derived/spec-registry/registry.json
spec-spine index            # scan manifests + specs -> .derived/codebase-index/index.json
spec-spine lint             # corpus conformance
spec-spine couple --base origin/main --head HEAD   # the PR-time drift gate
```

See **[docs/adoption-guide.md](docs/adoption-guide.md)** for the full
install → init → annotate → wire-CI walkthrough.

## The five capabilities + init

| Command | Capability |
|---|---|
| `spec-spine compile` | validate frontmatter, emit the deterministic registry |
| `spec-spine index` / `index check` | emit the codebase index / check it for staleness |
| `spec-spine registry list\|show\|status-report\|relationships` | typed read-only queries |
| `spec-spine lint [--fail-on-warn] [--fail-on-info]` | corpus well-formedness |
| `spec-spine couple` | the PR-time coupling gate (refuses drift) |
| `spec-spine init [--force]` | scaffold a new adopter |

Exit codes: `0` ok · `1` validation failure / not found / drift · `2` stale ·
`3` I/O / parse / schema / config.

## Crates

| Crate | Role |
|---|---|
| [`spec-spine-types`](crates/spec-spine-types) | DTOs, frontmatter grammar, `Config`, schema-version constants, embedded JSON Schemas, the `Error` enum |
| [`spec-spine-core`](crates/spec-spine-core) | the engine: compile / index / query / lint / couple + the JSON facade |
| [`spec-spine-cli`](crates/spec-spine-cli) | the thin `spec-spine` multi-call binary |

**The library API, not the CLI, is the stable surface bindings wrap.** Every
operation has a JSON-in/JSON-out facade (`compile_json`, `query_json`, …); see
[docs/api.md](docs/api.md).

## Documentation

| Doc | What |
|---|---|
| [concept.md](docs/concept.md) | the origin story and the model: what spec-spine is and why it exists |
| [design/00-architecture.md](docs/design/00-architecture.md) | the full design: crate layout, `Config`, public API, exit codes, schema plan |
| [adoption-guide.md](docs/adoption-guide.md) | install → init → annotate → wire CI; the full `Config` knob table |
| [api.md](docs/api.md) | the `spec-spine-core` public API + JSON facade |
| [overlay-contract.md](docs/overlay-contract.md) | layer domain output on top without forking the core |
| [bindings-plan.md](docs/bindings-plan.md) | the napi / pyo3 / cgo path (design only; no binding code yet) |
| [schema-versioning.md](docs/schema-versioning.md) | MINOR/MAJOR rules; how loaders react; how adopters pin |
| [releasing.md](docs/releasing.md) | maintainer runbook: crates.io publish order, tag-gated binaries, determinism gate |

## Determinism & self-governance

- **Deterministic by construction.** Sorted-key, pretty-printed JSON with LF and
  a trailing newline; content hashes over LF/BOM-normalized, path-sorted bytes;
  tree-sitter grammars pinned exact. CI proves **byte-identical `registry.json` +
  `index.json` across all five release triples**, not just locally.
- **spec-spine governs itself.** This repo's own coupling gate runs against its
  own spec corpus in CI: a spec-spine that is not itself spec-governed would be
  hypocritical.

## License

Apache-2.0, chosen for its explicit patent grant, which matters once FFI
bindings and corporate adopters arrive. See [LICENSE](LICENSE).
