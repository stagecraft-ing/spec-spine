//! The index capability (spec 004): code-as-source view + staleness + authorities.
//!
//! Pure function of `(config, file contents)`. Discovers packages, links code to
//! specs three ways, resolves the file/section/symbol grammar to physical
//! locations, and emits a deterministic `index.json`. All discovery is
//! path-sorted before hashing and emission (watch-item 1).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_types::{
    CodebaseIndex, Diagnostic, Diagnostics, Error, INDEX_SCHEMA_VERSION, ImplementingPath,
    IndexBuild, ResolvedLocation, ResolvedUnit, SourceField, TraceMapping, TraceSource,
    Traceability, Unit, parse_frontmatter,
};

use crate::manifest;
use crate::pathutil::{is_excluded, rel_posix};
use crate::sections;
use crate::symbols::{self, SymbolIndex};
use crate::{canonical_json, hash};

/// Resolver hard-error codes (`I-003`..`I-009`) that fail `index check`.
const BLOCKING_CODES: &[&str] = &[
    "I-003", "I-004", "I-005", "I-006", "I-007", "I-008", "I-009",
];

/// The result of an index run.
pub struct IndexOutcome {
    pub index: CodebaseIndex,
    pub json: String,
}

/// Index freshness relative to current inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale { expected: String, actual: String },
}

/// A spec's ownership declarations, parsed from frontmatter.
struct SpecInfo {
    id: String,
    status: String,
    depends_on: Vec<String>,
    amends: Vec<String>,
    /// (source_field, unit, ownership) for every declared unit.
    units: Vec<(SourceField, Unit, bool)>,
}

/// Build the codebase index under `repo_root`.
pub fn index(cfg: &spec_spine_types::Config, repo_root: &Path) -> Result<IndexOutcome, Error> {
    let discovered = manifest::discover(cfg, repo_root);
    let mut diagnostics = Diagnostics {
        warnings: Vec::new(),
        errors: discovered.diagnostics,
    };

    let specs = discover_specs(cfg, repo_root)?;
    let all_ids: BTreeSet<String> = specs.iter().map(|s| s.id.clone()).collect();

    // Comment-header linkage: file path -> spec id.
    let comment_links = scan_comment_headers(cfg, repo_root, &discovered.packages, &all_ids);

    // Symbol index — built only if some spec declares a symbol unit (avoids
    // parsing all source for corpora that use only file/section units).
    let needs_symbols = specs.iter().any(|s| {
        s.units
            .iter()
            .any(|(_, u, _)| matches!(u, Unit::Symbol { .. }))
    });
    let symbol_index = if needs_symbols {
        symbols::build_symbol_index(
            repo_root,
            &discovered.packages,
            &cfg.index.resolver_exclusions,
        )
    } else {
        SymbolIndex::default()
    };

    // --- traceability mappings ---
    let mut mappings: Vec<TraceMapping> = Vec::new();
    for spec in &specs {
        let mut paths: BTreeMap<String, BTreeSet<TraceSource>> = BTreeMap::new();
        let mut resolved_units: Vec<ResolvedUnit> = Vec::new();

        // Source 3: spec edges (units).
        for (field, unit, ownership) in &spec.units {
            let locations =
                resolve_unit(repo_root, unit, &symbol_index, &spec.id, &mut diagnostics);
            for loc in &locations {
                paths
                    .entry(loc.file.clone())
                    .or_default()
                    .insert(TraceSource::SpecEdge);
            }
            resolved_units.push(ResolvedUnit {
                unit: unit.clone(),
                source_field: *field,
                ownership: *ownership,
                locations,
            });
        }
        // Source 1: manifest metadata.
        for pkg in discovered
            .packages
            .iter()
            .filter(|p| p.spec_ref.as_deref() == Some(&spec.id))
        {
            paths
                .entry(pkg.path.clone())
                .or_default()
                .insert(TraceSource::ManifestMetadata);
        }
        // Source 2: comment headers.
        for (file, sid) in &comment_links {
            if sid == &spec.id {
                paths
                    .entry(file.clone())
                    .or_default()
                    .insert(TraceSource::CommentHeader);
            }
        }

        let implementing_paths: Vec<ImplementingPath> = paths
            .into_iter()
            .map(|(path, sources)| ImplementingPath {
                path,
                source: collapse_sources(&sources),
            })
            .collect();

        resolved_units.sort_by(|a, b| {
            (a.source_field as u8, canonical_unit(&a.unit))
                .cmp(&(b.source_field as u8, canonical_unit(&b.unit)))
        });

        mappings.push(TraceMapping {
            spec_id: spec.id.clone(),
            spec_status: Some(spec.status.clone()),
            depends_on: spec.depends_on.clone(),
            amends: spec
                .amends
                .iter()
                .map(|a| resolve_id(a, &all_ids))
                .collect(),
            amendment_record: None,
            implementing_paths,
            resolved_units,
        });
    }
    mappings.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));

    // orphaned specs: claim nothing that resolves anywhere.
    let orphaned_specs: Vec<String> = mappings
        .iter()
        .filter(|m| m.implementing_paths.is_empty())
        .map(|m| m.spec_id.clone())
        .collect();

    // untraced code: packages with neither a spec_ref nor any implementing path inside them.
    let claimed: BTreeSet<&str> = mappings
        .iter()
        .flat_map(|m| m.implementing_paths.iter().map(|p| p.path.as_str()))
        .collect();
    let mut untraced_code: Vec<String> = discovered
        .packages
        .iter()
        .filter(|p| {
            p.spec_ref.is_none()
                && !claimed
                    .iter()
                    .any(|c| c == &p.path || c.starts_with(&format!("{}/", p.path)))
        })
        .map(|p| p.path.clone())
        .collect();
    untraced_code.sort();

    // --- content hash over path-sorted manifests + specs + extra inputs ---
    let content_hash = hash::content_hash(collect_hash_inputs(
        cfg,
        repo_root,
        &discovered.manifest_paths,
    )?);

    let codebase_index = CodebaseIndex {
        schema_version: INDEX_SCHEMA_VERSION.to_string(),
        build: IndexBuild {
            indexer_id: cfg.branding.indexer_id.clone(),
            indexer_version: env!("CARGO_PKG_VERSION").to_string(),
            repo_root: ".".to_string(),
            content_hash,
        },
        packages: discovered.packages,
        traceability: Traceability {
            mappings,
            orphaned_specs,
            untraced_code,
        },
        diagnostics,
    };
    let json = canonical_json::to_string(&codebase_index)?;
    Ok(IndexOutcome {
        index: codebase_index,
        json,
    })
}

