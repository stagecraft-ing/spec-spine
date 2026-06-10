#!/usr/bin/env node
'use strict';

// Spec: specs/007-distribution/spec.md
//
// Assemble the per-triple platform packages (@spec-spine/cli-<os>-<cpu>) from
// the release archives, at publish time. Each package carries exactly one
// prebuilt binary plus os/cpu fields so npm installs only the matching one.
// Binaries and generated packages are never committed; this script rebuilds
// them from artifacts on demand.
//
// Two input modes:
//   --archives <dir>   extract binaries from spec-spine-<tag>-<triple>.{tar.gz,zip}
//   --binary <path>    use one already-built binary (requires a single --target)
//
// Usage:
//   node scripts/generate-platform-packages.js --archives ./dist/archives
//   node scripts/generate-platform-packages.js --target darwin-arm64 --binary ../target/release/spec-spine
//
// Options:
//   --version <v>   release version (e.g. 0.1.0 or v0.1.0); default: main package.json version
//   --out <dir>     output root; default: <npm>/dist/packages
//   --target <key>  build only this platform key (repeatable); default: all five
//   --write-main    rewrite the main package.json version + optionalDependencies to lock to --version
//   --lock-only     do not generate packages; only verify/lock the main package (implies --write-main)

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const NPM_DIR = path.resolve(__dirname, '..');
const REPO_ROOT = path.resolve(NPM_DIR, '..');

// platform key -> { release triple, node os, node cpu }. Mirrors release.yml and
// lib/platform.js's SUPPORTED map (specs/007-distribution/spec.md sec 3.2).
const TARGETS = {
  'darwin-arm64': { triple: 'aarch64-apple-darwin', os: 'darwin', cpu: 'arm64' },
  'darwin-x64': { triple: 'x86_64-apple-darwin', os: 'darwin', cpu: 'x64' },
  'linux-x64': { triple: 'x86_64-unknown-linux-gnu', os: 'linux', cpu: 'x64' },
  'linux-arm64': { triple: 'aarch64-unknown-linux-gnu', os: 'linux', cpu: 'arm64' },
  'win32-x64': { triple: 'x86_64-pc-windows-msvc', os: 'win32', cpu: 'x64' },
};

function die(msg) {
  process.stderr.write(`generate-platform-packages: ${msg}\n`);
  process.exit(1);
}

function log(msg) {
  process.stdout.write(`${msg}\n`);
}

function parseArgs(argv) {
  const opts = { targets: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    const next = () => {
      i += 1;
      if (i >= argv.length) die(`missing value for ${a}`);
      return argv[i];
    };
    switch (a) {
      case '--version':
        opts.version = next();
        break;
      case '--archives':
        opts.archives = next();
        break;
      case '--binary':
        opts.binary = next();
        break;
      case '--out':
        opts.out = next();
        break;
      case '--target':
        opts.targets.push(next());
        break;
      case '--write-main':
        opts.writeMain = true;
        break;
      case '--lock-only':
        opts.lockOnly = true;
        opts.writeMain = true;
        break;
      default:
        die(`unknown argument: ${a}`);
    }
  }
  return opts;
}

// Strip a leading "v": "v0.1.0" -> "0.1.0".
function normalizeVersion(v) {
  return v.replace(/^v/, '');
}

function mainPackageJsonPath() {
  return path.join(NPM_DIR, 'package.json');
}

function readMainPackage() {
  return JSON.parse(fs.readFileSync(mainPackageJsonPath(), 'utf8'));
}

// Extract the binary for one target from an archive directory into a temp file;
// returns its path. Tarballs via `tar`, zips via `unzip` (both present on the
// publish runner).
function extractBinary(target, archivesDir, tag, binFile) {
  const t = TARGETS[target];
  const isWin = t.os === 'win32';
  const archive = path.join(
    archivesDir,
    `spec-spine-${tag}-${t.triple}.${isWin ? 'zip' : 'tar.gz'}`,
  );
  if (!fs.existsSync(archive)) {
    die(`archive not found for ${target}: ${archive}`);
  }
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'spec-spine-extract-'));
  // Extract the whole archive rather than selecting one member: the release tar
  // is built with `tar -C staging .`, so members are stored "./"-prefixed
  // (./spec-spine). GNU tar (the Linux runner) will not match a bare
  // `spec-spine` against `./spec-spine` and errors "Not found in archive";
  // macOS bsdtar matches leniently, which masked this locally. Extracting all
  // members lands the binary at <tmp>/<binFile> on every tar/unzip flavor.
  if (isWin) {
    execFileSync('unzip', ['-o', '-q', archive, '-d', tmp], { stdio: 'inherit' });
  } else {
    execFileSync('tar', ['-C', tmp, '-xzf', archive], { stdio: 'inherit' });
  }
  const extracted = path.join(tmp, binFile);
  if (!fs.existsSync(extracted)) {
    die(`archive ${archive} did not contain ${binFile}`);
  }
  return extracted;
}

