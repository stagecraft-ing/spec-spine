//! The compile capability (spec 001): markdown corpus → deterministic registry.
//!
//! Pure function of `(config, file contents)`. Per-spec parse failures are
//! recorded as error-tier violations rather than aborting the run; `Err` is
//! reserved for I/O failures. The wall clock is never read here — it lives in
//! `build-meta.json`, written by the CLI.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use spec_spine_types::{
    Build, Config, Error, Frontmatter, FrontmatterIssue, REGISTRY_SCHEMA_VERSION, Registry,
    Severity, SpecRecord, Status, ValidationReport, Violation, parse_frontmatter_with,
    split_frontmatter,
};

use crate::{canonical_json, hash, markdown};

/// The cap on **undeclared** `extra_frontmatter` keys before `V-007` fires.
/// Keys listed in `frontmatter.extra_known_keys` are intentional and exempt;
/// the cap targets escape-hatch abuse (ported from OAP's ~8-entry V-002 cap).
pub const MAX_UNDECLARED_EXTRA_FRONTMATTER: usize = 8;

/// The result of a compile: the typed registry, its canonical JSON bytes, and
/// the validation flag the CLI maps to an exit code.
pub struct CompileOutcome {
    pub registry: Registry,
    pub json: String,
    pub validation_passed: bool,
}

