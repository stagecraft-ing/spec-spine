//! Section anchor parsing (spec 004 §3.3, widened by spec 022). Given a file's
//! content and name, enumerate its named sections as `(anchor, LineSpan)`.
//! Dispatchers: Makefile targets, Markdown heading slugs, `region:` markers, CI
//! `jobs.<name>` blocks, and the spec-022 bounded keypath grammar for the three
//! first-party structured-config shapes (workflow YAML, `Cargo.toml`,
//! `package.json`). Shared by the indexer (resolve a declared section unit) and,
//! later, the coupling gate (attribute a diff hunk to a section).
//!
//! Spec 022 widens which `anchor` strings resolve on the eligible structured
//! files only: a section anchor may be a dotted mapping/table keypath
//! (`permissions`, `on.merge_group`, `jobs.build.permissions`,
//! `workspace.package`, `scripts`). Eligibility is a hard predicate (§3.2): a
//! keypath resolves only on a `.github/workflows/*.yml`, a `Cargo.toml`, or a
//! `package.json`; foreign structured configs (`deny.toml`, Helm `values.yaml`)
//! keep whole-file / `region:` ownership and never receive keypath treatment, so
//! the gate never binds to a third-party schema spec-spine does not own.
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
    } else if base == "Cargo.toml" {
        // spec 022: first-party manifest -> table keypaths.
        cargo_toml_sections(content)
    } else if base == "package.json" {
        // spec 022: first-party manifest -> member keypaths (no region fallback:
        // JSON has no comment syntax).
        package_json_sections(content)
    } else if has_ext(base, &["yml", "yaml"]) {
        if is_workflow_path(file_name) {
            // spec 022: governed workflow -> keypath grammar (a strict superset
            // of the legacy bare-`jobs.<name>` behavior).
            workflow_yaml_sections(content)
        } else {
            // Foreign YAML keeps the legacy bare-job behavior; routing it to
            // region markers is spec 022 D4, deliberately deferred (§4).
            ci_job_sections(content)
        }
    } else {
        region_sections(content, comment_token(base))
    }
}

/// True if `path` is a governed workflow file: directly under `.github/workflows/`
/// and ending `.yml` / `.yaml` (spec 022 §3.2). The path is repo-relative POSIX
/// (the `Unit::Section.file` value), so a leading `./` is tolerated.
fn is_workflow_path(path: &str) -> bool {
    const DIR: &str = ".github/workflows/";
    let p = path.strip_prefix("./").unwrap_or(path);
    let Some(rest) = p.strip_prefix(DIR) else {
        return false;
    };
    // Directly in the directory (GitHub does not read nested workflow dirs).
    !rest.contains('/') && has_ext(rest, &["yml", "yaml"])
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

// ===== spec 022: workflow YAML keypaths =====

/// Bounded keypath enumeration for a governed workflow (spec 022 §3.3). Yields,
/// by an indentation-aware structural scan that generalizes [`ci_job_sections`]:
/// every top-level key (`on`, `permissions`, `env`, `jobs`, ...); every
/// second-level key as a dotted `parent.child` anchor (`on.merge_group`,
/// `jobs.build`); third-level keys *under a job only* (`jobs.<name>.<key>`, e.g.
/// `jobs.build.permissions`); and a bare `<name>` alias for each job
/// (back-compat). Max depth 3; no sequence indexing, no wildcards. A block's
/// span runs from its key line to the last line before the next key at the same
/// or shallower indent.
fn workflow_yaml_sections(content: &str) -> Vec<(String, LineSpan)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<(String, LineSpan)> = Vec::new();
    // Ancestor stack of (indent, key) giving the current mapping path.
    let mut stack: Vec<(usize, String)> = Vec::new();

    for (i, raw) in lines.iter().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let t = raw.trim_start();
        // Comments and sequence/flow items are not mapping keys.
        if t.starts_with(['#', '-', '{', '[', '|', '>']) {
            continue;
        }
        let Some(key) = mapping_key(t) else {
            continue;
        };
        let ind = indent(raw);
        while matches!(stack.last(), Some(&(pi, _)) if pi >= ind) {
            stack.pop();
        }
        let depth = stack.len() + 1;
        let top = stack.first().map(|(_, k)| k.as_str());
        let span = block_span(&lines, i, ind);
        match depth {
            1 => out.push((key.clone(), span)),
            2 => {
                let path = format!("{}.{}", stack[0].1, key);
                out.push((path, span));
                if top == Some("jobs") {
                    out.push((key.clone(), span)); // bare job alias (back-compat)
                }
            }
            3 if top == Some("jobs") => {
                out.push((format!("{}.{}.{}", stack[0].1, stack[1].1, key), span));
            }
            _ => {} // deeper / non-job depth-3: not enumerated (bounded)
        }
        stack.push((ind, key));
    }
    out
}

