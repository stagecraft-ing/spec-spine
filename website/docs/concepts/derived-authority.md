---
id: derived-authority
title: Derived Authority
sidebar_position: 3
---

# Derived Authority

Authority over any given path is **derived** by walking the graph, not declared directly. "Who currently has authority over file X, function Y?" is a query against the graph, not a guess.

## How authority is computed

"Who currently owns unit X" is a near-pure set-membership query, not a runtime graph walk; the indexer pre-flattens edges into resolved units at index time. The clearance rule used by the coupling gate works as follows:

1. **Base ownership:** The owners of a path are every spec whose edge units resolve to it.
2. **Supersession transfer:** `supersedes` contributes the predecessor's paths into the superseding spec's resolved units at index time, so current authority transfers. `establishes` remains the historical-origin edge.
3. **Amends-awareness:** If the changed path is exactly `specs/<id>/spec.md`, the owner set is *expanded* to include every spec that `amends` `<id>`, but **only if the base owner set is non-empty**. This is the strict-expansion guard: an amendment can add owners to an already-firing path, but can never silently enroll a new one.

## The code-to-spec link

Code connects back to specs via three linkage directions:

1. **Manifest key:** A compilation unit declares its owning spec in its manifest. For Rust, this is `[package.metadata.<ns>].spec = "NNN-slug"`. For npm, it is a top-level `{"<ns>": {"spec": "NNN-slug"}}`. This links a crate or package to a spec.
2. **Comment header:** A file can declare its owning spec via a doc-comment at the file root, e.g., `// Spec: specs/NNN-slug/spec.md`. This links a file to a spec.
3. **Spec edges:** A spec declares the units it owns via its frontmatter edges (`establishes`, `extends`, etc.). This links a spec to code.

The codebase indexer (`spec-spine index`) walks the tree, hashes these manifests along with the spec files, and builds the inverse map. A query layer backs the `authorities(unit)` function, answering "who currently owns this unit?" for both the coupling gate and any other consumer.
