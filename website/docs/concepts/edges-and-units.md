---
id: edges-and-units
title: Edges and Units
sidebar_position: 2
---

# Edges and Units

A document does not just claim "I exist." It declares, in machine-readable frontmatter, its relationships to the rest of the corpus. These relationships are expressed as **typed edges**, and the targets of those edges are **authority units**.

## The Eight Typed Edges

There are exactly eight typed edges in the spec-spine grammar. Seven of them carry ownership semantics; one is purely referential.

| Edge | Ownership? | Meaning |
|---|---|---|
| `establishes` | yes | I am the document that first brought this code into being. |
| `extends` | yes | I add surface to a predecessor's territory without disturbing it. |
| `refines` | yes | I tighten behavior on a specific aspect. |
| `supersedes` | yes | I replace this predecessor, partially or fully. I inherit its current authority. |
| `amends` | yes | I patch a predecessor in place (clarification, correction, restriction). I gain co-authority over its `spec.md`. |
| `co_authority` | yes | I share a path with another document on a named section. |
| `constrains` | yes | I assert an invariant that everyone else must respect. |
| `references` | **no** | I point at another document or artifact without claiming authority over it. |

`references` is the only non-owning edge; the coupling gate ignores it.

*(Note: `origin: retroactive` is a bootstrap marker, not an edge. It indicates that a spec is declaring authority it has held since before the graph existed.)*

## The Six Authority Units

Authority is over units, not just files. The graph expresses ownership at finer granularity. A unit can be one of six kinds:

| Unit kind | Shape | Resolution |
|---|---|---|
| `file` | `{ kind: file, path }` | Literal path. A trailing `/` indicates a directory subtree (prefix match). A bare string is shorthand for a `file` unit. |
| `section` | `{ kind: section, file, anchor }` | Anchor parser by file type (Makefile target, Markdown heading slug, `region:` marker, workflow `jobs.<name>`, or bounded keypath). |
| `symbol` | `{ kind: symbol, id }` | Resolved via tree-sitter (Rust and TypeScript supported) to a `(file, line-span)`. |
| `directory` | `{ kind: directory, path }` | Explicit subtree kind (same prefix-match semantics as a trailing-slash `file`). |
| `crate` | `{ kind: crate, id }` | Resolved by manifest name to the package directory subtree. |
| `module` | `{ kind: module, id }` | `::`-qualified module path, resolved via the module index. |

Section-scoped co-authority is the property that makes the canonical hard case tractable: a project-wide build file where many features each add targets. Co-authority is section-scoped, not file-scoped.