/// The span of the block opened by the key at 0-based line `ki` with indent
/// `key_indent`: from the key line through the last line before the next
/// non-blank line at the same-or-shallower indent (or EOF).
fn block_span(lines: &[&str], ki: usize, key_indent: usize) -> LineSpan {
    let start = ki + 1; // 1-based
    let mut end = lines.len();
    for (offset, line) in lines.iter().enumerate().skip(ki + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if indent(line) <= key_indent {
            end = offset; // last included 1-based line is `offset` (line before this)
            break;
        }
    }
    LineSpan::new(start, end.max(start))
}

/// The mapping key in a `key:` line (already left-trimmed), with surrounding
/// quotes stripped. `None` if there is no `:` or the key is empty.
fn mapping_key(trimmed: &str) -> Option<String> {
    let colon = trimmed.find(':')?;
    let key = trimmed[..colon]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    if key.is_empty() {
        return None;
    }
    Some(key.to_string())
}

// ===== spec 022: Cargo.toml table keypaths =====

/// Bounded table-keypath enumeration for a `Cargo.toml` (spec 022 §3.3). Uses
/// `toml_edit`'s span-aware immutable document so multi-line strings, dotted
/// keys, and inline tables are handled correctly (where a hand-rolled scanner
/// would mis-detect a `[header]` inside a multi-line string). Each table
/// resolves to the span from its header line through the last line before the
/// next sibling-or-shallower table (subtree-inclusive). Max depth 4 (covers
/// `package.metadata.<tool>`); array-of-tables are not addressable (no array
/// indexing). A malformed manifest yields no sections (the anchor falls to I-006).
fn cargo_toml_sections(content: &str) -> Vec<(String, LineSpan)> {
    const MAX_DEPTH: usize = 4;
    let Ok(doc) = toml_edit::ImDocument::parse(content) else {
        return Vec::new();
    };
    let mut tables: Vec<(Vec<String>, usize)> = Vec::new(); // (path, start byte)
    let mut leaves: Vec<(Vec<String>, std::ops::Range<usize>)> = Vec::new();
    collect_toml(
        doc.as_table(),
        &mut Vec::new(),
        &mut tables,
        &mut leaves,
        MAX_DEPTH,
    );

    // Table-start positions (depth, start byte), for the next-sibling boundary.
    let mut starts: Vec<(usize, usize)> = tables.iter().map(|(p, s)| (p.len(), *s)).collect();
    starts.sort_unstable_by_key(|&(_, s)| s);
    let total = content.lines().count().max(1);

    let mut out: Vec<(String, LineSpan)> = Vec::with_capacity(tables.len() + leaves.len());
    for (path, start) in &tables {
        let depth = path.len();
        let next = starts
            .iter()
            .filter(|&&(d, s)| s > *start && d <= depth)
            .map(|&(_, s)| s)
            .min();
        let start_line = byte_to_line(content, *start);
        let end_line = match next {
            Some(nb) => byte_to_line(content, nb).saturating_sub(1).max(start_line),
            None => total.max(start_line),
        };
        out.push((path.join("."), LineSpan::new(start_line, end_line)));
    }
    for (path, span) in &leaves {
        let start_line = byte_to_line(content, span.start);
        let end_line = byte_to_line(content, span.end.saturating_sub(1)).max(start_line);
        out.push((path.join("."), LineSpan::new(start_line, end_line)));
    }
    out.sort_by(|a, b| {
        (a.1.start_line, a.1.end_line, &a.0).cmp(&(b.1.start_line, b.1.end_line, &b.0))
    });
    out
}

