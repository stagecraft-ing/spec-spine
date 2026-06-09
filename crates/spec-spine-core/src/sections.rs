//! Section anchor parsing (spec 004 §3.3). Given a file's content and name,
//! enumerate its named sections as `(anchor, LineSpan)`. Four dispatchers:
//! Makefile targets, Markdown heading slugs, `region:` markers, and CI
//! `jobs.<name>` blocks. Shared by the indexer (resolve a declared section unit)
//! and, later, the coupling gate (attribute a diff hunk to a section).
//!
//! Spans are inclusive 1-based lines, aligned with `git diff -U0` hunk ranges.

use spec_spine_types::LineSpan;

/// Enumerate every named section in `content`, dispatching on `file_name`.
/// Deterministic: returned in source order.
pub fn enumerate_sections(content: &str, file_name: &str) -> Vec<(String, LineSpan)> {
    let base = file_name.rsplit('/').next().unwrap_or(file_name);
    if base == "Makefile" || base == "makefile" || base.ends_with(".mk") {
        makefile_sections(content)
    } else if has_ext(base, &["md", "markdown"]) {
        markdown_sections(content)
    } else if has_ext(base, &["yml", "yaml"]) {
        ci_job_sections(content)
    } else {
        region_sections(content, comment_token(base))
    }
}

/// Resolve a single section anchor to its span, if present.
pub fn resolve_section(content: &str, file_name: &str, anchor: &str) -> Option<LineSpan> {
    enumerate_sections(content, file_name)
        .into_iter()
        .find(|(a, _)| a == anchor)
        .map(|(_, span)| span)
}

// ===== Markdown =====

fn markdown_sections(content: &str) -> Vec<(String, LineSpan)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut headings: Vec<(usize, String, usize)> = Vec::new(); // (level, slug, start_line)
    let mut in_fence = false;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let level = t.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&level) {
            let rest = &t[level..];
            if rest.starts_with(' ') || rest.starts_with('\t') {
                headings.push((level, slug(rest.trim()), i + 1));
            }
        }
    }
    let total = lines.len().max(1);
    let mut out = Vec::with_capacity(headings.len());
    for (idx, (level, anchor, start)) in headings.iter().enumerate() {
        let mut end = total;
        for (next_level, _, next_start) in &headings[idx + 1..] {
            if next_level <= level {
                end = next_start.saturating_sub(1);
                break;
            }
        }
        out.push((anchor.clone(), LineSpan::new(*start, end.max(*start))));
    }
    out
}

/// Kebab-case slug of a heading: lowercase, alnum kept, runs of other chars
/// collapse to a single `-`, trimmed.
fn slug(text: &str) -> String {
    let mut s = String::new();
    let mut prev_dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            s.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            s.push('-');
            prev_dash = true;
        }
    }
    s.trim_matches('-').to_string()
}

// ===== Makefile =====

fn makefile_sections(content: &str) -> Vec<(String, LineSpan)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<(String, LineSpan)> = Vec::new();
    let mut pending_tag: Option<String> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // `## tag: name` tags the next target.
        if let Some(rest) = trimmed.strip_prefix("## tag:") {
            pending_tag = Some(rest.trim().to_string());
            i += 1;
            continue;
        }
        // `# BEGIN name` ... `# END[ name]` explicit region.
        if let Some(rest) = trimmed.strip_prefix("# BEGIN ") {
            let name = rest.trim().to_string();
            let start = i + 1;
            let mut end = start;
            let mut j = i + 1;
            while j < lines.len() {
                let t = lines[j].trim_start();
                if t.starts_with("# END") {
                    end = j + 1;
                    break;
                }
                j += 1;
                end = j;
            }
            out.push((name, LineSpan::new(start, end)));
            i += 1;
            continue;
        }

        // A target: `name:` at column 0, not an assignment, not a dot-directive.
        if !line.starts_with([' ', '\t']) {
            if let Some(target) = target_name(line) {
                let start = i + 1;
                let mut end = start;
                let mut j = i + 1;
                // Recipe lines are tab-indented; consume them.
                while j < lines.len() && lines[j].starts_with('\t') {
                    end = j + 1;
                    j += 1;
                }
                out.push((target.clone(), LineSpan::new(start, end)));
                if let Some(tag) = pending_tag.take() {
                    out.push((tag, LineSpan::new(start, end)));
                }
            }
        }
        i += 1;
    }
    out
}

/// The target name in a Makefile target line, or `None` if it is an assignment,
/// a dot-directive, or not a target.
fn target_name(line: &str) -> Option<String> {
    let colon = line.find(':')?;
    let before = &line[..colon];
    let after = line.get(colon + 1..).unwrap_or("");
    // Assignments (`:=`) and `=` before `:` are not targets.
    if after.starts_with('=') || before.contains('=') {
        return None;
    }
    let name = before.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name.to_string())
}

// ===== region markers =====

