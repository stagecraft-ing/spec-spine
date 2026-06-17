#!/usr/bin/env bash
# Spec: 020-derived-artifact-merge-driver
#
# Git merge driver `spec-spine-derived-regen` for the committed derived
# artifacts, which since spec 024 are sharded per authority unit:
#   .derived/spec-registry/by-spec/<id>.json        (compiler output)
#   .derived/codebase-index/by-spec/<id>.json        (indexer output)
#   .derived/codebase-index/by-package/<slug>.json   (indexer output)
#
# Sharding means two PRs that touch DIFFERENT specs/packages write disjoint
# files and no longer conflict at all. This driver covers the residual RARE
# same-shard conflict (two PRs editing the same spec/package): it regenerates
# BOTH artifacts from the merged working tree and hands the fresh shard named by
# %P back to git as the resolution, so a rebase onto a merged PR no longer leaves
# a derived-artifact conflict to fix by hand.
#
# Enable in this clone (opt-in, one command):
#   ./.githooks/enable-merge-driver.sh
#
# Or by hand:
#   git config merge.spec-spine-derived-regen.name "regenerate spec-spine derived artifacts on conflict"
#   git config merge.spec-spine-derived-regen.driver ".githooks/merge-derived-index.sh %O %A %B %P"
#
# Disable:
#   git config --unset merge.spec-spine-derived-regen.driver
#   git config --unset merge.spec-spine-derived-regen.name
#
# Path assignment lives in committed .gitattributes (the shard globs):
#   .derived/spec-registry/by-spec/*.json    merge=spec-spine-derived-regen
#   .derived/codebase-index/by-spec/*.json    merge=spec-spine-derived-regen
#   .derived/codebase-index/by-package/*.json merge=spec-spine-derived-regen
#
# Git invokes:  <driver> %O %A %B %P
#   $1 = %O  ancestor version  (unused: both artifacts are fully derived)
#   $2 = %A  ours temp file     (the driver MUST leave the merged result here and exit 0)
#   $3 = %B  theirs version     (unused)
#   $4 = %P  pathname being merged
#
# Fail-closed: if no spec-spine binary is found or regeneration fails, exit 1 and
# leave the conflict in place. The CI staleness gate (`spec-spine index check`,
# spec 004) remains the freshness source of truth; this driver is a convenience
# over the conflict, never a replacement for the gate.
#
# Known limitation (ported from OAP spec 188): regeneration is only as complete
# as the working tree at git's driver-invocation moment, so on some rebases the
# auto-heal can still emit a clean-but-stale hash (it cannot self-detect). Run
# `spec-spine index check` after the merge; the gate is the source of truth.

set -eu

OURS="${2:?merge driver expects %A as \$2}"
PATHNAME="${4:-<unknown>}"

root="$(git rev-parse --show-toplevel)"
cd "$root"

# Locate a usable spec-spine binary: prefer a release build, then debug, then
# one already on PATH. (.exe suffix for git-bash on Windows.)
BIN=""
for cand in \
  "target/release/spec-spine" "target/release/spec-spine.exe" \
  "target/debug/spec-spine" "target/debug/spec-spine.exe"; do
  if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ] && command -v spec-spine >/dev/null 2>&1; then
  BIN="spec-spine"
fi
if [ -z "$BIN" ]; then
  cat >&2 <<EOF
[merge-derived-index] no spec-spine binary found; cannot auto-resolve $PATHNAME.
            Build it (\`cargo build --bin spec-spine\`), then re-run the rebase/merge,
            or resolve manually:
                spec-spine compile && spec-spine index && git add .derived/
EOF
  exit 1
fi

# Regenerate BOTH artifacts from the merged working tree. The compiler/indexer
# are deterministic for a given committed input set, so the regenerated pair is
# the correct union of both branches' input changes.
if ! "$BIN" compile >/dev/null 2>&1 || ! "$BIN" index >/dev/null 2>&1; then
  cat >&2 <<EOF
[merge-derived-index] \`spec-spine compile && index\` failed; leaving conflict in
            $PATHNAME for manual resolution
            (\`spec-spine compile && spec-spine index && git add .derived/\`).
EOF
  exit 1
fi

# Hand the freshly regenerated artifact named by %P back to git as the result.
if [ ! -f "$PATHNAME" ]; then
  echo "[merge-derived-index] regenerated tree has no $PATHNAME; leaving conflict." >&2
  exit 1
fi
cp "$PATHNAME" "$OURS"
echo "[merge-derived-index] regenerated $PATHNAME from the merged tree." >&2
exit 0
