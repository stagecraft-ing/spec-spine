//! The embedded JSON Schemas.
//!
//! Schemas live inside this crate (a deliberate divergence from OAP's
//! runtime-loaded `standards/schemas/`): embedding makes the published crate
//! self-contained and ties each schema to its compile-time version constant.
//! The conformance tests (in `spec-spine-core`) assert that emitted JSON
//! validates against these strings.

/// JSON Schema for the aggregate registry, i.e. the in-memory `Registry` shape
/// (matches [`crate::REGISTRY_SCHEMA_VERSION`]). Since spec 024 the committed
/// form is sharded; this validates the assembled view and the compiler's
/// in-memory output.
pub const REGISTRY_SCHEMA: &str = include_str!("../schemas/registry.schema.json");

/// JSON Schema for one committed registry shard, `by-spec/<id>.json` (spec 024).
pub const REGISTRY_SPEC_SHARD_SCHEMA: &str =
    include_str!("../schemas/registry-spec-shard.schema.json");

/// JSON Schema for `build-meta.json` (matches [`crate::BUILD_META_SCHEMA_VERSION`]).
pub const BUILD_META_SCHEMA: &str = include_str!("../schemas/build-meta.schema.json");

/// JSON Schema for the aggregate codebase index, i.e. the in-memory
/// `CodebaseIndex` shape (matches [`crate::INDEX_SCHEMA_VERSION`]). Since spec
/// 024 the committed form is sharded; this validates the assembled view and the
/// indexer's in-memory output.
pub const INDEX_SCHEMA: &str = include_str!("../schemas/codebase-index.schema.json");

/// JSON Schema for one committed index traceability shard,
/// `by-spec/<id>.json` (spec 024).
pub const INDEX_SPEC_SHARD_SCHEMA: &str =
    include_str!("../schemas/codebase-index-spec-shard.schema.json");

/// JSON Schema for one committed index inventory shard,
/// `by-package/<slug>.json` (spec 024).
pub const INDEX_PACKAGE_SHARD_SCHEMA: &str =
    include_str!("../schemas/codebase-index-package-shard.schema.json");
