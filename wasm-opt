#!/usr/bin/env node

const path = require('path');
const cp = require('child_process');

const platform = process.platform;

const rootDir = path.resolve(__dirname, './wasm-opt' + (platform === 'win32' ? '.exe' : ''));

const spawned = cp.spawn(
  rootDir,
  process.argv.slice(2),
  { cwd: './', shell: platform === 'win32' }
);

spawned.stdout.on('data', (data) => {
  console.log(`${data}`);
});

spawned.stderr.on('data', (data) => {
  console.log(`${data}`);
});
