# spec-spine (Python / uvx)

The `spec-spine` CLI for Python users: compile a markdown spec corpus into a
deterministic authority ledger and refuse code that drifts from its owning spec.
Ships the prebuilt binary; **no Rust toolchain required**.

```sh
uvx spec-spine check          # run the CLI with no install
uv tool install spec-spine    # or install it as a persistent tool
pip install spec-spine        # or into a project/venv
```

## How it works

This is a **binary distribution**, not a Python binding. There is no native
extension and the engine is never called from Python. The project publishes:

- **five platform wheels** — one per supported target, each carrying the prebuilt
  `spec-spine` binary in the wheel's scripts directory. pip/uv select the one
  matching your host by its platform tag and install the binary onto `PATH`. On a
  supported host there is **no Python in the run path and no network at install**
  beyond fetching the wheel itself; it works offline from a warm cache or a
  private mirror, and under `--no-binary`-free resolution.
- **one sdist** — the unsupported-host fallback. It builds only when no wheel
  matches (musl/Alpine, win-arm64, 32-bit), and its `spec-spine` command prints a
  clear message pointing at `cargo install spec-spine-cli`.

This mirrors the npm shim (spec 007): npm uses `os`/`cpu`-gated
`optionalDependencies`; Python uses wheel platform tags. One project, many
wheels.

## Supported targets

| host | wheel platform tag | release triple |
|---|---|---|
| macOS arm64 | `macosx_11_0_arm64` | `aarch64-apple-darwin` |
| macOS x86_64 | `macosx_10_12_x86_64` | `x86_64-apple-darwin` |
| Linux x86_64 (glibc) | `manylinux_2_17_x86_64` | `x86_64-unknown-linux-gnu` |
| Linux arm64 (glibc) | `manylinux_2_17_aarch64` | `aarch64-unknown-linux-gnu` |
| Windows x86_64 | `win_amd64` | `x86_64-pc-windows-msvc` |

Linux binaries are **glibc**. Alpine/musl hosts have no wheel and must use
`cargo install spec-spine-cli` or a glibc-based image.

## License

Apache-2.0. See [LICENSE](./LICENSE).