fn region_sections(content: &str, token: &str) -> Vec<(String, LineSpan)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = Vec::new();
    let region_prefix = format!("{token} region:");
    let endregion = format!("{token} endregion");
    let region_prefix_tight = format!("{token}region:");
    let endregion_tight = format!("{token}endregion");

    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_start();
        let name = t
            .strip_prefix(&region_prefix)
            .or_else(|| t.strip_prefix(&region_prefix_tight));
        if let Some(name) = name {
            let name = name.trim().to_string();
            let start = i + 1;
            let mut end = lines.len();
            let mut j = i + 1;
            while j < lines.len() {
                let tj = lines[j].trim_start();
                if tj.starts_with(&endregion) || tj.starts_with(&endregion_tight) {
                    end = j + 1;
                    break;
                }
                j += 1;
            }
            out.push((name, LineSpan::new(start, end)));
        }
        i += 1;
    }
    out
}

fn comment_token(file_name: &str) -> &'static str {
    if has_ext(
        file_name,
        &[
            "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "go", "c", "cc", "cpp", "h", "hpp",
            "java",
        ],
    ) {
        "//"
    } else {
        "#"
    }
}

// ===== CI jobs (YAML) =====

fn ci_job_sections(content: &str) -> Vec<(String, LineSpan)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = Vec::new();

    // Find a top-level `jobs:` key.
    let Some(jobs_line) = lines
        .iter()
        .position(|l| l.trim_end() == "jobs:" && indent(l) == 0)
    else {
        return out;
    };

    // The first non-blank line after `jobs:` sets the job-key indent.
    let mut job_indent = None;
    for line in &lines[jobs_line + 1..] {
        if line.trim().is_empty() {
            continue;
        }
        if indent(line) == 0 {
            break; // dedented out of jobs without any job
        }
        job_indent = Some(indent(line));
        break;
    }
    let Some(job_indent) = job_indent else {
        return out;
    };

    let mut current: Option<(String, usize)> = None; // (name, start_line)
    let mut i = jobs_line + 1;
    while i < lines.len() {
        let line = lines[i];
        let is_blank = line.trim().is_empty();
        if !is_blank && indent(line) == 0 {
            break; // left the jobs block
        }
        if !is_blank && indent(line) == job_indent {
            if let Some(name) = yaml_key(line) {
                if let Some((prev, start)) = current.take() {
                    out.push((prev, LineSpan::new(start, i))); // end before this job
                }
                current = Some((name, i + 1));
            }
        }
        i += 1;
    }
    if let Some((name, start)) = current {
        out.push((name, LineSpan::new(start, lines.len())));
    }
    out
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn yaml_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let colon = trimmed.find(':')?;
    let key = trimmed[..colon].trim();
    if key.is_empty() || key.starts_with('#') {
        return None;
    }
    Some(key.to_string())
}

fn has_ext(name: &str, exts: &[&str]) -> bool {
    name.rsplit('.')
        .next()
        .map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_heading_spans() {
        let md = "# Top\nintro\n## A\naaa\n## B\nbbb\n### B1\nccc\n";
        let s = enumerate_sections(md, "README.md");
        let by = |name: &str| s.iter().find(|(a, _)| a == name).map(|(_, sp)| *sp);
        assert_eq!(by("top"), Some(LineSpan::new(1, 8)));
        assert_eq!(by("a"), Some(LineSpan::new(3, 4)));
        // B runs through its subheading B1 (B1 is deeper, so included).
        assert_eq!(by("b"), Some(LineSpan::new(5, 8)));
        assert_eq!(by("b1"), Some(LineSpan::new(7, 8)));
    }

    #[test]
    fn markdown_skips_code_fences() {
        let md = "# Real\n```\n# fake heading\n```\ntext\n";
        let names: Vec<String> = enumerate_sections(md, "x.md")
            .into_iter()
            .map(|(a, _)| a)
            .collect();
        assert_eq!(names, vec!["real"]);
    }

    #[test]
    fn makefile_targets_and_tags() {
        let mk = "## tag: deps\ninstall:\n\tnpm ci\n\tcargo fetch\n\nbuild:\n\tcargo build\n";
        let s = enumerate_sections(mk, "Makefile");
        let by = |name: &str| s.iter().find(|(a, _)| a == name).map(|(_, sp)| *sp);
        assert_eq!(by("install"), Some(LineSpan::new(2, 4)));
        assert_eq!(by("deps"), Some(LineSpan::new(2, 4))); // tag aliases the install target
        assert_eq!(by("build"), Some(LineSpan::new(6, 7)));
    }

    #[test]
    fn region_markers() {
        let rs = "fn a() {}\n// region: core\nfn b() {}\nfn c() {}\n// endregion\nfn d() {}\n";
        assert_eq!(
            resolve_section(rs, "lib.rs", "core"),
            Some(LineSpan::new(2, 5))
        );
    }

    #[test]
    fn ci_job_blocks() {
        let yml = "name: CI\non: push\njobs:\n  build:\n    runs-on: ubuntu\n    steps: []\n  test:\n    runs-on: ubuntu\n";
        let s = enumerate_sections(yml, ".github/workflows/ci.yml");
        let by = |name: &str| s.iter().find(|(a, _)| a == name).map(|(_, sp)| *sp);
        assert_eq!(by("build"), Some(LineSpan::new(4, 6)));
        assert_eq!(by("test"), Some(LineSpan::new(7, 8)));
    }
}
