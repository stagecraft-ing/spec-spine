//! Shared shard-storage primitives (spec 024).
//!
//! The two committed artifacts (the spec registry and the codebase index) are
//! stored as one file per authority unit (`by-spec/<id>.json`,
//! `by-package/<slug>.json`) instead of one monolithic file behind a global
//! content-hash line. Two PRs that touch different units then write disjoint
//! files and never conflict textually, so GitHub's server-side merge and the
//! merge queue's speculative build form clean stacks (the spec 020 merge driver
//! is needed only for the rare same-shard conflict).
//!
//! This module holds the storage mechanics common to both artifacts:
//! directory synchronization (write current shards, prune removed ones), the
//! global-inputs hash folded into every shard, the aggregate content hash
//! recomputed from shard hashes on read, and the filesystem-safe package slug.
//! The per-artifact projection (record/mapping shapes, per-shard hash inputs)
//! lives with each producer: [`crate::compile`] and [`crate::index`].

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_types::{Config, Error, parse_semver};

use crate::hash;
use crate::pathutil::rel_posix;

/// Reject a shard whose schema MAJOR differs from this build's (the versioning
/// policy: a build understands its own MAJOR line only). Mirrors
/// `query::reject_unknown_major`, applied per shard at the read boundary so a
/// stale-major shard fails with a clean [`Error::Schema`] (exit 3) rather than a
/// silent misread.
pub fn check_major(what: &str, found: &str, ours: &str) -> Result<(), Error> {
    let (want_major, ..) = parse_semver(ours).expect("our own version constant is semver");
    let (got_major, ..) = parse_semver(found)
        .ok_or_else(|| Error::Schema(format!("{what} schemaVersion '{found}' is not semver")))?;
    if got_major != want_major {
        return Err(Error::Schema(format!(
            "{what} schema MAJOR {got_major} is unsupported (this build understands {want_major}.x)"
        )));
    }
    Ok(())
}

/// A set of shard files to write into one directory: `(filename, content)`,
/// each content already canonical JSON.
pub type ShardFiles = Vec<(String, String)>;

/// The shard subdirectory holding per-spec shards under an artifact's dir.
pub const BY_SPEC_DIR: &str = "by-spec";
/// The shard subdirectory holding per-package shards under an artifact's dir.
pub const BY_PACKAGE_DIR: &str = "by-package";

/// The scalar folded into every shard's `shardHash` so a change to a globally
/// shared input (the `spec-spine.toml` config and any `index.extra_hashed_inputs`
/// file) restamps every shard rather than silently staling none. A config edit
/// is rare and inherently global, and two PRs that touch only disjoint specs do
/// not touch it, so the conflict-free property is unaffected. Pure function of
/// `(config, file contents)`. The registry deliberately does not fold this (its
/// pre-shard hash covered `spec.md` only); the index does (its pre-shard hash
/// covered `spec-spine.toml` + `extra_hashed_inputs`).
pub fn global_inputs_hash(cfg: &Config, repo_root: &Path) -> String {
    let mut pieces: Vec<(String, String)> = Vec::new();
    let cfg_path = repo_root.join("spec-spine.toml");
    if let Ok(content) = fs::read_to_string(&cfg_path) {
        pieces.push((rel_posix(repo_root, &cfg_path), content));
    }
    for pattern in &cfg.index.extra_hashed_inputs {
        for file in glob_files(repo_root, pattern) {
            if let Ok(content) = fs::read_to_string(&file) {
                pieces.push((rel_posix(repo_root, &file), content));
            }
        }
    }
    pieces.sort_by(|a, b| a.0.cmp(&b.0));
    pieces.dedup_by(|a, b| a.0 == b.0);
    hash::content_hash(pieces)
}

/// The aggregate content hash recomputed from the shard set on read: SHA-256
/// over the sorted `(shardKey, shardHash)` pairs. It is a pure function of the
/// shard hashes (each of which is a pure function of that shard's inputs), so it
/// is identical whether computed at emit or assembled on read, and it is never
/// committed to a shared file. `keyed` entries use a stable, collision-free key
/// such as `"spec:<id>"` / `"package:<slug>"`.
pub fn aggregate_content_hash(keyed: &[(String, String)]) -> String {
    hash::content_hash(keyed.to_vec())
}

/// A filesystem-safe slug for a package-shard filename, derived from the package
/// name: any character outside `[A-Za-z0-9._-]` becomes `_` (so a scoped npm
/// name like `@scope/pkg` yields `_scope_pkg`). Leading dots are escaped so a
/// shard is never a hidden file.
pub fn package_slug(name: &str) -> String {
    let mut slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if slug.starts_with('.') {
        slug.insert(0, '_');
    }
    if slug.is_empty() {
        slug.push('_');
    }
    slug
}

/// Write `files` (`(filename, content)`) into `dir`, creating it, and prune any
/// `*.json` already there whose name is not in `files`. This is what makes a
/// removed spec/package delete its shard: emit is a directory *sync*, not a
/// blind write, so the shard set always equals the current authority set.
pub fn sync_dir(dir: &Path, files: &[(String, String)]) -> Result<(), Error> {
    fs::create_dir_all(dir).map_err(|e| Error::Io(format!("create {}: {e}", dir.display())))?;
    let keep: BTreeSet<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_json = path.extension().and_then(|e| e.to_str()) == Some("json");
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string);
            if is_json && name.as_deref().is_some_and(|n| !keep.contains(n)) {
                fs::remove_file(&path)
                    .map_err(|e| Error::Io(format!("prune {}: {e}", path.display())))?;
            }
        }
    }
    for (name, content) in files {
        let path = dir.join(name);
        fs::write(&path, content)
            .map_err(|e| Error::Io(format!("write {}: {e}", path.display())))?;
    }
    Ok(())
}

/// Read every `*.json` file in `dir`, sorted by filename, as raw bytes. A
/// missing directory yields an empty list (an artifact with no shards of that
/// kind, e.g. a corpus with no packages, or a not-yet-built artifact).
pub fn read_shard_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(out),
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let bytes =
            fs::read(&path).map_err(|e| Error::Io(format!("read {}: {e}", path.display())))?;
        out.push((name, bytes));
    }
    Ok(out)
}

/// Glob `pattern` under `repo_root`, returning matched files, sorted.
fn glob_files(repo_root: &Path, pattern: &str) -> Vec<PathBuf> {
    let joined = repo_root.join(pattern);
    let mut out: Vec<PathBuf> = match glob::glob(&joined.to_string_lossy()) {
        Ok(paths) => paths
            .filter_map(std::result::Result::ok)
            .filter(|p| p.is_file())
            .collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_slug_is_filesystem_safe() {
        assert_eq!(package_slug("spec-spine-cli"), "spec-spine-cli");
        assert_eq!(package_slug("@scope/pkg"), "_scope_pkg");
        assert_eq!(package_slug(".hidden"), "_.hidden");
    }

    #[test]
    fn sync_dir_prunes_removed_shards() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("by-spec");
        sync_dir(
            &dir,
            &[("a.json".into(), "1".into()), ("b.json".into(), "2".into())],
        )
        .unwrap();
        sync_dir(&dir, &[("a.json".into(), "1".into())]).unwrap();
        assert!(dir.join("a.json").is_file());
        assert!(!dir.join("b.json").is_file(), "removed shard is pruned");
    }
}