/// Compile the spec corpus under `repo_root` into a registry.
///
/// Returns `Err` only on I/O failure (unreadable specs dir / file). Validation
/// failures are carried inside `registry.validation` with
/// `validation_passed == false`.
pub fn compile(cfg: &Config, repo_root: &Path) -> Result<CompileOutcome, Error> {
    let specs_dir = repo_root.join(&cfg.layout.specs_dir);
    let mut violations: Vec<Violation> = Vec::new();
    let mut hash_pieces: Vec<(String, String)> = Vec::new();

    // --- discover NNN-slug/spec.md, sorted by directory name ---
    let mut spec_files: Vec<(String, PathBuf)> = Vec::new();
    let entries = fs::read_dir(&specs_dir).map_err(|e| {
        Error::Io(format!(
            "cannot read specs dir {}: {e}",
            specs_dir.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::Io(e.to_string()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let spec_md = path.join("spec.md");
        if spec_md.is_file() {
            spec_files.push((entry.file_name().to_string_lossy().into_owned(), spec_md));
        }
    }
    spec_files.sort();

    // --- parse pass (records every file into the content hash) ---
    struct Parsed {
        dirname: String,
        spec_path: String,
        fm: Frontmatter,
        body: String,
    }
    let mut parsed: Vec<Parsed> = Vec::new();

    for (dirname, spec_md) in &spec_files {
        let raw = fs::read_to_string(spec_md)
            .map_err(|e| Error::Io(format!("read {}: {e}", spec_md.display())))?;
        let spec_path = rel_posix(repo_root, spec_md);
        hash_pieces.push((spec_path.clone(), raw.clone()));

        match parse_frontmatter_with(&raw, &cfg.frontmatter.extra_known_keys) {
            Ok(fm) => {
                let body = split_frontmatter(&raw).map(|(_, b)| b).unwrap_or_default();
                parsed.push(Parsed {
                    dirname: dirname.clone(),
                    spec_path,
                    fm,
                    body,
                });
            }
            // V-013 (spec 013 §3.3): a DECLARED extra key carrying a value
            // JSON cannot represent. Same skip-and-continue semantics as
            // V-002 (001 §3.1).
            Err(FrontmatterIssue::UnrepresentableDeclared { key, detail }) => {
                violations.push(error(
                    "V-013",
                    format!(
                        "declared extra-frontmatter key '{key}' carries an unrepresentable YAML value: {detail}"
                    ),
                    Some(spec_path),
                ));
            }
            Err(FrontmatterIssue::Malformed(m)) => violations.push(error(
                "V-002",
                format!("malformed frontmatter: {m}"),
                Some(spec_path),
            )),
        }
    }

    // --- cross-spec sets ---
    let all_ids: std::collections::BTreeSet<String> =
        parsed.iter().map(|p| p.fm.id.clone()).collect();
    let id_paths: Vec<(String, String)> = parsed
        .iter()
        .map(|p| (p.fm.id.clone(), p.spec_path.clone()))
        .collect();
    detect_duplicates(&id_paths, &mut violations);

    // --- short-id resolution (spec 016): rewrite a depends_on / superseded_by
    // reference that names a spec by its leading number (`109`) to the full id
    // (`109-slug`), before validation (V-008/V-010) and record construction see
    // it. A genuinely dangling or ambiguous reference is left unchanged, so its
    // V-code still fires. Resolution is a pure function of the id set, so the
    // registry stays deterministic.
    for p in &mut parsed {
        for dep in &mut p.fm.depends_on {
            *dep = resolve_spec_ref(dep, &all_ids);
        }
        if let Some(by) = p.fm.superseded_by.as_mut() {
            *by = resolve_spec_ref(by, &all_ids);
        }
    }

    // --- per-spec validation + record construction ---
    let mut records: Vec<SpecRecord> = Vec::new();
    for p in parsed {
        validate_spec(
            cfg,
            &p.dirname,
            &p.spec_path,
            &p.fm,
            &all_ids,
            &mut violations,
        );
        records.push(build_record(p.fm, p.spec_path, &p.body));
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));

    // --- assemble registry ---
    let validation = ValidationReport::from_violations(violations);
    let validation_passed = validation.passed;
    let registry = Registry {
        spec_version: REGISTRY_SCHEMA_VERSION.to_string(),
        build: Build {
            compiler_id: cfg.branding.compiler_id.clone(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            input_root: ".".to_string(),
            content_hash: hash::content_hash(hash_pieces),
        },
        specs: records,
        validation,
    };
    let json = canonical_json::to_string(&registry)?;

    Ok(CompileOutcome {
        registry,
        json,
        validation_passed,
    })
}

/// Per-spec validation codes V-001/005/006/007/008/009/010/012 (V-013 is
/// emitted at the parse stage above).
fn validate_spec(
    cfg: &Config,
    dirname: &str,
    spec_path: &str,
    fm: &Frontmatter,
    all_ids: &std::collections::BTreeSet<String>,
    out: &mut Vec<Violation>,
) {
    let at = || Some(spec_path.to_string());

    // V-012: id pattern.
    if !valid_id(&fm.id) {
        out.push(error(
            "V-012",
            format!(
                "id '{}' does not match ^[0-9]{{3}}-[a-z0-9]+(-[a-z0-9]+)*$",
                fm.id
            ),
            at(),
        ));
    }
    // V-001: directory name equals id.
    if dirname != fm.id {
        out.push(error(
            "V-001",
            format!("directory '{dirname}' does not equal id '{}'", fm.id),
            at(),
        ));
    }
    // V-005: domain allowlist (only when configured non-empty).
    if let Some(domain) = &fm.domain {
        if !cfg.domains.permits(domain) {
            out.push(error(
                "V-005",
                format!("domain '{domain}' is not in domains.allowed"),
                at(),
            ));
        }
    }
    // V-006: kind allowlist (only when configured non-empty).
    if let Some(kind) = &fm.kind {
        if !cfg.kind.permits(kind) {
            out.push(error(
                "V-006",
                format!("kind '{kind}' is not in kind.allowed"),
                at(),
            ));
        }
    }
    // V-007: undeclared extra_frontmatter count cap.
    let undeclared = fm
        .extra_frontmatter
        .keys()
        .filter(|k| !cfg.frontmatter.extra_known_keys.contains(k))
        .count();
    if undeclared > MAX_UNDECLARED_EXTRA_FRONTMATTER {
        out.push(error(
            "V-007",
            format!(
                "{undeclared} undeclared extra-frontmatter keys exceed the cap of {MAX_UNDECLARED_EXTRA_FRONTMATTER} (declare them in frontmatter.extra_known_keys or model them)"
            ),
            at(),
        ));
    }
    // V-008 / V-009: lifecycle requirements.
    if fm.status == Status::Superseded {
        match &fm.superseded_by {
            Some(by) if all_ids.contains(by) => {}
            Some(by) => out.push(error(
                "V-008",
                format!("superseded_by '{by}' does not resolve to an existing spec"),
                at(),
            )),
            None => out.push(error(
                "V-008",
                "status is 'superseded' but superseded_by is missing".into(),
                at(),
            )),
        }
    }
    if fm.status == Status::Retired && fm.retirement_rationale.is_none() {
        out.push(error(
            "V-009",
            "status is 'retired' but retirement_rationale is missing".into(),
            at(),
        ));
    }
    // V-010 (warning): dangling depends_on.
    for dep in &fm.depends_on {
        if !all_ids.contains(dep) {
            out.push(warning(
                "V-010",
                format!("depends_on '{dep}' does not resolve to an existing spec"),
                at(),
            ));
        }
    }
}

/// V-003 (duplicate id) and V-004 (duplicate numeric prefix). `specs` is
/// `(id, spec_path)` pairs in discovery order.
fn detect_duplicates(specs: &[(String, String)], out: &mut Vec<Violation>) {
    let mut id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut prefix_owner: BTreeMap<&str, &str> = BTreeMap::new();
    for (id, spec_path) in specs {
        *id_counts.entry(id.as_str()).or_insert(0) += 1;
        let prefix = &id[..id.len().min(3)];
        match prefix_owner.get(prefix) {
            Some(other) if *other != id.as_str() => out.push(error(
                "V-004",
                format!("numeric prefix '{prefix}' is shared by '{other}' and '{id}'"),
                Some(spec_path.clone()),
            )),
            _ => {
                prefix_owner.entry(prefix).or_insert(id.as_str());
            }
        }
    }
    for (id, count) in id_counts {
        if count > 1 {
            out.push(error(
                "V-003",
                format!("duplicate spec id '{id}' ({count} specs)"),
                None,
            ));
        }
    }
}

/// Build a `SpecRecord` from parsed frontmatter, copying `extra_frontmatter`
/// verbatim so downstream-specific keys reach `registry.json` (the overlay seam).
fn build_record(fm: Frontmatter, spec_path: String, body: &str) -> SpecRecord {
    SpecRecord {
        id: fm.id,
        title: fm.title,
        status: fm.status,
        created: fm.created,
        summary: fm.summary,
        spec_path,
        authors: fm.authors,
        owner: fm.owner,
        kind: fm.kind,
        domain: fm.domain,
        risk: fm.risk,
        implementation: fm.implementation,
        depends_on: fm.depends_on,
        code_aliases: fm.code_aliases,
        feature_branch: fm.feature_branch,
        section_headings: markdown::section_headings(body),
        establishes: fm.establishes,
        extends: fm.extends,
        refines: fm.refines,
        supersedes: fm.supersedes,
        amends: fm.amends,
        co_authority: fm.co_authority,
        constrains: fm.constrains,
        references: fm.references,
        superseded_by: fm.superseded_by,
        retirement_rationale: fm.retirement_rationale,
        amends_sections: fm.amends_sections,
        unamendable: fm.unamendable,
        amendment_record: fm.amendment_record,
        origin: fm.origin,
        extra_frontmatter: fm.extra_frontmatter,
    }
}

/// Resolve a short spec reference (`109`) to the full id (`109-slug`) by its
/// leading numeric segment, when exactly one spec matches. An exact id, an
/// ambiguous prefix, or no match returns the input unchanged (so V-008/V-010
/// still fire on a genuinely dangling reference). Mirrors the indexer's
/// `resolve_id` (spec 004) so compile-time and index-time resolution agree;
/// kept local to avoid coupling the compile gate (001) to the indexer's file.
fn resolve_spec_ref(short: &str, all_ids: &std::collections::BTreeSet<String>) -> String {
    if all_ids.contains(short) {
        return short.to_string();
    }
    let matches: Vec<&String> = all_ids
        .iter()
        .filter(|id| id.split('-').next() == Some(short))
        .collect();
    match matches.as_slice() {
        [only] => (*only).clone(),
        _ => short.to_string(),
    }
}

// --- small helpers ---

fn error(code: &str, message: String, path: Option<String>) -> Violation {
    Violation {
        code: code.to_string(),
        severity: Severity::Error,
        message,
        path,
    }
}

fn warning(code: &str, message: String, path: Option<String>) -> Violation {
    Violation {
        code: code.to_string(),
        severity: Severity::Warning,
        message,
        path,
    }
}

/// Repo-relative POSIX path of `file` under `repo_root` (forward slashes).
fn rel_posix(repo_root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(repo_root).unwrap_or(file);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*$`, matched without a regex dependency.
fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() < 5 || !bytes[..3].iter().all(u8::is_ascii_digit) || bytes[3] != b'-' {
        return false;
    }
    let slug = &id[4..];
    if slug.is_empty() || slug.starts_with('-') || slug.ends_with('-') || slug.contains("--") {
        return false;
    }
    slug.bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::valid_id;

    #[test]
    fn id_pattern() {
        assert!(valid_id("000-spec-spine-bootstrap"));
        assert!(valid_id("042-x"));
        assert!(!valid_id("42-x"), "needs 3 digits");
        assert!(!valid_id("000-"), "needs a slug");
        assert!(!valid_id("000-Foo"), "lowercase only");
        assert!(!valid_id("000--x"), "no double hyphen");
        assert!(!valid_id("000-x-"), "no trailing hyphen");
    }
}