function platformPackageJson(target, version) {
  const t = TARGETS[target];
  return {
    name: `@spec-spine/cli-${target}`,
    version,
    description: `Prebuilt spec-spine CLI binary for ${target}. Installed automatically by the \`spec-spine\` package; do not depend on it directly.`,
    license: 'Apache-2.0',
    repository: {
      type: 'git',
      url: 'git+https://github.com/bartekus/spec-spine.git',
    },
    os: [t.os],
    cpu: [t.cpu],
    files: ['bin/'],
  };
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function generateOne(target, version, tag, outRoot, opts) {
  const t = TARGETS[target];
  const binFile = t.os === 'win32' ? 'spec-spine.exe' : 'spec-spine';

  let source;
  if (opts.binary) {
    source = path.resolve(opts.binary);
    if (!fs.existsSync(source)) die(`--binary not found: ${source}`);
  } else {
    source = extractBinary(target, path.resolve(opts.archives), tag, binFile);
  }

  const pkgDir = path.join(outRoot, '@spec-spine', `cli-${target}`);
  const binDir = path.join(pkgDir, 'bin');
  fs.rmSync(pkgDir, { recursive: true, force: true });
  fs.mkdirSync(binDir, { recursive: true });

  fs.copyFileSync(source, path.join(binDir, binFile));
  fs.chmodSync(path.join(binDir, binFile), 0o755);

  writeJson(path.join(pkgDir, 'package.json'), platformPackageJson(target, version));

  const license = path.join(REPO_ROOT, 'LICENSE');
  if (fs.existsSync(license)) {
    fs.copyFileSync(license, path.join(pkgDir, 'LICENSE'));
  }
  fs.writeFileSync(
    path.join(pkgDir, 'README.md'),
    `# @spec-spine/cli-${target}\n\n` +
      `Prebuilt \`spec-spine\` binary for \`${target}\` (${t.triple}).\n\n` +
      'This package is installed automatically as an optional dependency of the\n' +
      '[`spec-spine`](https://www.npmjs.com/package/spec-spine) package. Do not\n' +
      'depend on it directly; its name and contents are an implementation detail.\n',
  );

  log(`  generated ${pkgDir}`);
  return pkgDir;
}

// Lock the main package.json version + optionalDependencies to `version`.
function lockMainPackage(version) {
  const file = mainPackageJsonPath();
  const pkg = readMainPackage();
  pkg.version = version;
  pkg.optionalDependencies = pkg.optionalDependencies || {};
  for (const target of Object.keys(TARGETS)) {
    pkg.optionalDependencies[`@spec-spine/cli-${target}`] = version;
  }
  writeJson(file, pkg);
  log(`  locked main package.json to ${version}`);
}

// Verify the committed main package already locks every target to `version`.
function verifyMainLock(version) {
  const pkg = readMainPackage();
  const problems = [];
  if (pkg.version !== version) {
    problems.push(`main version is ${pkg.version}, expected ${version}`);
  }
  const deps = pkg.optionalDependencies || {};
  for (const target of Object.keys(TARGETS)) {
    const name = `@spec-spine/cli-${target}`;
    if (deps[name] !== version) {
      problems.push(`${name} is pinned to ${deps[name]}, expected ${version}`);
    }
  }
  if (problems.length > 0) {
    die(
      `version lock mismatch (specs/007-distribution/spec.md sec 3.5):\n  - ${problems.join('\n  - ')}\n` +
        'Re-run with --write-main to update, or fix package.json.',
    );
  }
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const version = normalizeVersion(opts.version || readMainPackage().version);
  const tag = `v${version}`;

  if (opts.binary && opts.targets.length !== 1) {
    die('--binary requires exactly one --target');
  }

  if (opts.writeMain) {
    lockMainPackage(version);
  } else {
    verifyMainLock(version);
  }

  if (opts.lockOnly) {
    log(`version lock verified/applied: ${version}`);
    return;
  }

  const outRoot = path.resolve(opts.out || path.join(NPM_DIR, 'dist', 'packages'));
  const targets = opts.targets.length > 0 ? opts.targets : Object.keys(TARGETS);
  for (const target of targets) {
    if (!TARGETS[target]) die(`unknown target: ${target}`);
  }

  log(`generating platform packages for ${version} (${tag}):`);
  for (const target of targets) {
    generateOne(target, version, tag, outRoot, opts);
  }
  log(`done -> ${outRoot}`);
}

main();
