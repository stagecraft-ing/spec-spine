//! Golden tests for the spec 011 projections: the rendered markdown is a pure
//! function of `(config, index.json bytes)`: byte-exact, LF endings, trailing
//! newline. Fixtures go through `load_index` so the wire shapes are exercised
//! end to end.

use spec_spine_core::{load_index, orphans, render_markdown};
use spec_spine_types::Config;

/// Packages, mappings, orphans and diagnostics deliberately unsorted in the
/// fixture: the render order below is the projection's doing.
const FULL_FIXTURE: &str = r#"{
  "schemaVersion": "0.1.0",
  "build": {
    "indexerId": "spec-spine",
    "indexerVersion": "0.2.0",
    "repoRoot": ".",
    "contentHash": "cafe1234"
  },
  "packages": [
    {
      "name": "zeta",
      "path": "crates/zeta",
      "kind": "rust-lib",
      "version": "0.1.0",
      "specRef": "002-zeta"
    },
    { "name": "alpha", "path": "npm/alpha", "kind": "npm-package" }
  ],
  "traceability": {
    "mappings": [
      {
        "specId": "002-zeta",
        "specStatus": "approved",
        "implementingPaths": [{ "path": "crates/zeta", "source": "spec-edge" }]
      },
      { "specId": "001-alpha", "implementingPaths": [] }
    ],
    "orphanedSpecs": ["009-zzz", "003-orphan"],
    "untracedCode": ["npm/alpha"]
  },
  "diagnostics": {
    "warnings": [
      { "code": "I-002", "message": "untraced package", "path": "npm/alpha" }
    ],
    "errors": [
      {
        "code": "I-003",
        "message": "unit resolved nowhere",
        "path": "crates/zeta/src/gone.rs"
      }
    ]
  }
}"#;

const FULL_EXPECTED: &str = "# spec-spine codebase index\n\
\n\
- schemaVersion: 0.1.0\n\
- contentHash: cafe1234\n\
\n\
## Packages\n\
\n\
| name | path | kind | version | spec |\n\
|---|---|---|---|---|\n\
| alpha | npm/alpha | npm-package | - | - |\n\
| zeta | crates/zeta | rust-lib | 0.1.0 | 002-zeta |\n\
\n\
## Traceability\n\
\n\
| spec | status | paths | units |\n\
|---|---|---|---|\n\
| 001-alpha | - | 0 | 0 |\n\
| 002-zeta | approved | 1 | 0 |\n\
\n\
### Orphaned specs\n\
\n\
- 003-orphan\n\
- 009-zzz\n\
\n\
### Untraced code\n\
\n\
- npm/alpha\n\
\n\
## Diagnostics\n\
\n\
- I-002 [warning] untraced package (npm/alpha)\n\
- I-003 [error] unit resolved nowhere (crates/zeta/src/gone.rs)\n";

/// No orphans, no untraced code, no diagnostics: those sections are omitted
/// entirely (spec 011 §3.2 / §3.4).
const EMPTY_SECTIONS_FIXTURE: &str = r#"{
  "schemaVersion": "0.1.0",
  "build": {
    "indexerId": "spec-spine",
    "indexerVersion": "0.2.0",
    "repoRoot": ".",
    "contentHash": "beef5678"
  },
  "packages": [
    { "name": "alpha", "path": "crates/alpha", "kind": "rust-bin" }
  ],
  "traceability": {
    "mappings": [
      {
        "specId": "001-alpha",
        "implementingPaths": [{ "path": "crates/alpha", "source": "spec-edge" }]
      }
    ],
    "orphanedSpecs": [],
    "untracedCode": []
  },
  "diagnostics": { "warnings": [], "errors": [] }
}"#;

const EMPTY_SECTIONS_EXPECTED: &str = "# spec-spine codebase index\n\
\n\
- schemaVersion: 0.1.0\n\
- contentHash: beef5678\n\
\n\
## Packages\n\
\n\
| name | path | kind | version | spec |\n\
|---|---|---|---|---|\n\
| alpha | crates/alpha | rust-bin | - | - |\n\
\n\
## Traceability\n\
\n\
| spec | status | paths | units |\n\
|---|---|---|---|\n\
| 001-alpha | - | 1 | 0 |\n";

#[test]
fn render_full_fixture_is_byte_exact() {
    let index = load_index(FULL_FIXTURE.as_bytes()).unwrap();
    assert_eq!(render_markdown(&Config::default(), &index), FULL_EXPECTED);
}

#[test]
fn render_omits_empty_sections() {
    let index = load_index(EMPTY_SECTIONS_FIXTURE.as_bytes()).unwrap();
    assert_eq!(
        render_markdown(&Config::default(), &index),
        EMPTY_SECTIONS_EXPECTED
    );
}

#[test]
fn orphans_are_id_sorted() {
    let index = load_index(FULL_FIXTURE.as_bytes()).unwrap();
    assert_eq!(orphans(&index), ["003-orphan", "009-zzz"]);

    let empty = load_index(EMPTY_SECTIONS_FIXTURE.as_bytes()).unwrap();
    assert!(orphans(&empty).is_empty());
}
