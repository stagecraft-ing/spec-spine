//! `spec-spine couple` — the PR-time coupling gate (spec 005).
//!
//! This is the only place `git` runs: it invokes `git diff --no-color -U0
//! base...head`, parses the unified diff into a typed [`DiffInput`] (new-side
//! hunk spans), reads the PR body for a waiver, and calls the pure
//! `spec-spine-core` gate. Diff parsing is ported from OAP
//! `spec-code-coupling-check/src/main.rs` (`parse_unified_diff`,
//! `parse_hunk_header`).

use std::path::{Path, PathBuf};
use std::process::Command;

use spec_spine_core::{
    DiffFile, DiffInput, FileContents, couple, dependency_only_waiver, is_bypassed_path,
    load_index, parse_waiver,
};
use spec_spine_types::{Config, Error, LineSpan};

use crate::load_repo_config;

/// Arguments for `spec-spine couple`.
pub struct CoupleArgs {
    pub base: String,
    pub head: String,
    pub pr_body: Option<PathBuf>,
    pub paths_from: Option<PathBuf>,
}

pub fn run(repo: &Path, args: &CoupleArgs) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;

    let diff = build_diff_input(repo, args)?;
    let body = read_pr_body(args)?;
    let mut waiver = parse_waiver(&cfg, &body);
    let mut auto_waived = false;

    // Spec 005 §3.5 — mechanical dependency-only auto-waiver. Opt-in,
    // git-diff mode only (`--paths-from` has no content to compare), and
    // never overrides an explicit PR-body waiver.
    if waiver.is_none() && cfg.coupling.auto_waive_dependency_only && args.paths_from.is_none() {
        waiver = try_dependency_only_waiver(repo, &cfg, args, &diff)?;
        auto_waived = waiver.is_some();
    }

    let report = couple(&cfg, repo, &diff, waiver.as_ref())?;

    if report.has_blocking_drift() {
        eprintln!(
            "spec-spine couple: {} drift violation(s) — a changed path lacks an authoring edit to an owning spec.\n",
            report.violations.len()
        );
        for v in &report.violations {
            eprintln!("  {}", v.message);
        }
        eprintln!(
            "\nResolve by editing an owning spec's spec.md, or add a '{}' line to the PR body.",
            cfg.coupling.waiver_keyword
        );
    } else if let Some(reason) = &report.waiver {
        println!(
            "spec-spine couple: {} violation(s) {} — reason: {reason}",
            report.violations.len(),
            if auto_waived { "auto-waived" } else { "waived" }
        );
        for v in &report.violations {
            println!("  {} (waived)", v.message);
        }
    } else {
        println!(
            "spec-spine couple: OK — {} path(s) checked, no drift.",
            report.checked_paths
        );
    }

    Ok(report.exit_code())
}

/// Build the [`DiffInput`]: either from `--paths-from` (whole-file fallback, no
/// hunks) or from `git diff --no-color -U0 base...head`.
fn build_diff_input(repo: &Path, args: &CoupleArgs) -> Result<DiffInput, Error> {
    if let Some(path) = &args.paths_from {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Io(format!("read --paths-from {}: {e}", path.display())))?;
        let files = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|p| DiffFile {
                path: p.to_string(),
                hunks: Vec::new(),
            })
            .collect();
        return Ok(DiffInput { files });
    }

    let raw = run_git_diff(repo, &args.base, &args.head)?;
    Ok(parse_unified_diff(&raw))
}

/// Attempt the spec 005 §3.5 mechanical auto-waiver: every non-bypassed
/// changed path must be a `package.json` whose base→head change is
/// dependency-only. Contents come from `git show` at the **merge base** (the
/// diff is three-dot, so the base side is `merge-base(base, head)`, not the
/// base branch tip) and at `head`. Any git failure refuses the auto-waiver
/// fail-closed rather than erroring the gate.
///
/// The bypass verdict is claim-aware (spec 009): it reads the committed
/// index so a claim-overridden floor path counts as a candidate and refuses
/// the waiver, matching exactly the path set the gate evaluates. An
/// unreadable index refuses fail-closed (the gate itself will report the
/// real error).
fn try_dependency_only_waiver(
    repo: &Path,
    cfg: &Config,
    args: &CoupleArgs,
    diff: &DiffInput,
) -> Result<Option<spec_spine_core::Waiver>, Error> {
    let index_path = repo
        .join(&cfg.layout.derived_dir)
        .join("codebase-index")
        .join("index.json");
    let Some(index) = std::fs::read(&index_path)
        .ok()
        .and_then(|bytes| load_index(&bytes).ok())
    else {
        return Ok(None);
    };
    let candidates: Vec<&DiffFile> = diff
        .files
        .iter()
        .filter(|f| !is_bypassed_path(cfg, &index, &f.path))
        .collect();
    // Cheap pre-filter before any git spawn: the waiver can only ever apply
    // when every non-bypassed path is a package.json manifest.
    if candidates.is_empty()
        || !candidates
            .iter()
            .all(|f| spec_spine_core::is_package_json(&f.path))
    {
        return Ok(None);
    }

    let Some(merge_base) = git_merge_base(repo, &args.base, &args.head) else {
        return Ok(None);
    };

    let mut files: Vec<FileContents> = Vec::with_capacity(candidates.len());
    for f in &candidates {
        files.push(FileContents {
            path: f.path.clone(),
            base: git_show(repo, &merge_base, &f.path),
            head: git_show(repo, &args.head, &f.path),
        });
    }
    Ok(dependency_only_waiver(&files))
}

