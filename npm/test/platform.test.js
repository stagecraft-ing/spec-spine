'use strict';

// Spec: specs/007-distribution/spec.md
//
// Network- and disk-free unit test of the (platform, arch) -> platform-package
// mapping and the unsupported-host error (spec 007 sec 3.2 / 3.4). Runs under
// the built-in node:test runner: `node --test test/`.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const platform = require('../lib/platform.js');
const { targetFor, resolveBinaryPath, isMuslLinux, UnsupportedPlatformError } = platform;

test('maps every supported triple to its scoped package and binary name', () => {
  const cases = [
    ['darwin', 'arm64', '@spec-spine/cli-darwin-arm64', 'spec-spine'],
    ['darwin', 'x64', '@spec-spine/cli-darwin-x64', 'spec-spine'],
    ['linux', 'x64', '@spec-spine/cli-linux-x64', 'spec-spine'],
    ['linux', 'arm64', '@spec-spine/cli-linux-arm64', 'spec-spine'],
    ['win32', 'x64', '@spec-spine/cli-win32-x64', 'spec-spine.exe'],
  ];
  for (const [os, arch, pkg, bin] of cases) {
    const t = targetFor(os, arch);
    assert.equal(t.packageName, pkg, `${os}-${arch} package`);
    assert.equal(t.binaryName, bin, `${os}-${arch} binary`);
    assert.equal(t.key, `${os}-${arch}`);
  }
});

test('the SUPPORTED table is exactly the five release triples', () => {
  assert.deepEqual(
    Object.keys(platform.SUPPORTED).sort(),
    ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64', 'win32-x64'],
  );
});

test('refuses unsupported triples with a source-build hint', () => {
  for (const [os, arch] of [
    ['win32', 'arm64'],
    ['linux', 'ia32'],
    ['freebsd', 'x64'],
    ['darwin', 'ppc64'],
  ]) {
    assert.throws(
      () => targetFor(os, arch),
      (err) => {
        assert.ok(err instanceof UnsupportedPlatformError);
        assert.match(err.message, new RegExp(`${os}-${arch}`));
        assert.match(err.message, /cargo install spec-spine-cli/);
        return true;
      },
      `${os}-${arch} should be unsupported`,
    );
  }
});

test('refuses musl Linux even on an otherwise-supported arch', () => {
  assert.throws(
    () => targetFor('linux', 'x64', { muslLinux: true }),
    (err) => {
      assert.ok(err instanceof UnsupportedPlatformError);
      assert.match(err.message, /musl/);
      assert.match(err.message, /glibc-based image/);
      assert.match(err.message, /cargo install spec-spine-cli/);
      return true;
    },
  );
  // glibc Linux on the same arch still resolves.
  assert.equal(targetFor('linux', 'x64', { muslLinux: false }).packageName, '@spec-spine/cli-linux-x64');
});

test('isMuslLinux: false off Linux; tracks glibcVersionRuntime on Linux', () => {
  assert.equal(isMuslLinux({ platform: 'darwin' }), false);
  assert.equal(isMuslLinux({ platform: 'win32' }), false);

  const glibcProc = {
    platform: 'linux',
    report: { getReport: () => ({ header: { glibcVersionRuntime: '2.39' } }) },
  };
  assert.equal(isMuslLinux(glibcProc), false);

  const muslProc = {
    platform: 'linux',
    report: { getReport: () => ({ header: {} }) },
  };
  assert.equal(isMuslLinux(muslProc), true);

  // No report available: permissive (assume glibc, fail later at resolution).
  assert.equal(isMuslLinux({ platform: 'linux' }), false);
});

test('resolveBinaryPath joins the resolved package dir with bin/<name>', () => {
  const fakeProcess = { platform: 'darwin', arch: 'arm64' };
  const resolve = (request) => {
    assert.equal(request, '@spec-spine/cli-darwin-arm64/package.json');
    return path.join('/fake/node_modules/@spec-spine/cli-darwin-arm64', 'package.json');
  };
  const got = resolveBinaryPath({ process: fakeProcess, resolve });
  assert.equal(got, path.join('/fake/node_modules/@spec-spine/cli-darwin-arm64', 'bin', 'spec-spine'));
});

test('resolveBinaryPath reports a missing optional package clearly', () => {
  const fakeProcess = { platform: 'darwin', arch: 'x64' };
  const resolve = () => {
    throw new Error('Cannot find module');
  };
  assert.throws(
    () => resolveBinaryPath({ process: fakeProcess, resolve }),
    (err) => {
      assert.ok(err instanceof UnsupportedPlatformError);
      assert.match(err.message, /@spec-spine\/cli-darwin-x64/);
      assert.match(err.message, /--no-optional/);
      assert.match(err.message, /cargo install spec-spine-cli/);
      return true;
    },
  );
});