/// Walk `table`, recording every sub-table (path, start byte) up to `max_depth`
/// and every top-level leaf value. Returns the smallest start byte found in the
/// subtree, used to synthesize a start for an implicit table (one created only
/// by a deeper header, which carries no span of its own).
fn collect_toml(
    table: &toml_edit::Table,
    prefix: &mut Vec<String>,
    tables: &mut Vec<(Vec<String>, usize)>,
    leaves: &mut Vec<(Vec<String>, std::ops::Range<usize>)>,
    max_depth: usize,
) -> Option<usize> {
    let mut min_start: Option<usize> = None;
    for (key, item) in table.iter() {
        prefix.push(key.to_string());
        let depth = prefix.len();
        let own_span = item.span();
        if let Some(sub) = item.as_table() {
            let child_min = collect_toml(sub, prefix, tables, leaves, max_depth);
            let start = own_span.as_ref().map(|s| s.start).or(child_min);
            if let Some(st) = start {
                if depth <= max_depth {
                    tables.push((prefix.clone(), st));
                }
                min_start = Some(min_start.map_or(st, |m: usize| m.min(st)));
            }
        } else if item.is_value() {
            if let Some(s) = own_span {
                if depth == 1 {
                    leaves.push((prefix.clone(), s.clone()));
                }
                min_start = Some(min_start.map_or(s.start, |m: usize| m.min(s.start)));
            }
        }
        // Array-of-tables: not addressable by keypath (no array indexing).
        prefix.pop();
    }
    min_start
}

// ===== spec 022: package.json member keypaths =====

/// Bounded member-keypath enumeration for a `package.json` (spec 022 §3.3). A
/// brace-depth, string-aware line scan: each object member resolves to the span
/// from its `"key":` line to the line where its value ends (its matching close
/// for an object/array, its own line for a scalar). Max depth 2 (top level plus
/// one nest, e.g. `scripts`, `dependencies`, `scripts.test`); members reached
/// through an array are not addressable (no array indexing).
fn package_json_sections(content: &str) -> Vec<(String, LineSpan)> {
    const MAX_DEPTH: usize = 2;
    struct Member {
        key: String,
        start_line: usize,
        value_end_line: Option<usize>,
    }
    struct Obj {
        path: Vec<String>,
        expecting_value: bool,
        member: Option<Member>,
    }
    enum Frame {
        Obj(Obj),
        Arr,
    }

    let bytes = content.as_bytes();
    let mut out: Vec<(String, LineSpan)> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut line: usize = 1;
    let mut array_depth: usize = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'"' => {
                let start_line = line;
                let mut j = i + 1;
                let mut s = String::new();
                let mut esc = false;
                while j < bytes.len() {
                    let cj = bytes[j];
                    if esc {
                        s.push(cj as char);
                        esc = false;
                    } else if cj == b'\\' {
                        esc = true;
                    } else if cj == b'"' {
                        break;
                    } else {
                        if cj == b'\n' {
                            line += 1;
                        }
                        s.push(cj as char);
                    }
                    j += 1;
                }
                let end_line = line;
                i = j + 1;
                if let Some(Frame::Obj(o)) = stack.last_mut() {
                    if o.expecting_value {
                        if let Some(m) = o.member.as_mut() {
                            m.value_end_line = Some(end_line);
                        }
                    } else {
                        o.member = Some(Member {
                            key: s,
                            start_line,
                            value_end_line: None,
                        });
                    }
                }
            }
            b':' => {
                if let Some(Frame::Obj(o)) = stack.last_mut() {
                    o.expecting_value = true;
                }
                i += 1;
            }
            b',' => {
                if let Some(Frame::Obj(o)) = stack.last_mut() {
                    if let Some(m) = o.member.take() {
                        if array_depth == 0 {
                            if let Some(end) = m.value_end_line {
                                push_member(
                                    &mut out,
                                    &o.path,
                                    &m.key,
                                    m.start_line,
                                    end,
                                    MAX_DEPTH,
                                );
                            }
                        }
                    }
                    o.expecting_value = false;
                }
                i += 1;
            }
            b'{' => {
                let path = match stack.last() {
                    Some(Frame::Obj(o)) if o.expecting_value => {
                        let mut p = o.path.clone();
                        if let Some(m) = &o.member {
                            p.push(m.key.clone());
                        }
                        p
                    }
                    _ => Vec::new(),
                };
                stack.push(Frame::Obj(Obj {
                    path,
                    expecting_value: false,
                    member: None,
                }));
                i += 1;
            }
            b'[' => {
                array_depth += 1;
                stack.push(Frame::Arr);
                i += 1;
            }
            b'}' => {
                if let Some(Frame::Obj(o)) = stack.last_mut() {
                    if let Some(m) = o.member.take() {
                        if array_depth == 0 {
                            if let Some(end) = m.value_end_line {
                                push_member(
                                    &mut out,
                                    &o.path,
                                    &m.key,
                                    m.start_line,
                                    end,
                                    MAX_DEPTH,
                                );
                            }
                        }
                    }
                }
                stack.pop();
                // The popped object was the value of the parent's member.
                if let Some(Frame::Obj(po)) = stack.last_mut() {
                    if po.expecting_value {
                        if let Some(m) = po.member.take() {
                            if array_depth == 0 {
                                push_member(
                                    &mut out,
                                    &po.path,
                                    &m.key,
                                    m.start_line,
                                    line,
                                    MAX_DEPTH,
                                );
                            }
                        }
                        po.expecting_value = false;
                    }
                }
                i += 1;
            }
            b']' => {
                array_depth = array_depth.saturating_sub(1);
                stack.pop();
                // The popped array was the value of the parent's member.
                if let Some(Frame::Obj(po)) = stack.last_mut() {
                    if po.expecting_value {
                        if let Some(m) = po.member.take() {
                            if array_depth == 0 {
                                push_member(
                                    &mut out,
                                    &po.path,
                                    &m.key,
                                    m.start_line,
                                    line,
                                    MAX_DEPTH,
                                );
                            }
                        }
                        po.expecting_value = false;
                    }
                }
                i += 1;
            }
            c => {
                // A scalar value token (number / true / false / null) under a key.
                if !c.is_ascii_whitespace() {
                    if let Some(Frame::Obj(o)) = stack.last_mut() {
                        if o.expecting_value {
                            if let Some(m) = o.member.as_mut() {
                                m.value_end_line = Some(line);
                            }
                        }
                    }
                }
                i += 1;
            }
        }
    }
    out.sort_by(|a, b| {
        (a.1.start_line, a.1.end_line, &a.0).cmp(&(b.1.start_line, b.1.end_line, &b.0))
    });
    out
}

