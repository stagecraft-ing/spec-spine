# spec-spine-cli

The `spec-spine` command-line tool: a thin translation of `spec-spine-core`
results into stdout/stderr and stable exit codes. All `process::exit`, stdout,
and `git`/clock side effects live here; the engine ([`spec-spine-core`]) stays
pure.

```
spec-spine init [--force]                  # scaffold a new adopter (config, standards, specs/000, rules)
spec-spine compile                         # specs/*/spec.md -> .derived/spec-registry/registry.json
spec-spine index                           # scan manifests + specs -> .derived/codebase-index/index.json
spec-spine index check                     # staleness gate (exit 2 if stale)
spec-spine registry list [--status S]      # list specs
spec-spine registry show <id>              # show one spec
spec-spine registry status-report          # counts by status
spec-spine registry relationships <id>     # relationship neighborhood
spec-spine lint [--fail-on-warn] [--fail-on-info]   # corpus conformance
spec-spine couple --base origin/main --head HEAD [--pr-body FILE]   # the PR-time drift gate
```

`cargo install spec-spine-cli` installs the `spec-spine` binary. A global
`--repo <DIR>` selects the repository root (defaults to the current directory).

Exit codes: `0` ok · `1` validation failure / not found / coupling drift ·
`2` stale · `3` I/O / parse / schema / config.

See [docs/adoption-guide.md] for the full install → init → annotate → wire-CI
walkthrough. License: Apache-2.0.

[`spec-spine-core`]: https://crates.io/crates/spec-spine-core
[docs/adoption-guide.md]: https://github.com/bartekus/spec-spine/blob/main/docs/adoption-guide.md
