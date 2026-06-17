#!/usr/bin/env bash
# Spec: 020-derived-artifact-merge-driver
#
# One-command, idempotent enablement of the `spec-spine-derived-regen` git merge
# driver in THIS clone. Run once per clone: the driver registration lives in
# per-clone `.git/config`, which is not committed, so each clone (you may keep
# several) must enable it locally. Worktrees created off a clone inherit the
# clone's config, so one run covers every worktree under it.
#
#   ./.githooks/enable-merge-driver.sh
#
# Disable:
#   git config --unset merge.spec-spine-derived-regen.driver
#   git config --unset merge.spec-spine-derived-regen.name
#
# The path to driver assignment (the sharded `.derived/**/by-spec/*.json` and
# `.derived/codebase-index/by-package/*.json` globs -> `merge=spec-spine-derived-regen`,
# spec 024) lives in committed `.gitattributes`, and the driver itself is
# `.githooks/merge-derived-index.sh`; both travel with the repo. This script only
# wires the non-committed registration that connects them. Safe to re-run;
# `git config` overwrites idempotently.

set -eu

root="$(git rev-parse --show-toplevel)"
cd "$root"

git config merge.spec-spine-derived-regen.name "regenerate spec-spine derived artifacts on conflict"
git config merge.spec-spine-derived-regen.driver ".githooks/merge-derived-index.sh %O %A %B %P"

echo "[enable-merge-driver] spec-spine-derived-regen registered in $root/.git/config"
echo "  name:      $(git config --get merge.spec-spine-derived-regen.name)"
echo "  driver:    $(git config --get merge.spec-spine-derived-regen.driver)"
echo "  registry:  $(git check-attr merge .derived/spec-registry/by-spec/000-x.json)"
echo "  index:     $(git check-attr merge .derived/codebase-index/by-spec/000-x.json)"
echo "[enable-merge-driver] derived-artifact conflicts will now auto-regenerate on merge/rebase."
