---
id: "NNN-slug"                 # MUST equal the directory name; NNN = unique 3-digit ordinal
title: "Short imperative title"
status: draft                  # draft | approved | superseded | retired
created: "YYYY-MM-DD"
summary: >
  One short paragraph: what territory this spec claims and why it exists.
# --- optional descriptive keys ---
# owner: "name"
# authors: ["name"]
# risk: medium                 # low | medium | high | critical
# implementation: pending      # pending | in-progress | complete | n-a | deferred
# depends_on: ["NNN-other"]
# domain: "..."                # only if domains.allowed is configured non-empty
# kind: "..."                  # only if kind.allowed is configured non-empty
# --- typed edges (declare territory + relationships) ---
# establishes:
#   - { kind: file, path: "src/thing.rs" }
#   - "src/whole_file.rs"      # bare string == { kind: file, path: ... }
#   - { kind: symbol, id: "crate::module::function" }
#   - { kind: section, file: "Makefile", anchor: "build-target" }
# extends:
#   - { spec: "NNN-predecessor", unit: { kind: file, path: "src/added.rs" } }
# refines:
#   - { aspect: "error-handling", unit: { kind: symbol, id: "crate::run" } }
# supersedes: ["NNN-predecessor"]
# amends: ["NNN-predecessor"]
# co_authority:
#   - { unit: { kind: section, file: "Cargo.toml", anchor: "workspace-deps" }, with_specs: ["NNN-other"] }
# constrains:
#   - { unit: { kind: file, path: "src/api.rs" }, note: "public API is frozen" }
# references:
#   - { unit: { kind: file, path: "docs/notes.md" }, role: "context" }
# --- lifecycle / amendment (as applicable) ---
# superseded_by: "NNN-successor"     # required when status: superseded
# retirement_rationale: "why"        # required when status: retired
# amends_sections: ["anchor"]
# unamendable: ["anchor"]
---

# NNN: Title

## 1. Purpose

What problem this spec solves and what it owns.

## 2. Territory

The units this spec claims authority over (mirrors the frontmatter edges, in prose).

## 3. Behavior

What the governed code must do. Use MUST/SHOULD/MAY.

## 4. Out of scope

What this spec deliberately does not cover.
