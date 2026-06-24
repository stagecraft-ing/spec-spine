---
id: constitutional-hierarchy
title: Constitutional Hierarchy
sidebar_position: 6
---

# Constitutional Hierarchy

Not every document in the spec-spine corpus is equal. Conflicts resolve in a fixed order, where the highest tier wins.

## The three tiers of authority

1. **The bootstrap spec (Tier 1)**: The spec that defines what a spec is. It bootstraps the corpus; its invariants are non-overridable.
2. **The constitution (Tier 2)**: Durable principles (e.g., markdown-only authored truth, compiler-owned JSON, spec-first development, determinism, legacy-as-evidence). It is subordinate to the bootstrap spec where they differ.
3. **Ordinary specs (Tier 3)**: Feature-level claims operating within the constitutional envelope.

This hierarchy ensures that foundational rules cannot be quietly superseded by a feature-level specification.
