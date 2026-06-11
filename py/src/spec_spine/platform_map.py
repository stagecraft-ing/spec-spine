"""Spec: specs/008-python-distribution/spec.md (extends 007-distribution).

The single (os, cpu) -> target table for the Python/uvx channel, plus host
detection and the unsupported-host message. This is the Python-side mirror of
npm/lib/platform.js's SUPPORTED map and the §3.2 table in spec 007: the five
triples, the in-archive binary name, and the per-target wheel platform tag are
ONE FACT, and this module is its home on the Python side. A drift between this
table, release.yml's matrix, install.sh's detection, and npm/lib/platform.js is
a governed change (spec 007 §2, now four places).

Pure data + pure functions: no filesystem or network on the mapping path, so the
table is unit-tested directly (test/test_platform_map.py). Used by three
consumers: scripts/generate_wheels.py (publish time), _refuse.py (runtime, on the
unsupported path), and the tests.
"""

from __future__ import annotations

import platform as _platform
import sys
import sysconfig

# platform key -> release triple, npm-style (os, cpu), wheel platform tag, and
# whether the in-archive binary is `.exe`. The wheel platform tag is what makes
# pip/uv install only the matching wheel on a host -- the Python analogue of
# npm's os/cpu fields. glibc is encoded by the `manylinux_*` tags: a musl host
# matches none of these and falls through to the sdist refusal (§3.4 parity).
TARGETS: dict[str, dict] = {
    "darwin-arm64": {
        "triple": "aarch64-apple-darwin",
        "os": "darwin",
        "cpu": "arm64",
        "wheel_platform": "macosx_11_0_arm64",
        "windows": False,
    },
    "darwin-x64": {
        "triple": "x86_64-apple-darwin",
        "os": "darwin",
        "cpu": "x64",
        "wheel_platform": "macosx_10_12_x86_64",
        "windows": False,
    },
    "linux-x64": {
        "triple": "x86_64-unknown-linux-gnu",
        "os": "linux",
        "cpu": "x64",
        "wheel_platform": "manylinux_2_17_x86_64",
        "windows": False,
    },
    "linux-arm64": {
        "triple": "aarch64-unknown-linux-gnu",
        "os": "linux",
        "cpu": "arm64",
        "wheel_platform": "manylinux_2_17_aarch64",
        "windows": False,
    },
    "win32-x64": {
        "triple": "x86_64-pc-windows-msvc",
        "os": "win32",
        "cpu": "x64",
        "wheel_platform": "win_amd64",
        "windows": True,
    },
}

# The Python distribution name and the importable package / data-dir stem. PyPI
# normalizes "spec-spine" <-> "spec_spine"; the .data/scripts and .dist-info
# directories inside a wheel use the underscore form.
DIST_NAME = "spec-spine"
DIST_STEM = "spec_spine"


class UnsupportedHostError(RuntimeError):
    """Raised when the running host has no prebuilt binary."""


def binary_name(windows: bool) -> str:
    """In-archive / in-wheel binary name: `.exe` on Windows, bare elsewhere."""
    return "spec-spine.exe" if windows else "spec-spine"


def target_for(os_name: str, cpu: str) -> dict:
    """(os, cpu) -> target record. Raises UnsupportedHostError for any host with
    no prebuilt binary. `os`/`cpu` are the npm-style names ('darwin'/'linux'/
    'win32', 'x64'/'arm64'), not Python's ('linux2', 'x86_64')."""
    key = f"{os_name}-{cpu}"
    rec = TARGETS.get(key)
    if rec is None:
        raise UnsupportedHostError(unsupported_message(os_name, cpu))
    return {"key": key, **rec, "binary_name": binary_name(rec["windows"])}


# --- runtime host detection (the unsupported / sdist path) -------------------

def _normalize_machine(machine: str) -> str | None:
    m = machine.lower()
    if m in ("arm64", "aarch64"):
        return "arm64"
    if m in ("x86_64", "amd64", "x64"):
        return "x64"
    return None  # i686/ppc64/etc. -> unsupported, surfaced clearly


def is_musl_linux() -> bool:
    """True on a musl (non-glibc) Linux. Permissive: if we cannot tell, assume
    glibc and let resolution fail loudly later (mirrors npm's isMuslLinux)."""
    if sys.platform != "linux":
        return False
    try:
        if "musl" in (sysconfig.get_platform() or ""):
            return True
        libc, _ver = _platform.libc_ver()
        return libc != "" and "glibc" not in libc.lower()
    except Exception:
        return False


def detect_host() -> dict:
    """Best-effort host classification for messaging. Returns a dict with `key`
    (platform key or None), `os`, `cpu`, and `reason` ('musl' | 'arch' | None)."""
    os_map = {"darwin": "darwin", "linux": "linux", "win32": "win32"}
    os_name = os_map.get(sys.platform, sys.platform)
    cpu = _normalize_machine(_platform.machine())
    if os_name == "linux" and is_musl_linux():
        return {"key": None, "os": os_name, "cpu": cpu or "?", "reason": "musl"}
    if cpu is None or f"{os_name}-{cpu}" not in TARGETS:
        return {"key": None, "os": os_name, "cpu": cpu or _platform.machine().lower(),
                "reason": "arch"}
    return {"key": f"{os_name}-{cpu}", "os": os_name, "cpu": cpu, "reason": None}


def unsupported_message(os_name: str, cpu: str, reason: str | None = None) -> str:
    host = f"{os_name}-{cpu}" + (" (musl libc)" if reason == "musl" else "")
    lines = [
        f"spec-spine: no prebuilt binary for {host}.",
        "Prebuilt wheels cover darwin-arm64, darwin-x64, linux-x64 (glibc),",
        "linux-arm64 (glibc), and win32-x64. Install from source instead:",
        "    cargo install spec-spine-cli",
    ]
    if reason == "musl":
        lines.append(
            "(Alpine/musl: use a glibc-based image, or cargo install spec-spine-cli.)"
        )
    lines.append(
        "(If you are on a supported host and reached this message, you likely "
        "installed with --no-binary; reinstall allowing wheels.)"
    )
    return "\n".join(lines)
