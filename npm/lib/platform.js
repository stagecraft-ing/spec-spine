'use strict';

// Spec: specs/007-distribution/spec.md
//
// The pure (process.platform, process.arch) -> platform-package mapping behind
// the launcher (bin/spec-spine.js). No I/O on the mapping path, so it is
// unit-tested directly (test/platform.test.js). Resolution of the installed
// binary is the one filesystem touch, and it takes injectable seams so the
// tests stay network- and disk-free.

const path = require('node:path');

// The five supported (platform-arch) targets. This table is one fact shared
// with release.yml's build matrix and install.sh's detection; keep all three in
// lockstep (specs/007-distribution/spec.md sec 3.2).
const SUPPORTED = {
  'darwin-arm64': '@spec-spine/cli-darwin-arm64',
  'darwin-x64': '@spec-spine/cli-darwin-x64',
  'linux-x64': '@spec-spine/cli-linux-x64',
  'linux-arm64': '@spec-spine/cli-linux-arm64',
  'win32-x64': '@spec-spine/cli-win32-x64',
};

class UnsupportedPlatformError extends Error {
  constructor(message) {
    super(message);
    this.name = 'UnsupportedPlatformError';
  }
}

// The in-archive binary name: `.exe` on Windows, bare elsewhere (matches the
// names release.yml stages into each archive).
function binaryName(platform) {
  return platform === 'win32' ? 'spec-spine.exe' : 'spec-spine';
}

// Pure resolution: (platform, arch, { muslLinux }) -> { key, packageName, binaryName }.
// Throws UnsupportedPlatformError on any host without a prebuilt binary.
function targetFor(platform, arch, opts = {}) {
  if (platform === 'linux' && opts.muslLinux === true) {
    throw new UnsupportedPlatformError(unsupportedMessage(platform, arch, 'musl'));
  }
  const key = `${platform}-${arch}`;
  const packageName = SUPPORTED[key];
  if (!packageName) {
    throw new UnsupportedPlatformError(unsupportedMessage(platform, arch));
  }
  return { key, packageName, binaryName: binaryName(platform) };
}

function unsupportedMessage(platform, arch, reason) {
  const host = reason === 'musl' ? `${platform}-${arch} (musl libc)` : `${platform}-${arch}`;
  const lines = [
    `spec-spine: no prebuilt binary for ${host}.`,
    'Prebuilt binaries cover darwin-arm64, darwin-x64, linux-x64 (glibc),',
    'linux-arm64 (glibc), and win32-x64. Install from source instead:',
    '    cargo install spec-spine-cli',
  ];
  if (reason === 'musl') {
    lines.push('(Alpine/musl: use a glibc-based image, or cargo install spec-spine-cli.)');
  }
  return lines.join('\n');
}

function missingPackageMessage(packageName) {
  return [
    `spec-spine: the prebuilt binary package '${packageName}' is not installed.`,
    'It is an optional dependency, so this happens with `npm install --no-optional`,',
    'on an unsupported platform, or when its download failed.',
    'Reinstall without --no-optional, or install from source:',
    '    cargo install spec-spine-cli',
  ].join('\n');
}

// Detect a musl (non-glibc) Linux. Returns false off Linux. Uses process.report's
// glibcVersionRuntime, which is present on glibc and absent on musl. Permissive
// on error: if we cannot tell, assume glibc and let resolution fail loudly later.
function isMuslLinux(proc = process) {
  if (proc.platform !== 'linux') {
    return false;
  }
  try {
    if (!proc.report || typeof proc.report.getReport !== 'function') {
      return false; // cannot tell; assume glibc and let resolution fail loudly if wrong
    }
    const report = proc.report.getReport();
    const glibc = report && report.header ? report.header.glibcVersionRuntime : undefined;
    return !glibc; // report present but no glibc runtime => musl
  } catch {
    return false;
  }
}

// Resolve the absolute path to the platform binary on this host. `process` and
// `resolve` are injectable for tests; `resolve` defaults to this module's
// require.resolve so optional platform packages are found from the main package.
// Throws UnsupportedPlatformError when the matching optional package is absent.
function resolveBinaryPath(opts = {}) {
  const proc = opts.process || process;
  const resolve = opts.resolve || ((request) => require.resolve(request));
  const { packageName, binaryName: bin } = targetFor(proc.platform, proc.arch, {
    muslLinux: isMuslLinux(proc),
  });
  let manifest;
  try {
    manifest = resolve(`${packageName}/package.json`);
  } catch {
    throw new UnsupportedPlatformError(missingPackageMessage(packageName));
  }
  return path.join(path.dirname(manifest), 'bin', bin);
}

module.exports = {
  SUPPORTED,
  UnsupportedPlatformError,
  binaryName,
  targetFor,
  unsupportedMessage,
  missingPackageMessage,
  isMuslLinux,
  resolveBinaryPath,
};
