//! The render capability (spec 011): deterministic, human-shaped projections
//! of the **committed** `index.json`. Pure read-side: never recomputes the
//! index, never consults the working tree, never signals staleness
//! (recomputation is `index`, freshness is `index check`; three verbs, three
//! jobs). Output is a pure function of `(config, index)`: byte-identical
//! across platforms, LF line endings, trailing newline.

use std::fmt::Write as _;

use spec_spine_types::{CodebaseIndex, Config, Diagnostic, PackageKind};

/// The id-sorted `traceability.orphanedSpecs` list (spec 011 §3.3).
pub fn orphans(index: &CodebaseIndex) -> Vec<&str> {
    let mut ids: Vec<&str> = index
        .traceability
        .orphaned_specs
        .iter()
        .map(String::as_str)
        .collect();
    ids.sort_unstable();
    ids
}

/// The markdown projection of the committed index (spec 011 §3.2).
///
/// Section inventory and order are the v1 contract: header, package
/// inventory, traceability (orphans / untraced flat lists omitted when
/// empty), diagnostics (omitted when empty). The prose between sections is
/// not contractual.
pub fn render_markdown(config: &Config, index: &CodebaseIndex) -> String {
    let mut out = String::new();

    // 1. Header, traceable to the exact artifact that produced it.
    let _ = writeln!(out, "# {} codebase index", config.branding.indexer_id);
    out.push('\n');
    let _ = writeln!(out, "- schemaVersion: {}", index.schema_version);
    let _ = writeln!(out, "- contentHash: {}", index.build.content_hash);

    // 2. Package inventory, sorted by name, ties by path.
    out.push_str("\n## Packages\n\n");
    let mut packages: Vec<_> = index.packages.iter().collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
    out.push_str("| name | path | kind | version | spec |\n");
    out.push_str("|---|---|---|---|---|\n");
    for p in packages {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            p.name,
            p.path,
            kind_label(p.kind),
            p.version.as_deref().unwrap_or("-"),
            p.spec_ref.as_deref().unwrap_or("-"),
        );
    }

    // 3. Traceability: per-spec summary, then the flat lists (omitted when
    //    empty).
    out.push_str("\n## Traceability\n\n");
    let mut mappings: Vec<_> = index.traceability.mappings.iter().collect();
    mappings.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));
    out.push_str("| spec | status | paths | units |\n");
    out.push_str("|---|---|---|---|\n");
    for m in mappings {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            m.spec_id,
            m.spec_status.as_deref().unwrap_or("-"),
            m.implementing_paths.len(),
            m.resolved_units.len(),
        );
    }
    if !index.traceability.orphaned_specs.is_empty() {
        out.push_str("\n### Orphaned specs\n\n");
        for id in orphans(index) {
            let _ = writeln!(out, "- {id}");
        }
    }
    if !index.traceability.untraced_code.is_empty() {
        out.push_str("\n### Untraced code\n\n");
        let mut paths: Vec<&str> = index
            .traceability
            .untraced_code
            .iter()
            .map(String::as_str)
            .collect();
        paths.sort_unstable();
        for path in paths {
            let _ = writeln!(out, "- {path}");
        }
    }

    // 4. Diagnostics, sorted by (code, file); omitted when empty.
    let mut diagnostics: Vec<(&Diagnostic, &str)> = index
        .diagnostics
        .errors
        .iter()
        .map(|d| (d, "error"))
        .chain(index.diagnostics.warnings.iter().map(|d| (d, "warning")))
        .collect();
    if !diagnostics.is_empty() {
        diagnostics.sort_by_key(|(d, _)| (d.code.as_str(), d.path.as_deref().unwrap_or("")));
        out.push_str("\n## Diagnostics\n\n");
        for (d, severity) in diagnostics {
            match &d.path {
                Some(path) => {
                    let _ = writeln!(out, "- {} [{severity}] {} ({path})", d.code, d.message);
                }
                None => {
                    let _ = writeln!(out, "- {} [{severity}] {}", d.code, d.message);
                }
            }
        }
    }

    out
}

/// The kebab-case kind label, matching the serde wire form in `index.json`.
fn kind_label(kind: PackageKind) -> &'static str {
    match kind {
        PackageKind::RustLib => "rust-lib",
        PackageKind::RustBin => "rust-bin",
        PackageKind::RustLibBin => "rust-lib-bin",
        PackageKind::NpmPackage => "npm-package",
        PackageKind::NpmWorkspace => "npm-workspace",
    }
}
