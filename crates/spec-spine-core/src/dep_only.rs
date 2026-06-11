//! Mechanical dependency-only auto-waiver (spec 005 §3.5 amendment,
//! 2026-06-11).
//!
//! Dependabot-class PRs change only version strings inside `package.json`
//! dependency tables, but a `package.json` claimed by a spec (via its
//! manifest metadata) fires the coupling gate, and a bot cannot edit specs
//! or PR bodies. Path-level bypass is the wrong tool: it would exempt the
//! whole manifest, including the spec-binding metadata the gate exists to
//! protect. The mechanical rule instead compares the **parsed JSON** of the
//! base and head versions of each changed manifest: the two documents must
//! be semantically identical everywhere except *version strings* inside the
//! standard dependency tables (same package keys; values may differ only
//! where both sides are strings). Anything else — a new or removed package,
//! a `scripts` edit, a spec-metadata edit, a non-object document — refuses
//! the auto-waiver, fail-closed.
//!
//! Like everything in core, this is pure: the CLI resolves the merge-base,
//! fetches both file versions via `git show`, and hands the contents in.

use crate::couple::Waiver;

/// The `package.json` tables whose **values** (version strings) may change
/// under the auto-waiver. Key sets must be identical on both sides.
pub const DEPENDENCY_TABLES: &[&str] = &[
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

/// One changed file with both sides of its content. `None` = the file is
/// absent on that side (created or deleted) — never dependency-only.
#[derive(Clone, Debug)]
pub struct FileContents {
    pub path: String,
    pub base: Option<String>,
    pub head: Option<String>,
}

/// The mechanical verdict over a whole diff: `Some(Waiver)` iff **every**
/// entry is a `package.json` whose base→head change is dependency-only.
/// An empty slice yields `None` — there is nothing to waive.
pub fn dependency_only_waiver(files: &[FileContents]) -> Option<Waiver> {
    if files.is_empty() {
        return None;
    }
    for f in files {
        if !is_package_json(&f.path) {
            return None;
        }
        let (Some(base), Some(head)) = (&f.base, &f.head) else {
            return None; // created or deleted manifest — not a version bump
        };
        if !dependency_only_change(base, head) {
            return None;
        }
    }
    Some(Waiver {
        reason: format!(
            "dependency-only diff (mechanical auto-waiver): version-string \
             changes confined to dependency tables in {} package.json file(s)",
            files.len()
        ),
    })
}

/// `path` names a `package.json` manifest (any directory).
pub fn is_package_json(path: &str) -> bool {
    path == "package.json" || path.ends_with("/package.json")
}

/// True iff `base` and `head` parse as JSON objects that are **equal
/// everywhere except version strings inside [`DEPENDENCY_TABLES`]**:
///
/// - every non-table key: present in both with exactly equal values;
/// - every table: present on both sides (or neither), an object on both,
///   with identical key sets; per-key values may differ only when both
///   sides are strings.
///
/// Parse failure or a non-object document is `false` (fail-closed). A
/// formatting-only change (semantically equal documents) is `true`: it
/// alters no governed fact.
pub fn dependency_only_change(base: &str, head: &str) -> bool {
    use serde_json::Value;

    let (Ok(Value::Object(base)), Ok(Value::Object(head))) = (
        serde_json::from_str::<Value>(base),
        serde_json::from_str::<Value>(head),
    ) else {
        return false;
    };

    let keys: std::collections::BTreeSet<&String> = base.keys().chain(head.keys()).collect();
    for key in keys {
        let is_table = DEPENDENCY_TABLES.contains(&key.as_str());
        match (base.get(key), head.get(key)) {
            (Some(b), Some(h)) if is_table => {
                let (Value::Object(b), Value::Object(h)) = (b, h) else {
                    return false;
                };
                if b.keys().ne(h.keys()) {
                    return false; // package added or removed
                }
                for (name, bv) in b {
                    let hv = &h[name];
                    if bv != hv && !(bv.is_string() && hv.is_string()) {
                        return false;
                    }
                }
            }
            (Some(b), Some(h)) => {
                if b != h {
                    return false;
                }
            }
            // A key (table or not) present on only one side.
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fc(path: &str, base: &str, head: &str) -> FileContents {
        FileContents {
            path: path.to_string(),
            base: Some(base.to_string()),
            head: Some(head.to_string()),
        }
    }

    const BASE: &str = r#"{
        "name": "app",
        "version": "1.0.0",
        "scripts": { "build": "tsc" },
        "spec-spine": { "spec": "014-api" },
        "dependencies": { "express": "^4.18.0", "zod": "3.22.0" },
        "devDependencies": { "vitest": "1.0.0" }
    }"#;

    #[test]
    fn version_bump_is_dependency_only() {
        let head = BASE.replace("3.22.0", "3.23.1").replace("1.0.0\" }", "1.2.0\" }");
        assert!(dependency_only_change(BASE, &head));
    }

    #[test]
    fn added_package_is_not() {
        let head = BASE.replace(
            r#""zod": "3.22.0""#,
            r#""zod": "3.22.0", "left-pad": "1.0.0""#,
        );
        assert!(!dependency_only_change(BASE, &head));
    }

    #[test]
    fn removed_package_is_not() {
        let head = BASE.replace(r#", "zod": "3.22.0""#, "");
        assert!(!dependency_only_change(BASE, &head));
    }

    #[test]
    fn script_edit_is_not() {
        let head = BASE.replace(r#""build": "tsc""#, r#""build": "tsc && evil.sh""#);
        assert!(!dependency_only_change(BASE, &head));
    }

    #[test]
    fn spec_metadata_edit_is_not() {
        let head = BASE.replace("014-api", "999-other");
        assert!(!dependency_only_change(BASE, &head));
    }

    #[test]
    fn package_own_version_edit_is_not() {
        let head = BASE.replace(r#""version": "1.0.0""#, r#""version": "2.0.0""#);
        assert!(!dependency_only_change(BASE, &head));
    }

    #[test]
    fn new_table_is_not() {
        let head = BASE.replace(
            r#""devDependencies""#,
            r#""peerDependencies": { "react": "18" }, "devDependencies""#,
        );
        assert!(!dependency_only_change(BASE, &head));
    }

    #[test]
    fn reformat_only_is_dependency_only() {
        let head = serde_json::to_string_pretty(
            &serde_json::from_str::<serde_json::Value>(BASE).unwrap(),
        )
        .unwrap();
        assert!(dependency_only_change(BASE, &head));
    }

    #[test]
    fn unparseable_is_not() {
        assert!(!dependency_only_change(BASE, "{ not json"));
        assert!(!dependency_only_change("[]", "[]")); // non-object
    }

    #[test]
    fn waiver_requires_all_files_to_qualify() {
        let bump = fc(
            "apps/api/package.json",
            r#"{"dependencies":{"a":"1"}}"#,
            r#"{"dependencies":{"a":"2"}}"#,
        );
        let other = fc("src/lib.rs", "x", "y");
        assert!(dependency_only_waiver(std::slice::from_ref(&bump)).is_some());
        assert!(dependency_only_waiver(&[bump.clone(), other]).is_none());
        assert!(dependency_only_waiver(&[]).is_none());

        let created = FileContents {
            path: "package.json".to_string(),
            base: None,
            head: Some(r#"{"dependencies":{"a":"1"}}"#.to_string()),
        };
        assert!(dependency_only_waiver(&[created]).is_none());
    }

    #[test]
    fn package_json_path_shapes() {
        assert!(is_package_json("package.json"));
        assert!(is_package_json("apps/api/package.json"));
        assert!(!is_package_json("apps/api/package.json5"));
        assert!(!is_package_json("not-package.json/file.ts"));
    }
}
