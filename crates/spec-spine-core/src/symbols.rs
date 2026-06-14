//! Symbol resolution (spec 004 §3.3): build a deterministic index from
//! `::`-qualified ids to physical `(file, line-span)` locations via tree-sitter.
//!
//! Symbol indexing covers top-level items only (no `impl` methods, no inline
//! `mod` bodies) for Rust (`.rs`) and TypeScript (`.ts`/`.tsx`). The module index
//! (spec 017) additionally resolves top-level inline `mod X { ... }` blocks to
//! their block spans. The tree-sitter core and grammar crates are pinned exactly
//! so spans are identical across platforms.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_types::{LineSpan, PackageKind, PackageRecord, ResolvedLocation};
use tree_sitter::{Language, Node, Parser};

use crate::pathutil::{is_excluded, rel_posix};

/// A resolved symbol index: qualified id → sorted physical locations.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    map: BTreeMap<String, Vec<ResolvedLocation>>,
}

impl SymbolIndex {
    /// Locations for `id` (empty if unknown).
    pub fn resolve(&self, id: &str) -> Vec<ResolvedLocation> {
        self.map.get(id).cloned().unwrap_or_default()
    }

    #[cfg(test)]
    pub fn ids(&self) -> Vec<&str> {
        self.map.keys().map(String::as_str).collect()
    }
}

/// A resolved Rust module index (spec 017): `::`-qualified module path → physical
/// locations. File-modules resolve whole-file (`span: None`); a top-level inline
/// `mod X { ... }` block resolves to its block span. TypeScript carries no
/// analogous module authority unit in the corpus, so this is Rust-only.
#[derive(Debug, Default)]
pub struct ModuleIndex {
    map: BTreeMap<String, Vec<ResolvedLocation>>,
}

impl ModuleIndex {
    /// Locations for module `id` (empty if unknown).
    pub fn resolve(&self, id: &str) -> Vec<ResolvedLocation> {
        self.map.get(id).cloned().unwrap_or_default()
    }
}

/// Build the Rust module index across all Rust packages. Deterministic: packages
/// and files are processed in sorted order and every id's locations are sorted.
pub fn build_module_index(
    repo_root: &Path,
    packages: &[PackageRecord],
    exclusions: &[String],
) -> ModuleIndex {
    let mut map: BTreeMap<String, Vec<ResolvedLocation>> = BTreeMap::new();
    for pkg in packages {
        if !matches!(
            pkg.kind,
            PackageKind::RustLib | PackageKind::RustBin | PackageKind::RustLibBin
        ) {
            continue;
        }
        let crate_name = pkg.name.replace('-', "_");
        let src_dir = repo_root.join(&pkg.path).join("src");
        for file in walk_files(&src_dir, &["rs"], repo_root, exclusions) {
            let Ok(content) = fs::read_to_string(&file) else {
                continue;
            };
            let module = rust_module_path(&src_dir, &file);
            let rel = rel_posix(repo_root, &file);
            // File-module: the file's own module path → whole file (the crate
            // root, `lib.rs`/`main.rs`, resolves to the bare crate name).
            let file_mod_id = qualify_module(&crate_name, &module);
            map.entry(file_mod_id).or_default().push(ResolvedLocation {
                file: rel.clone(),
                span: None,
            });
            // Top-level inline `mod X { ... }` blocks → block span.
            for (name, span) in extract_inline_mods(&content) {
                let mut path = module.clone();
                path.push(name);
                let id = qualify_module(&crate_name, &path);
                map.entry(id).or_default().push(ResolvedLocation {
                    file: rel.clone(),
                    span: Some(span),
                });
            }
        }
    }
    for locs in map.values_mut() {
        locs.sort_by(|a, b| {
            (a.file.as_str(), a.span.map(|s| s.start_line))
                .cmp(&(b.file.as_str(), b.span.map(|s| s.start_line)))
        });
        locs.dedup();
    }
    ModuleIndex { map }
}

