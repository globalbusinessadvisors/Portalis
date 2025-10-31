#!/usr/bin/env node

/**
 * Portalis - GPU-accelerated Python to Rust/WASM transpiler
 *
 * This is the Node.js entry point for the npm package.
 * The actual CLI is a Rust binary downloaded during installation.
 */

const { spawn } = require('child_process');
const path = require('path');

const binPath = path.join(__dirname, 'bin', process.platform === 'win32' ? 'portalis.exe' : 'portalis');

const child = spawn(binPath, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: true,
});

child.on('exit', (code) => {
  process.exit(code);
});