/// Recompute the content hash and compare it to the committed `index.json`.
pub fn check_index_freshness(
    cfg: &spec_spine_types::Config,
    repo_root: &Path,
) -> Result<Freshness, Error> {
    let index_path = repo_root
        .join(&cfg.layout.derived_dir)
        .join("codebase-index")
        .join("index.json");
    let bytes = fs::read(&index_path).map_err(|e| {
        Error::Io(format!(
            "read {} (run `spec-spine index` first?): {e}",
            index_path.display()
        ))
    })?;
    let committed = crate::load_index(&bytes)?;

    // Blocking resolver diagnostics also fail freshness.
    if committed
        .diagnostics
        .errors
        .iter()
        .any(|d| BLOCKING_CODES.contains(&d.code.as_str()))
    {
        return Ok(Freshness::Stale {
            expected: "no blocking diagnostics".to_string(),
            actual: "blocking resolver diagnostics present".to_string(),
        });
    }

    let discovered = manifest::discover(cfg, repo_root);
    let actual = hash::content_hash(collect_hash_inputs(
        cfg,
        repo_root,
        &discovered.manifest_paths,
    )?);
    if actual == committed.build.content_hash {
        Ok(Freshness::Fresh)
    } else {
        Ok(Freshness::Stale {
            expected: committed.build.content_hash,
            actual,
        })
    }
}

/// "Who currently owns this unit?" — a set query over resolved traceability.
pub fn authorities(index: &CodebaseIndex, unit: &Unit) -> Vec<String> {
    let mut owners: BTreeSet<String> = BTreeSet::new();
    for mapping in &index.traceability.mappings {
        for ru in &mapping.resolved_units {
            if ru.ownership && &ru.unit == unit {
                owners.insert(mapping.spec_id.clone());
            }
        }
        if let Unit::File { path } = unit {
            if mapping.implementing_paths.iter().any(|p| &p.path == path) {
                owners.insert(mapping.spec_id.clone());
            }
        }
    }
    owners.into_iter().collect()
}

