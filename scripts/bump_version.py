#!/usr/bin/env python3
"""Bump the package version in lockstep across all three distribution shims.

The release version lives in three committed files, and a release tag publishes
all three (crates.io, npm, PyPI) under the same number. They MUST agree:

  - Cargo.toml           [workspace.package] version  (crates.io)
                         + the internal [workspace.dependencies] pins for the
                           sibling crates (a 0.x MINOR bump crosses the caret
                           boundary, so `^0.3.0` would stop matching `0.4.0` and
                           break `cargo build`/`cargo package` if left stale)
  - npm/package.json     version + the 5 optionalDependencies pins  (npm)
  - py/pyproject.toml    [project] version  (PyPI)

History (why this script exists): v0.2.0 bumped Cargo + npm but missed
pyproject, so the PyPI release failed on generate_wheels.py's version lock
(spec 008 §3.5) while npm published fine -- the npm generator runs with
--write-main and silently rewrites a stale version to the tag, so only PyPI
enforces. This script removes the chance to bump one and forget another.

NOTE: this bumps the *package* version only. Schema-version constants in
spec-spine-types are deliberately decoupled (a release can ship without a
schema change); bump those separately per docs/schema-versioning.md.

Usage:
  scripts/bump_version.py 0.2.0      # set all four locations to 0.2.0
  scripts/bump_version.py v0.2.0     # leading 'v' is stripped
  scripts/bump_version.py --check    # verify all locations agree (no writes)
  scripts/bump_version.py --check 0.2.0   # ...and that they equal 0.2.0
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
PACKAGE_JSON = ROOT / "npm" / "package.json"
PYPROJECT = ROOT / "py" / "pyproject.toml"

# The first top-level `version = "..."` line. In Cargo.toml that is the
# [workspace.package] version (dependency versions are inline `{ version = ... }`,
# never at column 0); in pyproject.toml it is the [project] version.
_TOML_VERSION = re.compile(r'(?m)^(version\s*=\s*)"[^"]*"')
# The internal sibling-crate pins in Cargo.toml's [workspace.dependencies]:
# `spec-spine-types = { version = "x.y.z", path = ... }`. Matched by the
# dependency-name prefix at column 0, so this never collides with the
# [workspace.package] `version =` line above.
_CARGO_INTERNAL_DEP = re.compile(
    r'(?m)^(spec-spine-(?:types|core)\s*=\s*\{\s*version\s*=\s*)"[^"]*"'
)
# package.json: the single top-level "version" key (opt-dep keys are package
# names, never the literal "version").
_JSON_VERSION = re.compile(r'("version"\s*:\s*)"[^"]*"')
# The 5 platform pins under optionalDependencies.
_JSON_PIN = re.compile(r'("@spec-spine/cli-[^"]+"\s*:\s*)"[^"]*"')


def die(msg: str) -> "None":
    print(f"bump-version: {msg}", file=sys.stderr)
    raise SystemExit(1)


def normalize(v: str) -> str:
    return v[1:] if v.startswith("v") else v


def _sub_once(pattern: re.Pattern, text: str, version: str, what: str) -> str:
    new, n = pattern.subn(rf'\g<1>"{version}"', text, count=1)
    if n != 1:
        die(f"expected exactly one {what} to rewrite, found {n}")
    return new


# --- readers (also used by --check) ------------------------------------------

def read_cargo() -> str:
    m = _TOML_VERSION.search(CARGO.read_text())
    return m.group(0).split('"')[1] if m else die("no version in Cargo.toml")


def read_cargo_internal_deps() -> set[str]:
    return {m.group(0).split('"')[1] for m in _CARGO_INTERNAL_DEP.finditer(CARGO.read_text())}


def read_pyproject() -> str:
    m = _TOML_VERSION.search(PYPROJECT.read_text())
    return m.group(0).split('"')[1] if m else die("no version in pyproject.toml")


def read_npm() -> tuple[str, set[str]]:
    text = PACKAGE_JSON.read_text()
    vm = _JSON_VERSION.search(text)
    pins = {m.group(0).split('"')[3] for m in _JSON_PIN.finditer(text)}
    return (vm.group(0).split('"')[3] if vm else die("no version in package.json"), pins)


def current_versions() -> dict[str, str]:
    npm_v, npm_pins = read_npm()
    pin_repr = next(iter(npm_pins)) if len(npm_pins) == 1 else f"MIXED{sorted(npm_pins)}"
    dep_pins = read_cargo_internal_deps()
    dep_repr = next(iter(dep_pins)) if len(dep_pins) == 1 else f"MIXED{sorted(dep_pins)}"
    return {
        "Cargo.toml": read_cargo(),
        "Cargo.toml (internal deps)": dep_repr,
        "npm/package.json (version)": npm_v,
        "npm/package.json (pins)": pin_repr,
        "py/pyproject.toml": read_pyproject(),
    }


# --- commands ----------------------------------------------------------------

def do_bump(version: str) -> None:
    cargo = _sub_once(_TOML_VERSION, CARGO.read_text(), version, "version in Cargo.toml")
    cargo, ndeps = _CARGO_INTERNAL_DEP.subn(rf'\g<1>"{version}"', cargo)
    if ndeps != 2:
        die(f"expected 2 internal [workspace.dependencies] pins in Cargo.toml, rewrote {ndeps}")
    CARGO.write_text(cargo)

    pkg = PACKAGE_JSON.read_text()
    pkg = _sub_once(_JSON_VERSION, pkg, version, 'top-level "version" in package.json')
    pkg, npins = _JSON_PIN.subn(rf'\g<1>"{version}"', pkg)
    if npins != 5:
        die(f"expected 5 optionalDependencies pins in package.json, rewrote {npins}")
    PACKAGE_JSON.write_text(pkg)

    PYPROJECT.write_text(_sub_once(_TOML_VERSION, PYPROJECT.read_text(), version, "version in pyproject.toml"))

    # Read back and prove they all agree -- a regex that silently no-ops on a
    # restructured file would otherwise pass unnoticed.
    if not check(version, quiet=True):
        die("post-write verification failed (locations disagree after bump)")
    print(f"bumped all locations to {version}")
    for name, v in current_versions().items():
        print(f"  {name}: {v}")


def check(expected: str | None, quiet: bool = False) -> bool:
    versions = current_versions()
    distinct = set(versions.values())
    ok = len(distinct) == 1 and (expected is None or distinct == {expected})
    if not quiet:
        for name, v in versions.items():
            print(f"  {name}: {v}")
        if ok:
            print(f"OK: all locations agree on {distinct.pop()}")
        elif len(distinct) != 1:
            print("MISMATCH: version locations disagree", file=sys.stderr)
        else:
            print(f"MISMATCH: locations agree on {distinct.pop()} but expected {expected}", file=sys.stderr)
    return ok


def main(argv: list[str]) -> int:
    p = argparse.ArgumentParser(prog="bump-version", description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("version", nargs="?", help="target version, e.g. 0.2.0 (leading 'v' ok)")
    p.add_argument("--check", action="store_true",
                   help="verify all locations agree (and equal VERSION if given); no writes")
    args = p.parse_args(argv)

    if args.check:
        return 0 if check(normalize(args.version) if args.version else None) else 1
    if not args.version:
        p.error("a version is required (or pass --check to verify)")
    do_bump(normalize(args.version))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
