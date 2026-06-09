# spec-spine-core

The spec-spine engine. Compiles a markdown spec corpus into a deterministic,
hash-verifiable authority registry (`registry.json`) and provides typed,
read-only query over it.

Every artifact-producing function is a pure function of `(config, file
contents)`, with no ambient clock or environment reads, so the same inputs produce
byte-identical output. The public API returns owned, `serde`-serializable DTOs
(from `spec-spine-types`) and a JSON-in/JSON-out facade, the seam future FFI
bindings will wrap.

See `docs/design/00-architecture.md` for the design. License: Apache-2.0.
