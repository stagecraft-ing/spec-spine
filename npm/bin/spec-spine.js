#!/usr/bin/env node
'use strict';

// Spec: specs/007-distribution/spec.md
//
// The launcher. Resolves the prebuilt binary for this host (see lib/platform.js)
// and exec's it, forwarding argv and the child's exit code. It is a pure
// translation layer: no flags added, no arguments rewritten, nothing printed on
// the success path. `spec-spine <args>` through npm is identical to the native
// binary. NOT a native addon; the Rust engine runs as a child process.

const { execFileSync } = require('node:child_process');
const { resolveBinaryPath } = require('../lib/platform.js');

let binPath;
try {
  binPath = resolveBinaryPath();
} catch (err) {
  process.stderr.write(`${err.message}\n`);
  process.exit(1);
}

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
} catch (err) {
  if (typeof err.status === 'number') {
    process.exit(err.status); // forward the binary's own exit code
  }
  if (err.signal) {
    process.stderr.write(`spec-spine: binary terminated by signal ${err.signal}\n`);
    process.exit(1);
  }
  process.stderr.write(`spec-spine: failed to run binary: ${err.message}\n`);
  process.exit(1);
}
