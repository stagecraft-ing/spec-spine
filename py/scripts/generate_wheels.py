#!/usr/bin/env python3
"""Spec: specs/008-python-distribution/spec.md (extends 007-distribution).

Assemble the five per-triple platform wheels for the Python/uvx channel from the
release archives, at publish time -- the Python parallel of npm's
generate-platform-packages.js. Each wheel carries exactly one prebuilt binary in
its `*.data/scripts/` directory and a platform tag, so pip/uv install only the
matching one and the binary lands on PATH (executable) with no Python launcher.
Binaries and wheels are never committed; this script rebuilds them on demand from
the same archives release.yml already produced (no second Rust build).

Two input modes (mirroring the npm generator):
  --archives <dir>   extract binaries from spec-spine-<tag>-<triple>.{tar.gz,zip}
  --binary <path>    use one already-built binary (requires a single --target)

Options:
  --version <v>      release version (e.g. 0.1.0 or v0.1.0); default: pyproject
  --out <dir>        output root; default: <py>/dist/wheels
  --target <key>     build only this platform key (repeatable); default: all five
  --requires-python  Requires-Python stamped on the wheels (default: >=3.8;
                     deliberately permissive so a Python version mismatch never
                     pushes a supported host onto the sdist)
  --write-main       rewrite pyproject version to lock to --version
  --lock-only        do not build wheels; only verify/lock the version (implies
                     --write-main)
  --build-sdist      also build the source distribution (the unsupported-host
                     refusal) via `python -m build --sdist`

Usage:
  python scripts/generate_wheels.py --archives ./dist/archives
  python scripts/generate_wheels.py --target linux-x64 \
      --binary ../target/release/spec-spine
"""

from __future__ import annotations

import argparse
import base64
import gzip
import hashlib
import io
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

PY_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PY_DIR / "src"))
from spec_spine import platform_map as pm  # noqa: E402  (after sys.path setup)

REPO_ROOT = PY_DIR.parent
GENERATOR = "spec-spine-generate-wheels 1.0"


def die(msg: str) -> None:
    sys.stderr.write(f"generate-wheels: {msg}\n")
    raise SystemExit(1)


def log(msg: str) -> None:
    sys.stdout.write(msg + "\n")


def normalize_version(v: str) -> str:
    return v[1:] if v.startswith("v") else v


def read_pyproject_version() -> str:
    import tomllib

    with open(PY_DIR / "pyproject.toml", "rb") as fh:
        return tomllib.load(fh)["project"]["version"]


# --- binary acquisition ------------------------------------------------------

def _extract_from_archive(target: str, archives_dir: Path, tag: str, bin_file: str) -> Path:
    """Extract the binary for `target` from its release archive into a temp dir.

    Extracts the WHOLE archive and then reads the binary by its known name rather
    than selecting a member: the release tar is built with `tar -C staging .`, so
    members are stored "./"-prefixed (./spec-spine). A strict tar (GNU tar) will
    not match a bare `spec-spine`; extracting everything lands the binary at
    <tmp>/<bin_file> on every tar/zip flavor. (The npm generator learned this the
    hard way; see spec 007 §3.6.)
    """
    rec = pm.TARGETS[target]
    is_win = rec["windows"]
    ext = "zip" if is_win else "tar.gz"
    archive = archives_dir / f"spec-spine-{tag}-{rec['triple']}.{ext}"
    if not archive.exists():
        die(f"archive not found for {target}: {archive}")
    tmp = Path(tempfile.mkdtemp(prefix="spec-spine-extract-"))
    if is_win:
        with zipfile.ZipFile(archive) as zf:
            zf.extractall(tmp)
    else:
        with tarfile.open(archive, "r:gz") as tf:
            try:
                tf.extractall(tmp, filter="data")  # py3.12+: path-traversal safe
            except TypeError:  # pragma: no cover - older Python on the runner
                tf.extractall(tmp)
    # Find by basename anywhere in the tree (handles ./ prefix and flat layouts).
    matches = [p for p in tmp.rglob(bin_file) if p.is_file()]
    if not matches:
        die(f"archive {archive} did not contain {bin_file}")
    return matches[0]


def _resolve_binary(target: str, opts: argparse.Namespace, tag: str, bin_file: str) -> Path:
    if opts.binary:
        src = Path(opts.binary).resolve()
        if not src.exists():
            die(f"--binary not found: {src}")
        return src
    return _extract_from_archive(target, Path(opts.archives).resolve(), tag, bin_file)


# --- wheel assembly ----------------------------------------------------------

def _record_hash(data: bytes) -> str:
    digest = hashlib.sha256(data).digest()
    return "sha256=" + base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")


