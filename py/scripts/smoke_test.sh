#!/usr/bin/env bash
# Spec: specs/008-python-distribution/spec.md
#
# Local build + install smoke test: prove `spec-spine --version` resolves through
# a generated platform wheel to a real prebuilt binary, end to end, no network.
#
#   1. cargo build the host binary
#   2. generate the host platform wheel from that binary
#   3. install the wheel into a throwaway env (uv tool install, or pip into a venv)
#   4. run the installed `spec-spine --version` and assert it works + round-trips
#
# macOS / Linux only (bash). Windows uses `cargo install` or the .zip directly.

set -euo pipefail

PY_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$PY_DIR/.." && pwd)"

say() { printf 'smoke: %s\n' "$1" >&2; }
die() { printf 'smoke: error: %s\n' "$1" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

have cargo || die "cargo not found (needed to build the host binary)"
PYBIN="$(command -v python3 || command -v python)" || die "python not found"

VERSION="$("$PYBIN" - <<'PY'
import tomllib, pathlib, os
p = pathlib.Path(os.environ["PY_DIR"]) / "pyproject.toml"
print(tomllib.load(open(p, "rb"))["project"]["version"])
PY
)"
export PY_DIR
TARGET="$("$PYBIN" - <<'PY'
import sys, platform
osname = {"darwin": "darwin", "linux": "linux", "win32": "win32"}.get(sys.platform, sys.platform)
m = platform.machine().lower()
cpu = "arm64" if m in ("arm64", "aarch64") else "x64" if m in ("x86_64", "amd64") else m
print(f"{osname}-{cpu}")
PY
)"
say "host target: ${TARGET}, version: ${VERSION}"
case "$TARGET" in
  darwin-arm64|darwin-x64|linux-x64|linux-arm64) : ;;
  *) die "unsupported host target ${TARGET}; the shim has no wheel for it" ;;
esac

WORK="$(mktemp -d "${TMPDIR:-/tmp}/spec-spine-py-smoke.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT INT TERM

say "building host binary (cargo build --release --bin spec-spine)…"
cargo build --release --bin spec-spine --manifest-path "${REPO_ROOT}/Cargo.toml" >/dev/null
HOST_BIN="${REPO_ROOT}/target/release/spec-spine"
[ -x "$HOST_BIN" ] || die "built binary not found at ${HOST_BIN}"

say "generating the ${TARGET} platform wheel…"
WHEEL_OUT="${WORK}/wheels"
"$PYBIN" "${PY_DIR}/scripts/generate_wheels.py" \
  --version "$VERSION" --target "$TARGET" --binary "$HOST_BIN" --out "$WHEEL_OUT"
WHEEL="$(ls "${WHEEL_OUT}"/spec_spine-*.whl)"
[ -f "$WHEEL" ] || die "generator did not produce a wheel"
say "  wheel: $(basename "$WHEEL")"

if have uv; then
  say "installing with uv tool install and running…"
  export UV_TOOL_DIR="${WORK}/uv-tools" UV_TOOL_BIN_DIR="${WORK}/uv-bin"
  mkdir -p "$UV_TOOL_BIN_DIR"
  uv tool install --reinstall "$WHEEL" >/dev/null
  RUN="${UV_TOOL_BIN_DIR}/spec-spine"
else
  say "uv not found; installing into a venv with pip…"
  "$PYBIN" -m venv "${WORK}/venv"
  "${WORK}/venv/bin/pip" install --quiet "$WHEEL"
  RUN="${WORK}/venv/bin/spec-spine"
fi
[ -x "$RUN" ] || die "installed launcher not found/executable at ${RUN}"

say "running: spec-spine --version"
OUT="$("$RUN" --version)"
say "  -> ${OUT}"
echo "$OUT" | grep -q "$VERSION" || die "version output '${OUT}' did not contain ${VERSION}"
"$RUN" --help >/dev/null || die "spec-spine --help failed through the installed binary"

say "OK: wheel installs and the bundled binary runs as the spec-spine command"
