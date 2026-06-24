---
id: waivers
title: Waivers
sidebar_position: 7
---

# Waivers

The coupling gate would be tyranny without an escape valve, and the escape valve itself has to be in the ledger. When a drift is deliberate and reviewed, it can be waived.

## The PR-body waiver

A named waiver declared in the PR body is explicit, scoped, and cites the reason it applies. It is the blessed path for legitimate consolidated changes, for example, a dependency refresh that touches many owned paths.

You should never amend an owner spec just to satisfy a mechanical refresh; waive instead.

To apply a waiver, add a line to the PR body using the configured keyword (default `Spec-Drift-Waiver:`):

```text
Spec-Drift-Waiver: refactor moves helper out of the owned section; behavior unchanged
```

The waiver is global to the run and downgrades violations to warnings.

## Amends-aware coupling

The gate is also aware of amendments. An amendment to a predecessor's paths is recognized as legitimate authority, not drift.

If a PR modifies `specs/<id>/spec.md`, the gate expands the owner set for paths owned by `<id>` to include any spec that `amends` `<id>`. This allows a newer spec to patch an older spec's territory without being flagged as drift, provided the older spec is also part of the diff.
