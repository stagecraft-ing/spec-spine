//! Shared path helpers for the indexer: repo-relative POSIX paths and
//! exclusion matching against `index.resolver_exclusions`.

use std::path::Path;

/// Repo-relative POSIX path (forward slashes) of `path` under `repo_root`.
pub fn rel_posix(repo_root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// True if any component of `path` (relative to `repo_root`) is an excluded
/// directory name (e.g. `target`, `node_modules`).
pub fn is_excluded(repo_root: &Path, path: &Path, exclusions: &[String]) -> bool {
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    rel.components().any(|c| {
        let seg = c.as_os_str().to_string_lossy();
        exclusions.iter().any(|ex| ex == seg.as_ref())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rel_posix_uses_forward_slashes() {
        let root = PathBuf::from("/repo");
        assert_eq!(rel_posix(&root, &root.join("a").join("b.rs")), "a/b.rs");
    }

    #[test]
    fn exclusion_matches_any_component() {
        let root = PathBuf::from("/repo");
        let ex = vec!["target".to_string(), "node_modules".to_string()];
        assert!(is_excluded(&root, &root.join("crates/x/target/debug"), &ex));
        assert!(is_excluded(&root, &root.join("web/node_modules/pkg"), &ex));
        assert!(!is_excluded(&root, &root.join("crates/x/src/lib.rs"), &ex));
    }
}