def _metadata(version: str, requires_python: str) -> bytes:
    body = (
        "Metadata-Version: 2.4\n"
        f"Name: {pm.DIST_NAME}\n"
        f"Version: {version}\n"
        "Summary: The spec-spine CLI: compile a markdown spec corpus into a "
        "deterministic authority ledger and refuse code that drifts from its "
        "owning spec. Ships the prebuilt binary; no Rust toolchain required.\n"
        "License-Expression: Apache-2.0\n"
        "License-File: LICENSE\n"
        "Project-URL: Homepage, https://github.com/bartekus/spec-spine\n"
        "Project-URL: Repository, https://github.com/bartekus/spec-spine\n"
        "Project-URL: Issues, https://github.com/bartekus/spec-spine/issues\n"
        "Keywords: spec,specification,authority,governance,coupling,cli,rust,deterministic\n"
        "Classifier: Development Status :: 4 - Beta\n"
        "Classifier: Environment :: Console\n"
        "Classifier: Programming Language :: Rust\n"
        "Classifier: Topic :: Software Development :: Quality Assurance\n"
        f"Requires-Python: {requires_python}\n"
        "Description-Content-Type: text/markdown\n"
        "\n"
        "Prebuilt `spec-spine` binary wheel. The binary is installed to your "
        "environment's scripts directory; `spec-spine`/`uvx spec-spine` runs it "
        "directly. See https://github.com/bartekus/spec-spine#readme.\n"
    )
    return body.encode("utf-8")


def _wheel_meta(tag: str) -> bytes:
    return (
        "Wheel-Version: 1.0\n"
        f"Generator: {GENERATOR}\n"
        "Root-Is-Purelib: false\n"
        f"Tag: {tag}\n"
    ).encode("utf-8")


def _zipinfo(name: str, mode: int) -> zipfile.ZipInfo:
    zi = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))  # fixed -> reproducible
    zi.compress_type = zipfile.ZIP_DEFLATED
    zi.create_system = 3  # Unix, so the mode bits are honored
    zi.external_attr = (0o100000 | (mode & 0o7777)) << 16  # S_IFREG | mode
    return zi


def _build_wheel(target: str, version: str, requires_python: str,
                 binary_path: Path, out_root: Path) -> Path:
    rec = pm.TARGETS[target]
    bin_file = pm.binary_name(rec["windows"])
    tag = f"py3-none-{rec['wheel_platform']}"
    distinfo = f"{pm.DIST_STEM}-{version}.dist-info"
    datadir = f"{pm.DIST_STEM}-{version}.data/scripts"

    # METADATA unconditionally declares `License-File: LICENSE`, so the file MUST
    # be present in the wheel or the artifact is internally inconsistent (and
    # twine check will not catch it). Resolve it deterministically: prefer the
    # in-channel copy (what `cp ../LICENSE LICENSE` produces and what the sdist
    # ships via pyproject license-files), fall back to the repo root for an
    # in-tree build. Fail loudly if neither exists -- never emit a wheel whose
    # METADATA claims a license file it does not carry.
    license_path = next(
        (p for p in (PY_DIR / "LICENSE", REPO_ROOT / "LICENSE") if p.exists()),
        None,
    )
    if license_path is None:
        raise SystemExit(
            "generate_wheels: LICENSE not found at "
            f"{PY_DIR / 'LICENSE'} or {REPO_ROOT / 'LICENSE'}; "
            "METADATA declares 'License-File: LICENSE' so the file is required. "
            "Run `cp ../LICENSE LICENSE` in the channel dir first (see the "
            "publish-pypi job)."
        )
    license_data = license_path.read_bytes()

    # (arcname, bytes, mode). RECORD is appended last and lists every entry.
    entries: list[tuple[str, bytes, int]] = [
        (f"{datadir}/{bin_file}", binary_path.read_bytes(), 0o755),
        (f"{distinfo}/METADATA", _metadata(version, requires_python), 0o644),
        (f"{distinfo}/WHEEL", _wheel_meta(tag), 0o644),
    ]
    entries.append((f"{distinfo}/licenses/LICENSE", license_data, 0o644))

    record_lines = [f"{name},{_record_hash(data)},{len(data)}" for name, data, _ in entries]
    record_name = f"{distinfo}/RECORD"
    record_lines.append(f"{record_name},,")  # RECORD lists itself with no hash/size
    record_data = ("\n".join(record_lines) + "\n").encode("utf-8")
    entries.append((record_name, record_data, 0o644))

    out_root.mkdir(parents=True, exist_ok=True)
    wheel_path = out_root / f"{pm.DIST_STEM}-{version}-{tag}.whl"
    if wheel_path.exists():
        wheel_path.unlink()
    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data, mode in entries:
            zf.writestr(_zipinfo(name, mode), data)
    log(f"  generated {wheel_path.name}  ({len(binary_path.read_bytes())} byte binary)")
    return wheel_path


