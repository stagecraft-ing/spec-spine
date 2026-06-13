//! Package discovery (spec 004 §3.1): the configurable manifest scan.
//!
//! Rust crates come from the root Cargo workspace members plus
//! `layout.standalone_rust_workspaces`; npm/pnpm packages from the workspace
//! globs declared by `layout.npm_workspaces` (the default reads root
//! `package.json#workspaces`, the template-encore fix) plus
//! `layout.standalone_npm_packages`. The owning spec is read from the configurable
//! `manifest.metadata_namespace`. Everything is path-sorted for determinism and
//! `index.resolver_exclusions` directories are never descended.

use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_types::{Config, Diagnostic, PackageKind, PackageRecord};

use crate::pathutil::{is_excluded, rel_posix};

/// Discovered packages plus the manifest paths to fold into the content hash.
pub struct Discovered {
    pub packages: Vec<PackageRecord>,
    pub manifest_paths: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Discover all Rust and npm packages under `repo_root`.
pub fn discover(cfg: &Config, repo_root: &Path) -> Discovered {
    let mut packages = Vec::new();
    let mut manifest_paths = Vec::new();
    let mut diagnostics = Vec::new();

    discover_rust(
        cfg,
        repo_root,
        &mut packages,
        &mut manifest_paths,
        &mut diagnostics,
    );
    discover_npm(
        cfg,
        repo_root,
        &mut packages,
        &mut manifest_paths,
        &mut diagnostics,
    );

    packages.sort_by(|a, b| a.path.cmp(&b.path));
    packages.dedup_by(|a, b| a.path == b.path);
    manifest_paths.sort();
    manifest_paths.dedup();
    Discovered {
        packages,
        manifest_paths,
        diagnostics,
    }
}

// ===== Rust =====

fn discover_rust(
    cfg: &Config,
    repo_root: &Path,
    packages: &mut Vec<PackageRecord>,
    manifests: &mut Vec<PathBuf>,
    diags: &mut Vec<Diagnostic>,
) {
    let root_manifest = repo_root.join(&cfg.layout.cargo_workspace);
    let mut members: Vec<String> = Vec::new();
    if let Ok(src) = fs::read_to_string(&root_manifest) {
        manifests.push(root_manifest.clone());
        if let Ok(doc) = src.parse::<toml::Value>() {
            // A root [package], if present, is itself a crate.
            if doc.get("package").is_some() {
                if let Some(rec) = parse_cargo(
                    repo_root,
                    &root_manifest,
                    &cfg.manifest.metadata_namespace,
                    diags,
                ) {
                    packages.push(rec);
                }
            }
            if let Some(arr) = doc
                .get("workspace")
                .and_then(|w| w.get("members"))
                .and_then(|m| m.as_array())
            {
                members.extend(arr.iter().filter_map(|v| v.as_str().map(String::from)));
            }
        }
    }
    members.extend(cfg.layout.standalone_rust_workspaces.iter().cloned());

    for member in &members {
        for manifest in glob_manifests(
            repo_root,
            member,
            "Cargo.toml",
            &cfg.index.resolver_exclusions,
        ) {
            manifests.push(manifest.clone());
            if let Some(rec) = parse_cargo(
                repo_root,
                &manifest,
                &cfg.manifest.metadata_namespace,
                diags,
            ) {
                packages.push(rec);
            }
        }
    }
}

fn parse_cargo(
    repo_root: &Path,
    manifest: &Path,
    namespace: &str,
    diags: &mut Vec<Diagnostic>,
) -> Option<PackageRecord> {
    let src = fs::read_to_string(manifest).ok()?;
    let doc = match src.parse::<toml::Value>() {
        Ok(d) => d,
        Err(e) => {
            diags.push(diag(
                "I-001",
                format!("cannot parse {}: {e}", manifest.display()),
                repo_root,
                manifest,
            ));
            return None;
        }
    };
    let pkg = doc.get("package")?;
    let name = pkg.get("name").and_then(|v| v.as_str())?.to_string();
    let dir = manifest.parent().unwrap_or(repo_root);

    let has_lib = doc.get("lib").is_some() || dir.join("src/lib.rs").is_file();
    let has_bin = doc.get("bin").is_some() || dir.join("src/main.rs").is_file();
    let kind = match (has_lib, has_bin) {
        (true, true) => PackageKind::RustLibBin,
        (false, true) => PackageKind::RustBin,
        _ => PackageKind::RustLib,
    };

    Some(PackageRecord {
        name,
        path: rel_posix(repo_root, dir),
        kind,
        version: pkg
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        edition: pkg
            .get("edition")
            .and_then(|v| v.as_str())
            .map(String::from),
        spec_ref: pkg
            .get("metadata")
            .and_then(|m| m.get(namespace))
            .and_then(|n| n.get("spec"))
            .and_then(|s| s.as_str())
            .map(String::from),
    })
}

// ===== npm / pnpm =====

fn discover_npm(
    cfg: &Config,
    repo_root: &Path,
    packages: &mut Vec<PackageRecord>,
    manifests: &mut Vec<PathBuf>,
    diags: &mut Vec<Diagnostic>,
) {
    let mut globs: Vec<String> = Vec::new();

    for decl in &cfg.layout.npm_workspaces {
        let decl_path = repo_root.join(decl);
        if !decl_path.is_file() {
            continue;
        }
        let Ok(src) = fs::read_to_string(&decl_path) else {
            continue;
        };
        if decl.ends_with(".json") {
            // package.json: a `workspaces` array, or `{ "workspaces": { "packages": [...] } }`.
            if let Ok(doc) = serde_json::from_str::<serde_json::Value>(&src) {
                manifests.push(decl_path.clone());
                let ws = doc.get("workspaces");
                if let Some(arr) = ws.and_then(|w| w.as_array()) {
                    globs.extend(arr.iter().filter_map(|v| v.as_str().map(String::from)));
                } else if let Some(arr) = ws
                    .and_then(|w| w.get("packages"))
                    .and_then(|p| p.as_array())
                {
                    globs.extend(arr.iter().filter_map(|v| v.as_str().map(String::from)));
                }
                // The root package.json that declares workspaces is itself a record.
                if doc.get("name").is_some() {
                    if let Some(rec) =
                        npm_record(repo_root, &decl_path, &cfg.manifest.metadata_namespace)
                    {
                        packages.push(rec);
                    }
                }
            }
        } else {
            // pnpm-workspace.yaml: a `packages` list.
            if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&src) {
                manifests.push(decl_path.clone());
                if let Some(arr) = doc.get("packages").and_then(|p| p.as_sequence()) {
                    globs.extend(arr.iter().filter_map(|v| v.as_str().map(String::from)));
                }
            }
        }
    }