/// Record a resolved member keypath if it is within the depth bound.
fn push_member(
    out: &mut Vec<(String, LineSpan)>,
    path: &[String],
    key: &str,
    start: usize,
    end: usize,
    max_depth: usize,
) {
    if path.len() < max_depth {
        let mut full = path.to_vec();
        full.push(key.to_string());
        out.push((full.join("."), LineSpan::new(start, end.max(start))));
    }
}

/// 1-based line number containing `byte` (the count of newlines before it + 1).
fn byte_to_line(content: &str, byte: usize) -> usize {
    let b = byte.min(content.len());
    content.as_bytes()[..b]
        .iter()
        .filter(|&&c| c == b'\n')
        .count()
        + 1
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

    // ===== spec 022: keypath section anchors =====

    const WORKFLOW: &str = "\
name: CI
on:
  push:
  merge_group:
permissions:
  contents: read
env:
  FOO: bar
jobs:
  build:
    permissions:
      contents: write
    runs-on: ubuntu
    steps:
      - run: echo hi
  test:
    runs-on: ubuntu
";

    fn ws_span(content: &str, file: &str, anchor: &str) -> Option<LineSpan> {
        resolve_section(content, file, anchor)
    }

    #[test]
    fn workflow_keypaths_and_back_compat() {
        let f = ".github/workflows/ci.yml";
        // Top-level keys.
        assert_eq!(ws_span(WORKFLOW, f, "on"), Some(LineSpan::new(2, 4)));
        assert_eq!(
            ws_span(WORKFLOW, f, "permissions"),
            Some(LineSpan::new(5, 6))
        );
        assert_eq!(ws_span(WORKFLOW, f, "env"), Some(LineSpan::new(7, 8)));
        // Second-level dotted keys (the merge-queue / trigger contract).
        assert_eq!(
            ws_span(WORKFLOW, f, "on.merge_group"),
            Some(LineSpan::new(4, 4))
        );
        assert_eq!(ws_span(WORKFLOW, f, "on.push"), Some(LineSpan::new(3, 3)));
        // Bare job alias (back-compat) and qualified job both resolve identically.
        assert_eq!(ws_span(WORKFLOW, f, "build"), Some(LineSpan::new(10, 15)));
        assert_eq!(
            ws_span(WORKFLOW, f, "jobs.build"),
            Some(LineSpan::new(10, 15))
        );
        // Per-job security boundary (depth 3 under a job).
        assert_eq!(
            ws_span(WORKFLOW, f, "jobs.build.permissions"),
            Some(LineSpan::new(11, 12))
        );
    }

    #[test]
    fn workflow_bounds_reject_index_and_overdepth() {
        let f = ".github/workflows/ci.yml";
        // Sequence index does not resolve (no array indexing).
        assert_eq!(ws_span(WORKFLOW, f, "jobs.build.steps.0"), None);
        // Over-deep keypath does not resolve (max depth 3).
        assert_eq!(ws_span(WORKFLOW, f, "jobs.build.steps.run"), None);
        // Depth-3 under a non-job top-level key is not enumerated.
        assert_eq!(ws_span(WORKFLOW, f, "on.push.branches"), None);
    }

    const CARGO: &str = "\
[workspace]
resolver = \"2\"
members = [\"a\"]

[workspace.package]
version = \"0.3.0\"

[dependencies]
serde = \"1\"

[package.metadata.oap]
spec = \"001\"
";

    #[test]
    fn cargo_toml_table_keypaths() {
        let f = "Cargo.toml";
        // Subtree-inclusive: workspace runs through workspace.package to the line
        // before [dependencies].
        assert_eq!(ws_span(CARGO, f, "workspace"), Some(LineSpan::new(1, 7)));
        assert_eq!(
            ws_span(CARGO, f, "workspace.package"),
            Some(LineSpan::new(5, 7))
        );
        assert_eq!(
            ws_span(CARGO, f, "dependencies"),
            Some(LineSpan::new(8, 10))
        );
        assert_eq!(
            ws_span(CARGO, f, "package.metadata.oap"),
            Some(LineSpan::new(11, 12))
        );
        // A bare basename match is enough (any path with that basename).
        assert!(ws_span(CARGO, "crates/x/Cargo.toml", "dependencies").is_some());
    }

    const PKG: &str = "\
{
  \"name\": \"spec-spine\",
  \"version\": \"0.3.0\",
  \"scripts\": {
    \"test\": \"node --test\",
    \"build\": \"tsc\"
  },
  \"dependencies\": {
    \"left-pad\": \"^1.0.0\"
  }
}
";

    #[test]
    fn package_json_member_keypaths() {
        let f = "package.json";
        assert_eq!(ws_span(PKG, f, "scripts"), Some(LineSpan::new(4, 7)));
        assert_eq!(ws_span(PKG, f, "dependencies"), Some(LineSpan::new(8, 10)));
        // One nested level resolves (depth 2).
        assert_eq!(ws_span(PKG, f, "scripts.test"), Some(LineSpan::new(5, 5)));
        // No member reached through an array, no over-depth.
        assert_eq!(ws_span(PKG, f, "scripts.test.shell"), None);
    }

    #[test]
    fn eligibility_boundary_is_hard() {
        // A dotted anchor on a non-eligible file MUST NOT resolve (§3.2). It
        // falls through to region markers (deny.toml has `#` comments) / nothing.
        let deny = "[advisories]\nignore = []\n";
        assert_eq!(resolve_section(deny, "deny.toml", "advisories"), None);
        // A non-workflow YAML is not keypath-eligible: the trigger keypath that
        // resolves inside a governed workflow does not resolve here.
        assert_eq!(
            resolve_section(WORKFLOW, "deploy/values.yaml", "on.merge_group"),
            None
        );
        // ...and the eligible workflow path DOES resolve it (proves the predicate).
        assert!(resolve_section(WORKFLOW, ".github/workflows/ci.yml", "on.merge_group").is_some());
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