# --- sdist normalization ------------------------------------------------------

def _normalize_sdist(sdist: Path) -> None:
    """Rewrite the sdist into a canonical form: members sorted by name, owner
    cleared, mtimes clamped to SOURCE_DATE_EPOCH (default 1980-01-01, the same
    fixed date the wheels' zip entries use), gzip header timestamp zeroed.
    setuptools stamps wall-clock mtimes on the files it regenerates at build
    time (PKG-INFO, egg-info, setup.cfg), so the raw sdist differs byte-wise
    run to run; after this rewrite it is a pure function of file contents --
    the same reproducibility property the wheels get from _zipinfo. Member
    contents are untouched."""
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "315532800"))  # 1980-01-01
    with tarfile.open(sdist, "r:gz") as tf:
        members = [
            (m, tf.extractfile(m).read() if m.isfile() else None)
            for m in tf.getmembers()
        ]
    members.sort(key=lambda pair: pair[0].name)  # dirs prefix-sort before contents
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as out:
        for m, data in members:
            m.mtime = epoch
            m.uid = m.gid = 0
            m.uname = m.gname = ""
            # Read-back members carry the original float mtime in their PAX
            # extended headers, which would override the clamp at write time.
            m.pax_headers = {}
            out.addfile(m, io.BytesIO(data) if data is not None else None)
    with open(sdist, "wb") as fh:
        with gzip.GzipFile(fileobj=fh, mode="wb", mtime=0) as gz:
            gz.write(raw.getvalue())


# --- version lock (Python-side §3.5) -----------------------------------------

def _pyproject_path() -> Path:
    return PY_DIR / "pyproject.toml"


def lock_version(version: str) -> None:
    """Rewrite the single `version = "..."` line in [project]. Python has one
    project + N wheels, so there are no per-dependency pins to lock (unlike npm's
    optionalDependencies) -- the wheels are stamped with --version at build."""
    path = _pyproject_path()
    text = path.read_text(encoding="utf-8")
    import re

    new, n = re.subn(
        r'(?m)^(version\s*=\s*)"[^"]*"', rf'\g<1>"{version}"', text, count=1
    )
    if n != 1:
        die("could not find a single version line to lock in pyproject.toml")
    path.write_text(new, encoding="utf-8")
    log(f"  locked pyproject version to {version}")


def verify_version_lock(version: str) -> None:
    current = read_pyproject_version()
    if current != version:
        die(
            "version lock mismatch (spec 008 §3.5):\n"
            f"  - pyproject version is {current}, expected {version}\n"
            "Re-run with --write-main to update, or fix pyproject.toml."
        )


# --- cli ---------------------------------------------------------------------

def parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(prog="generate-wheels", add_help=True)
    p.add_argument("--version")
    p.add_argument("--archives")
    p.add_argument("--binary")
    p.add_argument("--out")
    p.add_argument("--target", action="append", default=[], dest="targets")
    p.add_argument("--requires-python", default=">=3.8")
    p.add_argument("--write-main", action="store_true")
    p.add_argument("--lock-only", action="store_true")
    p.add_argument("--build-sdist", action="store_true")
    opts = p.parse_args(argv)
    if opts.lock_only:
        opts.write_main = True
    return opts


def main(argv: list[str]) -> int:
    opts = parse_args(argv)
    version = normalize_version(opts.version or read_pyproject_version())
    tag = f"v{version}"

    if opts.binary and len(opts.targets) != 1:
        die("--binary requires exactly one --target")

    if opts.write_main:
        lock_version(version)
    else:
        verify_version_lock(version)

    if opts.lock_only:
        log(f"version lock verified/applied: {version}")
        return 0

    out_root = Path(opts.out).resolve() if opts.out else (PY_DIR / "dist" / "wheels")
    targets = opts.targets or list(pm.TARGETS.keys())
    for t in targets:
        if t not in pm.TARGETS:
            die(f"unknown target: {t}")

    log(f"generating platform wheels for {version} ({tag}):")
    for t in targets:
        rec = pm.TARGETS[t]
        bin_file = pm.binary_name(rec["windows"])
        binary = _resolve_binary(t, opts, tag, bin_file)
        _build_wheel(t, version, opts.requires_python, binary, out_root)

    if opts.build_sdist:
        log("building sdist (the unsupported-host refusal)…")
        subprocess.run(
            [sys.executable, "-m", "build", "--sdist", "--outdir", str(out_root), str(PY_DIR)],
            check=True,
        )
        sdist = out_root / f"{pm.DIST_STEM}-{version}.tar.gz"
        if not sdist.exists():
            die(f"sdist build did not produce {sdist}")
        _normalize_sdist(sdist)
        log(f"  normalized {sdist.name} (deterministic member order/mtimes)")

    log(f"done -> {out_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