// ===== helpers =====

fn discover_specs(
    cfg: &spec_spine_types::Config,
    repo_root: &Path,
) -> Result<Vec<SpecInfo>, Error> {
    let specs_dir = repo_root.join(&cfg.layout.specs_dir);
    let mut out = Vec::new();
    let entries = fs::read_dir(&specs_dir).map_err(|e| {
        Error::Io(format!(
            "cannot read specs dir {}: {e}",
            specs_dir.display()
        ))
    })?;
    let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    for dir in dirs {
        let spec_md = dir.join("spec.md");
        if !spec_md.is_file() {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&spec_md) else {
            continue;
        };
        let Ok(fm) = parse_frontmatter(&raw) else {
            continue; // compile reports the V-002; the index skips it
        };
        let mut units: Vec<(SourceField, Unit, bool)> = Vec::new();
        for u in &fm.establishes {
            units.push((SourceField::Establishes, u.clone(), true));
        }
        for e in &fm.extends {
            if let Some(u) = &e.unit {
                units.push((SourceField::Extends, u.clone(), true));
            }
        }
        for r in &fm.refines {
            if let Some(u) = &r.unit {
                units.push((SourceField::Refines, u.clone(), true));
            }
        }
        for c in &fm.co_authority {
            units.push((SourceField::CoAuthority, c.unit.clone(), true));
        }
        for c in &fm.constrains {
            units.push((SourceField::Constrains, c.unit.clone(), true));
        }
        for r in &fm.references {
            if let Some(u) = &r.unit {
                units.push((SourceField::References, u.clone(), false));
            }
        }
        out.push(SpecInfo {
            id: fm.id,
            status: status_str(fm.status),
            depends_on: fm.depends_on,
            amends: fm.amends,
            units,
        });
    }
    Ok(out)
}

fn resolve_unit(
    repo_root: &Path,
    unit: &Unit,
    symbols: &SymbolIndex,
    spec_id: &str,
    diagnostics: &mut Diagnostics,
) -> Vec<ResolvedLocation> {
    match unit {
        Unit::File { path } => {
            let abs = repo_root.join(path);
            if abs.exists() {
                vec![ResolvedLocation {
                    file: path.clone(),
                    span: None,
                }]
            } else {
                diagnostics.errors.push(Diagnostic {
                    code: "I-004".to_string(),
                    message: format!("spec '{spec_id}' file unit '{path}' does not exist"),
                    path: Some(path.clone()),
                });
                Vec::new()
            }
        }
        Unit::Section { file, anchor } => {
            let abs = repo_root.join(file);
            let span = fs::read_to_string(&abs)
                .ok()
                .and_then(|content| sections::resolve_section(&content, file, anchor));
            match span {
                Some(span) => vec![ResolvedLocation {
                    file: file.clone(),
                    span: Some(span),
                }],
                None => {
                    diagnostics.errors.push(Diagnostic {
                        code: "I-006".to_string(),
                        message: format!(
                            "spec '{spec_id}' section unit '{anchor}' not found in {file}"
                        ),
                        path: Some(file.clone()),
                    });
                    Vec::new()
                }
            }
        }
        Unit::Symbol { id } => {
            let locations = symbols.resolve(id);
            if locations.is_empty() {
                diagnostics.errors.push(Diagnostic {
                    code: "I-005".to_string(),
                    message: format!("spec '{spec_id}' symbol unit '{id}' did not resolve"),
                    path: None,
                });
            }
            locations
        }
    }
}

