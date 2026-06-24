---
id: quickstart
title: Quickstart
sidebar_position: 2
---

# Quickstart

This guide walks you through scaffolding a new spec-spine corpus, compiling the registry, indexing your codebase, and running the coupling gate. It assumes you have already [installed spec-spine](installation.md).

You can run this end-to-end in an empty repository to see the mechanics in action.

## 1. Scaffold the corpus

Run `spec-spine init` at the root of your repository. This command generates the required directory structure, the configuration file, the tier-1 bootstrap spec, and the constitutional templates.

```bash
spec-spine init
```

This creates several files, including:
- `spec-spine.toml`: The configuration file with all knobs defaulted.
- `standards/spec/constitution.md`: The tier-2 durable principles.
- `specs/000-bootstrap/spec.md`: The hand-authored bootstrap spec that defines what a spec is.

## 2. Author a spec

Create a new specification file for a feature. In this example, we will create `specs/001-hello-world/spec.md`.

```bash
mkdir -p specs/001-hello-world
```

Create the file `specs/001-hello-world/spec.md` with the following content:

```markdown
---
status: approved
establishes:
  - src/main.rs
---

# Hello World

This spec establishes the main entry point for the application.
```

This frontmatter declares that the spec `001-hello-world` owns the file `src/main.rs` via an `establishes` edge.

## 3. Create the code

Now, create the file that the spec claims to own.

```bash
mkdir -p src
```

Create `src/main.rs` and add a comment header linking it back to the spec:

```rust
// Spec: specs/001-hello-world/spec.md

fn main() {
    println!("Hello, world!");
}
```

## 4. Compile the registry

The compiler reads the markdown corpus and emits a frozen JSON registry. This is the spec-as-source view.

```bash
spec-spine compile
```

You will see output indicating that the registry shards have been written to `.derived/spec-registry/by-spec/`.

## 5. Index the codebase

The indexer scans the repository for manifests and code files, mapping them back to their owning specs. This is the code-as-source view.

```bash
spec-spine index
```

The index shards are written to `.derived/codebase-index/by-spec/` and `.../by-package/`.

## 6. Run the coupling gate

The coupling gate joins the registry and the index against a Git diff. It refuses the merge if a path is modified without its owning spec also being modified (or vice versa).

First, commit your changes so the gate has a baseline to compare against:

```bash
git add .
git commit -m "Initial commit with spec and code"
```

Now, make a modification to `src/main.rs` without updating the spec:

```rust
// Spec: specs/001-hello-world/spec.md

fn main() {
    println!("Hello, modified world!");
}
```

Run the coupling gate against the `HEAD` commit:

```bash
spec-spine couple --base HEAD --head HEAD
```

*(Note: In a real CI environment, `--base` would be `origin/main` and `--head` would be the PR branch. Here we use `HEAD` to simulate a local diff.)*

Because we modified `src/main.rs` without modifying `specs/001-hello-world/spec.md`, the gate will exit with status code `1` and report a drift violation.

To resolve the drift, you must either modify the spec to reflect the code change, or provide a waiver in the PR body.

## Next steps

- Read the [Concepts](../concepts/overview.md) to understand the authority graph.
- Explore the [CLI Reference](../cli/overview.md) for detailed command usage.
- See the [Adoption Guide](../adoption-guide.md) for integrating spec-spine into an existing project.