/// `crate::seg::…::seg` for a module path; the crate root is the bare crate name.
fn qualify_module(crate_name: &str, module: &[String]) -> String {
    let mut parts = Vec::with_capacity(module.len() + 1);
    parts.push(crate_name.to_string());
    parts.extend(module.iter().cloned());
    parts.join("::")
}

/// Top-level `mod X { ... }` blocks (those with a body) and their spans. A
/// bodyless `mod X;` is skipped: the file-module entry for its file covers it.
fn extract_inline_mods(src: &str) -> Vec<(String, LineSpan)> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let Some(tree) = parser.parse(src, None) else {
        return Vec::new();
    };
    let root = tree.root_node();
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "mod_item" && child.child_by_field_name("body").is_some() {
            if let Some(sym) = symbol_of(child, src) {
                out.push(sym);
            }
        }
    }
    out
}

/// Build the symbol index across all packages. Deterministic: packages and
/// files are processed in sorted order and every id's locations are sorted.
pub fn build_symbol_index(
    repo_root: &Path,
    packages: &[PackageRecord],
    exclusions: &[String],
) -> SymbolIndex {
    let mut map: BTreeMap<String, Vec<ResolvedLocation>> = BTreeMap::new();
    for pkg in packages {
        let pkg_dir = repo_root.join(&pkg.path);
        match pkg.kind {
            PackageKind::RustLib | PackageKind::RustBin | PackageKind::RustLibBin => {
                index_rust(repo_root, pkg, &pkg_dir, exclusions, &mut map);
            }
            PackageKind::NpmPackage | PackageKind::NpmWorkspace => {
                index_ts(repo_root, pkg, &pkg_dir, exclusions, &mut map);
            }
        }
    }
    for locs in map.values_mut() {
        locs.sort_by(|a, b| {
            (a.file.as_str(), a.span.map(|s| s.start_line))
                .cmp(&(b.file.as_str(), b.span.map(|s| s.start_line)))
        });
        locs.dedup();
    }
    SymbolIndex { map }
}

// ===== Rust =====

const RUST_KINDS: &[&str] = &[
    "function_item",
    "struct_item",
    "enum_item",
    "union_item",
    "trait_item",
    "const_item",
    "static_item",
    "type_item",
    "mod_item",
];

fn index_rust(
    repo_root: &Path,
    pkg: &PackageRecord,
    pkg_dir: &Path,
    exclusions: &[String],
    map: &mut BTreeMap<String, Vec<ResolvedLocation>>,
) {
    let crate_name = pkg.name.replace('-', "_");
    let src_dir = pkg_dir.join("src");
    for file in walk_files(&src_dir, &["rs"], repo_root, exclusions) {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        let module = rust_module_path(&src_dir, &file);
        let rel = rel_posix(repo_root, &file);
        for (name, span) in extract(&content, tree_sitter_rust::LANGUAGE.into(), RUST_KINDS) {
            let id = qualify(&crate_name, &module, &name);
            map.entry(id).or_default().push(ResolvedLocation {
                file: rel.clone(),
                span: Some(span),
            });
        }
    }
}

/// Rust module path from a file under `src/`: `src/lib.rs`/`main.rs`/`mod.rs`
/// → crate root (`[]`); `src/foo.rs` → `["foo"]`; `src/foo/bar.rs` → `["foo","bar"]`.
fn rust_module_path(src_dir: &Path, file: &Path) -> Vec<String> {
    let rel = file.strip_prefix(src_dir).unwrap_or(file);
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if let Some(last) = parts.pop() {
        let stem = last.trim_end_matches(".rs");
        if !matches!(stem, "lib" | "main" | "mod") {
            parts.push(stem.to_string());
        }
    }
    parts
}

// ===== TypeScript =====

const TS_KINDS: &[&str] = &[
    "function_declaration",
    "class_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
];

