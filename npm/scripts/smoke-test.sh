#!/usr/bin/env bash
# Spec: specs/007-distribution/spec.md
#
# Local pack + install smoke test: prove `spec-spine --version` resolves through
# the npm launcher to a real prebuilt binary, end to end, with no network.
#
#   1. cargo build the host binary
#   2. generate the host platform package from that binary
#   3. npm pack the main package and the platform package
#   4. install both tarballs into a throwaway project
#   5. run the installed `spec-spine --version` and assert it works
#
# macOS / Linux only (bash). Windows uses `cargo install` or the .zip directly.

set -euo pipefail

NPM_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$NPM_DIR/.." && pwd)"

say() { printf 'smoke: %s\n' "$1" >&2; }
die() { printf 'smoke: error: %s\n' "$1" >&2; exit 1; }

command -v node >/dev/null 2>&1 || die "node not found"
command -v npm  >/dev/null 2>&1 || die "npm not found"
command -v cargo >/dev/null 2>&1 || die "cargo not found (needed to build the host binary)"

VERSION="$(node -p "require('${NPM_DIR}/package.json').version")"
TARGET="$(node -p "process.platform + '-' + process.arch")"
say "host target: ${TARGET}, version: ${VERSION}"

case "$TARGET" in
  darwin-arm64|darwin-x64|linux-x64|linux-arm64) BIN="spec-spine" ;;
  win32-x64) die "use the Windows smoke path (.zip); this script is bash-only" ;;
  *) die "unsupported host target ${TARGET}; the shim has no binary for it" ;;
esac

WORK="$(mktemp -d "${TMPDIR:-/tmp}/spec-spine-smoke.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT INT TERM

say "building host binary (cargo build --release --bin spec-spine)…"
cargo build --release --bin spec-spine --manifest-path "${REPO_ROOT}/Cargo.toml" >/dev/null
HOST_BIN="${REPO_ROOT}/target/release/${BIN}"
[ -x "$HOST_BIN" ] || die "built binary not found at ${HOST_BIN}"

say "generating the ${TARGET} platform package…"
PKG_OUT="${WORK}/packages"
node "${NPM_DIR}/scripts/generate-platform-packages.js" \
  --version "$VERSION" --target "$TARGET" --binary "$HOST_BIN" --out "$PKG_OUT"
PLATFORM_PKG_DIR="${PKG_OUT}/@spec-spine/cli-${TARGET}"
[ -d "$PLATFORM_PKG_DIR" ] || die "generator did not produce ${PLATFORM_PKG_DIR}"

say "packing tarballs…"
PACK_DIR="${WORK}/tarballs"
mkdir -p "$PACK_DIR"
# `npm pack` honors the package `files` field; LICENSE is copied in by the
# release workflow at publish time and is optional here.
MAIN_TGZ="$(cd "$PACK_DIR" && npm pack "$NPM_DIR" --silent)"
PLATFORM_TGZ="$(cd "$PACK_DIR" && npm pack "$PLATFORM_PKG_DIR" --silent)"
say "  main:     ${MAIN_TGZ}"
say "  platform: ${PLATFORM_TGZ}"

say "installing into a throwaway project…"
PROJ="${WORK}/proj"
mkdir -p "$PROJ"
( cd "$PROJ" && npm init -y >/dev/null 2>&1 )
( cd "$PROJ" && npm install --no-audit --no-fund --silent \
    "${PACK_DIR}/${MAIN_TGZ}" "${PACK_DIR}/${PLATFORM_TGZ}" )

BIN_LINK="${PROJ}/node_modules/.bin/spec-spine"
[ -e "$BIN_LINK" ] || die "launcher not linked at ${BIN_LINK}"

say "running the installed launcher: spec-spine --version"
OUT="$("$BIN_LINK" --version)"
say "  -> ${OUT}"
echo "$OUT" | grep -q "$VERSION" || die "version output '${OUT}' did not contain ${VERSION}"

# A real subcommand round-trips through the launcher (exit code forwarding).
"$BIN_LINK" --help >/dev/null || die "spec-spine --help failed through the launcher"

say "OK: launcher resolves the platform package and exec's the binary"
