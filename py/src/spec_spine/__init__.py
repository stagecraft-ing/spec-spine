"""spec-spine — Python/uvx distribution.

On a supported host you never import this package: pip/uv select a platform
wheel whose only payload is the prebuilt `spec-spine` binary in the wheel's
scripts directory, so `spec-spine`/`uvx spec-spine` runs the native binary with
no Python in the path. This importable package ships only in the sdist, which is
built solely on hosts with no matching wheel (musl, win-arm64, 32-bit), where its
`spec-spine` entry point prints a clear refusal (see _refuse.py). Spec:
specs/008-python-distribution/spec.md.
"""

from __future__ import annotations

try:  # populated from installed dist metadata; no hardcoded second source of truth
    from importlib.metadata import version as _version

    __version__ = _version("spec-spine")
except Exception:  # pragma: no cover - not installed as a dist
    __version__ = "0+unknown"