    for member in globs
        .iter()
        .chain(cfg.layout.standalone_npm_packages.iter())
    {
        for manifest in glob_manifests(
            repo_root,
            member,
            "package.json",
            &cfg.index.resolver_exclusions,
        ) {
            manifests.push(manifest.clone());
            if let Some(rec) = npm_record(repo_root, &manifest, &cfg.manifest.metadata_namespace) {
                packages.push(rec);
            } else {
                diags.push(diag(
                    "I-002",
                    "cannot parse package.json".into(),
                    repo_root,
                    &manifest,
                ));
            }
        }
    }
}

fn npm_record(repo_root: &Path, manifest: &Path, namespace: &str) -> Option<PackageRecord> {
    let src = fs::read_to_string(manifest).ok()?;
    let doc = serde_json::from_str::<serde_json::Value>(&src).ok()?;
    let name = doc.get("name").and_then(|v| v.as_str())?.to_string();
    let dir = manifest.parent().unwrap_or(repo_root);
    let kind = if doc.get("workspaces").is_some() {
        PackageKind::NpmWorkspace
    } else {
        PackageKind::NpmPackage
    };
    Some(PackageRecord {
        name,
        path: rel_posix(repo_root, dir),
        kind,
        version: doc
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        edition: None,
        spec_ref: doc
            .get(namespace)
            .and_then(|n| n.get("spec"))
            .and_then(|s| s.as_str())
            .map(String::from),
    })
}

/// The governance projection of an npm manifest: exactly the fields
/// discovery consumes (`npm_record` + workspace-glob extraction): `name`,
/// `version`, `workspaces`, and the adopter's metadata-namespace object.
/// The index content hash folds this INSTEAD of the raw bytes (spec 004
/// §3.5 amendment, 2026-06-11): a dependency-table version bump
/// (dependabot-class) is not a governed input and must not stale the
/// committed index, while any change to a field the indexer actually reads
/// still does. `None` (unparseable / non-object) tells the caller to fall
/// back to raw bytes: over-hashing is the fail-closed direction.
pub fn npm_hash_projection(content: &str, namespace: &str) -> Option<String> {
    let doc: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = doc.as_object()?;
    let mut proj = serde_json::Map::new();
    for key in ["name", "version", "workspaces"] {
        if let Some(v) = obj.get(key) {
            proj.insert(key.to_string(), v.clone());
        }
    }
    if let Some(v) = obj.get(namespace) {
        proj.insert(namespace.to_string(), v.clone());
    }
    serde_json::to_string(&serde_json::Value::Object(proj)).ok()
}

// ===== shared =====

/// Expand `<member>/<manifest_file>` under `repo_root` (member may contain glob
/// metacharacters), returning matches not inside an excluded directory, sorted.
fn glob_manifests(
    repo_root: &Path,
    member: &str,
    manifest_file: &str,
    exclusions: &[String],
) -> Vec<PathBuf> {
    let pattern = repo_root.join(member).join(manifest_file);
    let pattern = pattern.to_string_lossy();
    let mut out: Vec<PathBuf> = match glob::glob(&pattern) {
        Ok(paths) => paths
            .filter_map(std::result::Result::ok)
            .filter(|p| !is_excluded(repo_root, p, exclusions))
            .collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out.dedup();
    out
}

fn diag(code: &str, message: String, repo_root: &Path, path: &Path) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        message,
        path: Some(rel_posix(repo_root, path)),
    }
}
