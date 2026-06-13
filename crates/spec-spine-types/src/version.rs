//! Schema-version constants — library-owned, started fresh at `0.1.0`.
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

/// `specVersion` emitted in `registry.json`.
/// `0.2.0`: declared extra-frontmatter values widen to arbitrary JSON (spec 013).
/// `0.3.0`: structured/partial `supersedes` items (spec 019); full supersession
/// stays a bare string, so a full-only corpus is byte-identical.
pub const REGISTRY_SCHEMA_VERSION: &str = "0.3.0";

/// `schemaVersion` emitted in `index.json`. (Index DTOs land in Phase 3.)
/// `0.2.0`: additive `build.sliceHashes` (spec 012).
/// `0.3.0`: additive `directory`/`crate`/`module` resolved-unit kinds (spec 017).
pub const INDEX_SCHEMA_VERSION: &str = "0.3.0";

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
