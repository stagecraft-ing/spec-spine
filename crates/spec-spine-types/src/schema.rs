//! The embedded JSON Schemas.
//!
//! Schemas live inside this crate (a deliberate divergence from OAP's
//! runtime-loaded `standards/schemas/`): embedding makes the published crate
//! self-contained and ties each schema to its compile-time version constant.
//! The conformance tests (in `spec-spine-core`) assert that emitted JSON
//! validates against these strings.

/// JSON Schema for `registry.json` (matches [`crate::REGISTRY_SCHEMA_VERSION`]).
pub const REGISTRY_SCHEMA: &str = include_str!("../schemas/registry.schema.json");

/// JSON Schema for `build-meta.json` (matches [`crate::BUILD_META_SCHEMA_VERSION`]).
pub const BUILD_META_SCHEMA: &str = include_str!("../schemas/build-meta.schema.json");

/// JSON Schema for `index.json` (matches [`crate::INDEX_SCHEMA_VERSION`]).
pub const INDEX_SCHEMA: &str = include_str!("../schemas/codebase-index.schema.json");