fn index_ts(
    repo_root: &Path,
    pkg: &PackageRecord,
    pkg_dir: &Path,
    exclusions: &[String],
    map: &mut BTreeMap<String, Vec<ResolvedLocation>>,
) {
    for file in walk_files(pkg_dir, &["ts", "tsx"], repo_root, exclusions) {
        // .vue and .d.ts are out of v1 scope.
        if file.to_string_lossy().ends_with(".d.ts") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        let module = ts_module_path(pkg_dir, &file);
        let rel = rel_posix(repo_root, &file);
        for (name, span) in extract(
            &content,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            TS_KINDS,
        ) {
            let id = qualify(&pkg.name, &module, &name);
            map.entry(id).or_default().push(ResolvedLocation {
                file: rel.clone(),
                span: Some(span),
            });
        }
    }
}

/// TS module path from a file under the package dir: components minus extension,
/// dropping a trailing `index`.
fn ts_module_path(pkg_dir: &Path, file: &Path) -> Vec<String> {
    let rel = file.strip_prefix(pkg_dir).unwrap_or(file);
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if let Some(last) = parts.pop() {
        let stem = last.rsplit_once('.').map(|(s, _)| s).unwrap_or(&last);
        if stem != "index" {
            parts.push(stem.to_string());
        }
    }
    parts
}

// ===== shared =====

/// `prefix::module::...::name` (module segments omitted when empty).
fn qualify(prefix: &str, module: &[String], name: &str) -> String {
    let mut parts = Vec::with_capacity(module.len() + 2);
    parts.push(prefix.to_string());
    parts.extend(module.iter().cloned());
    parts.push(name.to_string());
    parts.join("::")
}

/// Parse `src` and return `(item_name, span)` for each top-level node whose kind
/// is in `kinds`, unwrapping a TS `export_statement` to reach its declaration.
fn extract(src: &str, language: Language, kinds: &[&str]) -> Vec<(String, LineSpan)> {
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(src, None) else {
        return Vec::new();
    };
    let root = tree.root_node();
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        let decl = if child.kind() == "export_statement" {
            child.child_by_field_name("declaration").unwrap_or(child)
        } else {
            child
        };
        if kinds.contains(&decl.kind()) {
            if let Some(sym) = symbol_of(decl, src) {
                out.push(sym);
            }
        }
    }
    out
}

fn symbol_of(node: Node, src: &str) -> Option<(String, LineSpan)> {
    let name = node
        .child_by_field_name("name")?
        .utf8_text(src.as_bytes())
        .ok()?
        .to_string();
    Some((
        name,
        LineSpan::new(node.start_position().row + 1, node.end_position().row + 1),
    ))
}

/// Recursively collect files with one of `exts` under `root`, sorted, skipping
/// excluded directories.
fn walk_files(root: &Path, exts: &[&str], repo_root: &Path, exclusions: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, exts, repo_root, exclusions, &mut out);
    out.sort();
    out
}

fn walk(
    dir: &Path,
    exts: &[&str],
    repo_root: &Path,
    exclusions: &[String],
    out: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if is_excluded(repo_root, &path, exclusions) {
            continue;
        }
        if path.is_dir() {
            walk(&path, exts, repo_root, exclusions, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| exts.contains(&e))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_module_qualification() {
        let src = Path::new("/p/src");
        assert!(rust_module_path(src, Path::new("/p/src/lib.rs")).is_empty());
        assert_eq!(
            rust_module_path(src, Path::new("/p/src/compile.rs")),
            vec!["compile"]
        );
        assert_eq!(
            rust_module_path(src, Path::new("/p/src/index/mod.rs")),
            vec!["index"]
        );
        assert_eq!(
            rust_module_path(src, Path::new("/p/src/index/foo.rs")),
            vec!["index", "foo"]
        );
    }

    #[test]
    fn extract_rust_items() {
        let items = extract(
            "fn a(){}\nstruct B{}\n",
            tree_sitter_rust::LANGUAGE.into(),
            RUST_KINDS,
        );
        let names: Vec<&str> = items.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["a", "B"]);
    }
}