fn git_merge_base(repo: &Path, base: &str, head: &str) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["merge-base", base, head])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let rev = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if rev.is_empty() { None } else { Some(rev) }
}

fn git_show(repo: &Path, rev: &str, path: &str) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", &format!("{rev}:{path}")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None; // absent at this rev (created/deleted) — fail closed upstream
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_git_diff(repo: &Path, base: &str, head: &str) -> Result<String, Error> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--no-color", "-U0"])
        .arg(format!("{base}...{head}"))
        .output()
        .map_err(|e| Error::Io(format!("spawn git diff: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(Error::Io(format!(
            "git diff exited {:?}: {stderr}",
            out.status.code()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse `git diff --no-color -U0` output into a [`DiffInput`]. New-side hunk
/// ranges become inclusive [`LineSpan`]s; a deleted file (`+++ /dev/null`) is
/// registered with no hunks (a whole-file change).
fn parse_unified_diff(diff_text: &str) -> DiffInput {
    use std::collections::BTreeMap;
    let mut files: BTreeMap<String, Vec<LineSpan>> = BTreeMap::new();
    let mut current_path: Option<String> = None;
    let mut minus_path: Option<String> = None;

    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("--- ") {
            minus_path = strip_diff_prefix(rest.trim());
        } else if let Some(rest) = line.strip_prefix("+++ ") {
            let p = rest.trim();
            if p == "/dev/null" {
                // Deletion: the changed path is the old (minus) side, whole-file.
                current_path = minus_path.clone();
            } else {
                current_path = strip_diff_prefix(p);
            }
            if let Some(path) = &current_path {
                files.entry(path.clone()).or_default();
            }
        } else if line.starts_with("@@") {
            if let Some(path) = &current_path {
                if let Some(span) = parse_hunk_header(line) {
                    files.entry(path.clone()).or_default().push(span);
                }
            }
        }
    }

    DiffInput {
        files: files
            .into_iter()
            .map(|(path, hunks)| DiffFile { path, hunks })
            .collect(),
    }
}

/// `a/<path>` / `b/<path>` → `<path>`; `/dev/null` → `None`.
fn strip_diff_prefix(p: &str) -> Option<String> {
    if p == "/dev/null" {
        return None;
    }
    Some(
        p.strip_prefix("a/")
            .or_else(|| p.strip_prefix("b/"))
            .unwrap_or(p)
            .to_string(),
    )
}

/// Parse `@@ -<old> +<new> @@` into the inclusive new-side span. A pure-deletion
/// hunk (`+start,0`) collapses to the single line at `start`.
fn parse_hunk_header(line: &str) -> Option<LineSpan> {
    let after_at = line.strip_prefix("@@")?.trim_start();
    let rest = after_at.strip_prefix('-')?;
    let plus_pos = rest.find('+')?;
    let new_part = rest[plus_pos + 1..].trim_start();
    let new_range = new_part.split_whitespace().next()?;
    let (start_s, count_s) = match new_range.split_once(',') {
        Some((a, b)) => (a, b),
        None => (new_range, "1"),
    };
    let start: usize = start_s.parse().ok()?;
    let count: usize = count_s.parse().ok()?;
    if start == 0 {
        // `+0,0` — a deletion with no new-side line. Whole-file fallback handles
        // the path; emit nothing for this hunk.
        return None;
    }
    let count = count.max(1);
    Some(LineSpan::new(start, start + count - 1))
}

fn read_pr_body(args: &CoupleArgs) -> Result<String, Error> {
    if let Some(path) = &args.pr_body {
        std::fs::read_to_string(path)
            .map_err(|e| Error::Io(format!("read --pr-body {}: {e}", path.display())))
    } else if let Ok(s) = std::env::var("SPEC_SPINE_PR_BODY") {
        Ok(s)
    } else {
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modification_hunks_to_inclusive_spans() {
        let diff = "diff --git a/Makefile b/Makefile\n\
                    --- a/Makefile\n\
                    +++ b/Makefile\n\
                    @@ -10,2 +10,3 @@ ctx\n\
                    @@ -50 +51,5 @@\n";
        let d = parse_unified_diff(diff);
        let f = d.files.iter().find(|f| f.path == "Makefile").unwrap();
        assert_eq!(f.hunks, vec![LineSpan::new(10, 12), LineSpan::new(51, 55)]);
    }

    #[test]
    fn deleted_file_is_whole_file_change() {
        let diff = "diff --git a/gone.rs b/gone.rs\n\
                    deleted file mode 100644\n\
                    --- a/gone.rs\n\
                    +++ /dev/null\n\
                    @@ -1,5 +0,0 @@\n";
        let d = parse_unified_diff(diff);
        let f = d.files.iter().find(|f| f.path == "gone.rs").unwrap();
        assert!(f.hunks.is_empty(), "deletion ⇒ whole-file (no hunks)");
    }
}
