---
id: rules
title: Rules Reference
sidebar_position: 7
---

# Rules Reference

The kit ships 3 rules under `kit/.claude/rules/`. These are the same floor
`spec-spine init` scaffolds, so if you ran `init` you already have them. All are
portable.

| Rule | What it enforces |
|---|---|
| `orchestrator-rules` | Execute phased work in order; stop at human checkpoints; keep the working tree green; recompute derived artifacts before opening a PR. |
| `governed-artifact-reads` | Read compiled `.derived/` artifacts only through `spec-spine` subcommands, never via ad-hoc `jq` / `awk` / `sed` / `python`. |
| `adversarial-prompt-refusal` | When the coupling gate fails because code and its owning spec disagree, surface the contradiction; never edit the spec just to satisfy a mechanical refresh. Waive instead, with a cited `Spec-Drift-Waiver:`. |

These three are the behavioral backbone of the loop. `orchestrator-rules` makes
checkpoints real stops, `governed-artifact-reads` keeps reads typed (so schema
drift fails cleanly at the deserializer), and `adversarial-prompt-refusal` is the
coherence guard that stops an agent from rewriting a spec to launder
contradicting code.

## Add a paths-scoped context rule

The upstream source also carried paths-scoped rules that auto-load documentation
when an agent works inside a particular directory (a build-commands reference, a
service-layer description). Those are instances of a generic pattern rather than
portable content, so they are not shipped. To add one:

1. Create `.claude/rules/<your-area>.md`.
2. Add `paths:` frontmatter scoping it to a directory or file glob.
3. Document the conventions, build commands, or invariants for that area.

The rule loads automatically when an agent reads or edits a matching file. The
pattern is generic; the content is yours.