/// Scan package source files for a `// Spec: <specs_dir>/NNN-slug/spec.md` header.
fn scan_comment_headers(
    cfg: &spec_spine_types::Config,
    repo_root: &Path,
    packages: &[spec_spine_types::PackageRecord],
    all_ids: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mut links: Vec<(String, String)> = Vec::new();
    let exts = ["rs", "ts", "tsx", "js", "jsx", "go", "py", "sh"];
    for pkg in packages {
        let pkg_dir = repo_root.join(&pkg.path);
        for file in walk_source(&pkg_dir, &exts, repo_root, &cfg.index.resolver_exclusions) {
            let Ok(content) = fs::read_to_string(&file) else {
                continue;
            };
            for line in content.lines().take(16) {
                let t = line.trim_start();
                let body = t
                    .strip_prefix("//")
                    .or_else(|| t.strip_prefix('#'))
                    .unwrap_or(t);
                if let Some(rest) = body.trim_start().strip_prefix("Spec:") {
                    if let Some(id) = spec_id_from_path(rest.trim(), all_ids) {
                        links.push((rel_posix(repo_root, &file), id));
                    }
                    break;
                }
            }
        }
    }
    links.sort();
    links.dedup();
    links
}

/// Extract the spec id from a `<specs_dir>/NNN-slug/spec.md` reference.
fn spec_id_from_path(reference: &str, all_ids: &BTreeSet<String>) -> Option<String> {
    let trimmed = reference.trim_end_matches("/spec.md");
    let candidate = trimmed.rsplit('/').next().unwrap_or(trimmed);
    all_ids.contains(candidate).then(|| candidate.to_string())
}

fn collect_hash_inputs(
    cfg: &spec_spine_types::Config,
    repo_root: &Path,
    manifest_paths: &[PathBuf],
) -> Result<Vec<(String, String)>, Error> {
    let mut pieces: Vec<(String, String)> = Vec::new();
    let push = |abs: &Path, pieces: &mut Vec<(String, String)>| {
        if let Ok(content) = fs::read_to_string(abs) {
            pieces.push((rel_posix(repo_root, abs), content));
        }
    };

    // Manifests.
    for m in manifest_paths {
        push(m, &mut pieces);
    }
    // Every spec.md.
    let specs_dir = repo_root.join(&cfg.layout.specs_dir);
    if let Ok(entries) = fs::read_dir(&specs_dir) {
        for entry in entries.filter_map(std::result::Result::ok) {
            let spec_md = entry.path().join("spec.md");
            if spec_md.is_file() {
                push(&spec_md, &mut pieces);
            }
        }
    }
    // The config itself.
    let cfg_path = repo_root.join("spec-spine.toml");
    if cfg_path.is_file() {
        push(&cfg_path, &mut pieces);
    }
    // Adopter-declared extra inputs.
    for pattern in &cfg.index.extra_hashed_inputs {
        for file in glob_files(repo_root, pattern) {
            push(&file, &mut pieces);
        }
    }
    // content_hash sorts by path; order here is irrelevant.
    Ok(pieces)
}

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

fn walk_source(dir: &Path, exts: &[&str], repo_root: &Path, exclusions: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, exts, repo_root, exclusions, &mut out);
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

fn collapse_sources(sources: &BTreeSet<TraceSource>) -> TraceSource {
    if sources.len() > 1 {
        TraceSource::Multiple
    } else {
        *sources.iter().next().unwrap_or(&TraceSource::SpecEdge)
    }
}

/// A short id (`001`) resolves to the full id (`001-slug`) by unique prefix.
fn resolve_id(short: &str, all_ids: &BTreeSet<String>) -> String {
    if all_ids.contains(short) {
        return short.to_string();
    }
    let matches: Vec<&String> = all_ids
        .iter()
        .filter(|id| id.split('-').next() == Some(short))
        .collect();
    if matches.len() == 1 {
        matches[0].clone()
    } else {
        short.to_string()
    }
}

/// A stable canonical string for a unit, for deterministic sorting.
fn canonical_unit(unit: &Unit) -> String {
    match unit {
        Unit::File { path } => format!("file:{path}"),
        Unit::Section { file, anchor } => format!("section:{file}#{anchor}"),
        Unit::Symbol { id } => format!("symbol:{id}"),
    }
}

fn status_str(status: spec_spine_types::Status) -> String {
    use spec_spine_types::Status::*;
    match status {
        Draft => "draft",
        Approved => "approved",
        Superseded => "superseded",
        Retired => "retired",
    }
    .to_string()
}
