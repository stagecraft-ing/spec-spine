# Bindings plan (napi / pyo3 / cgo)

> **Design only: no binding code exists in this repo, by mandate.** This
> document describes how npm, Python, and Go bindings will wrap `spec-spine-core`
> later. The library is built to make this a thin, mechanical exercise: every
> operation already has a `&str → Result<String, Error>` facade
> (see [api.md](api.md) §7), which is the single seam every binding wraps.

## Why the JSON facade is the seam (not the typed API)

The typed Rust API is ergonomic for Rust callers but crosses an FFI boundary
poorly: owned structs, enums, and `Result` do not map cleanly to a C ABI or a
JS/Python object without per-type marshalling. The JSON facade collapses the
entire surface to one signature:

```rust
fn op(input_json: &str) -> Result<String, Error>
```

A binding only needs to: (1) pass a UTF-8 string in, (2) get a UTF-8 string (or
an error) out, (3) wrap that in the host language's idiom. No per-DTO
marshalling, no lifetime/generic/trait-object handling; the boundary is already
FFI-friendly *by construction* (no lifetimes, generics, or trait objects;
`unsafe_code = "forbid"`; a single `Error` enum).

The facade functions to wrap:

| Facade fn | Wraps |
|---|---|
| `compile_json(config_json, repo_root)` | `compile` |
| `index_json(config_json, repo_root)` | `index` |
| `lint_json(config_json, repo_root)` | `lint` |
| `check_freshness_json(config_json, repo_root)` | `check_index_freshness` |
| `couple_json(request_json)` | `couple` |
| `query_json(request_json)` | `list` / `show` / `status_report` / `relationships` |
| `load_config_json(toml_src)` | `load_config` |
| `scaffold_init_json(config_json)` | `scaffold_init` |

## The uniform envelope

In Rust the facade returns `Result<String, Error>`. The binding layer maps that
to one envelope shape across all languages, so callers handle success and failure
uniformly:

```jsonc
// success
{ "ok": true,  "data": <the facade's returned JSON, parsed>, "error": null }
// failure
{ "ok": false, "data": null, "error": { "code": "Validation" | "NotFound" | "Stale"
                                              | "Config" | "Io" | "Parse" | "Schema",
                                         "message": "…", "exitCode": 1 | 2 | 3 } }
```

`error.code` is the `Error` variant name; `error.exitCode` is
`Error::exit_code()`. Both are stable (the `Error` enum is `#[non_exhaustive]`,
so new variants are additive; a binding should treat an unknown `code` as a
generic failure). This is the only mapping logic a binding must implement; it is
identical across napi/pyo3/cgo.

---

## napi-rs (Node / npm): sketch (do not build yet)

A `spec-spine-napi` crate using [`napi-rs`](https://napi.rs):

```rust
// crates/spec-spine-napi/src/lib.rs  (illustrative only, not in this repo)
#[napi]
pub fn compile(config_json: String, repo_root: String) -> napi::Result<String> {
    spec_spine_core::compile_json(&config_json, &repo_root).map_err(to_napi_err)
}
// … one #[napi] fn per facade fn; to_napi_err builds the {code,message,exitCode} envelope.
```

- Ships as a prebuilt `.node` per platform via napi-rs's GitHub Actions matrix:
  the same triple matrix as the binary release.
- The published npm package wraps each export to parse the returned JSON and
  expose idiomatic JS (`await specSpine.compile(config, repoRoot)` returning the
  parsed registry, throwing a typed `SpecSpineError` carrying `code`/`exitCode`).

## pyo3 (Python): sketch

A `spec-spine-py` crate using [`pyo3`](https://pyo3.rs) +
[`maturin`](https://maturin.rs):

```rust
// illustrative only
#[pyfunction]
fn compile(config_json: &str, repo_root: &str) -> PyResult<String> {
    spec_spine_core::compile_json(config_json, repo_root).map_err(to_py_err)
}
#[pymodule]
fn spec_spine(m: &Bound<PyModule>) -> PyResult<()> { m.add_function(wrap_pyfunction!(compile, m)?)? /* … */ }
```

- `maturin` builds wheels per platform (the same triple matrix); publishes to
  PyPI.
- A thin Python layer parses the JSON and raises `SpecSpineError(code, exit_code,
  message)` on `ok == false`, returning `dict`/dataclasses on success.

## cgo (Go): sketch

A `cdylib`/`staticlib` crate exposing a C ABI, consumed from Go via cgo:

```rust
// illustrative only: the one place `unsafe`/`extern "C"` is permitted (a binding crate, not core)
#[no_mangle]
pub extern "C" fn spec_spine_compile(config_json: *const c_char, repo_root: *const c_char) -> *mut c_char { … }
#[no_mangle]
pub extern "C" fn spec_spine_string_free(p: *mut c_char) { … }   // caller frees returned strings
```

- The C header is generated (`cbindgen`); the Go package wraps each `extern "C"`
  fn, marshals strings across cgo, and frees them via `spec_spine_string_free`.
- Go callers get `func Compile(configJSON, repoRoot string) (Registry, error)`,
  decoding the envelope and returning a typed error on `ok == false`.

---

## Design rules these bindings rely on (already guaranteed by the core)

- **Pure functions of `(config, file bytes)`**, no ambient clock/env, so a
  binding can call from any host without surprise side effects (the sole
  wall-clock value, `build-meta.json.builtAt`, is written by the CLI, not the
  facade).
- **Owned, `serde`-serializable DTOs**: everything crossing the boundary is
  already JSON-representable.
- **A single, stable `Error` enum → stable exit codes**: the envelope's
  `code`/`exitCode` are a direct, stable projection.
- **No `unsafe` in core**: the only `extern "C"`/`unsafe` lives in the (future)
  cgo binding crate, never in `spec-spine-core`.
- **`publish = false` is set on none of the shipped crates**: bindings can
  depend on the published `spec-spine-core` from crates.io.

## Repository shape when bindings land (future)

```
crates/
├─ spec-spine-types/   (published)
├─ spec-spine-core/    (published; bindings depend on this)
├─ spec-spine-cli/     (published)
├─ spec-spine-napi/    (future; napi-rs → npm)
├─ spec-spine-py/      (future; pyo3/maturin → PyPI)
└─ spec-spine-ffi/     (future; cdylib + cbindgen → Go/C/others)
```

Each binding crate is a thin shell over the facade; the engine and its
guarantees stay in `spec-spine-core`. **Nothing in this list is built yet.**
