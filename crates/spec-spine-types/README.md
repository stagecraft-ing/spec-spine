# spec-spine-types

Typed data substrate for [spec-spine](https://github.com/spec-spine/spec-spine):
DTOs, the spec frontmatter grammar, the `Config` model, the authority-unit and
typed-edge grammar, schema-version constants, and the stable `Error` enum.

This crate is plain data — owned, `serde`-serializable types with no lifetimes,
generics, or trait objects at the public boundary — so it can back both the
`spec-spine-core` engine and future FFI bindings (napi / pyo3 / cgo).

See `docs/design/00-architecture.md` in the workspace for the full design.

License: Apache-2.0.
