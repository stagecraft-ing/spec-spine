//! Schema-version constants: library-owned, started fresh at `0.1.0`.
//!
//! These are deliberately decoupled from the reference repos' version lines
//! (OAP registry `2.2.0`, index `3.0.0`). They are compile-time constants; in
//! Phase 2/3 the conformance tests assert that emitted JSON validates against the
//! embedded schema of the matching version, so a mismatch fails the build rather
//! than at runtime.
//!
//! Versioning policy (see `docs/design/00-architecture.md` §7 and, in Phase 5,
//! `docs/schema-versioning.md`): MINOR = additive only; MAJOR = breaking, and
//! loaders reject an unknown MAJOR. Under `0.x`, MINOR may break (SemVer `0.x`).

/// `specVersion` emitted in the registry, carried by each registry shard.
/// `0.2.0`: declared extra-frontmatter values widen to arbitrary JSON (spec 013).
/// `0.3.0`: structured/partial `supersedes` items (spec 019); full supersession
/// stays a bare string, so a full-only corpus is byte-identical.
/// `1.0.0`: **MAJOR** (spec 024). The committed registry is sharded per-spec
/// under `by-spec/<id>.json`; the single `registry.json` is no longer emitted.
/// The aggregate view (validation, content hash) is recomputed on read. Loaders
/// reject an unknown MAJOR, so a 0.x reader cannot misread a 1.x shard tree.
pub const REGISTRY_SCHEMA_VERSION: &str = "1.0.0";

/// `schemaVersion` emitted in the codebase index, carried by each index shard.
/// `0.2.0`: additive `build.sliceHashes` (spec 012).
/// `0.3.0`: additive `directory`/`crate`/`module` resolved-unit kinds (spec 017).
/// `1.0.0`: **MAJOR** (spec 024). The committed index is sharded per-spec under
/// `by-spec/<id>.json` and per-package under `by-package/<slug>.json`; the single
/// `index.json` is no longer emitted. The aggregate view (orphans, untraced code,
/// content hash) is recomputed on read; staleness is per-shard.
/// `1.1.0`: additive (spec 025). The resolver downgrades an unresolved unit to a
/// non-blocking `W-001` (draft/pending owning) or `W-002` (non-owning reference)
/// warning instead of a hard error; the `warnings` tier and free-form diagnostic
/// `code` already exist, so no schema-file edit is needed.
pub const INDEX_SCHEMA_VERSION: &str = "1.1.0";

/// `schemaVersion` emitted in `build-meta.json` (the non-deterministic artifact).
pub const BUILD_META_SCHEMA_VERSION: &str = "0.1.0";

/// The `spec-spine.toml` config schema version (optional `config_version` key).
pub const CONFIG_VERSION: &str = "0.1.0";

/// Parse a `MAJOR.MINOR.PATCH` string into its numeric components.
///
/// Returns `None` if the string is not three dot-separated non-negative integers.
/// Used by loaders to reject an unknown MAJOR.
pub fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}
