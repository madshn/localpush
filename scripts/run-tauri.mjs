#!/usr/bin/env node

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cargoTargetDirScript = path.join(root, 'scripts', 'cargo-target-dir.mjs');
const tauriBin = path.join(
  root,
  'node_modules',
  '.bin',
  process.platform === 'win32' ? 'tauri.cmd' : 'tauri',
);

async function resolveTargetDir() {
  if (process.env.CARGO_TARGET_DIR) {
    return process.env.CARGO_TARGET_DIR;
  }

  const { stdout } = await import('node:child_process').then(
    ({ execFile }) =>
      new Promise((resolve, reject) => {
        execFile(
          process.execPath,
          [cargoTargetDirScript, '--mkdir'],
          { cwd: root },
          (error, stdoutText, stderrText) => {
            if (error) {
              reject(new Error(stderrText || error.message));
              return;
            }
            resolve({ stdout: stdoutText });
          },
        );
      }),
  );

  return stdout.trim();
}

const args = process.argv.slice(2);
const resolvedTargetDir = await resolveTargetDir();
fs.mkdirSync(resolvedTargetDir, { recursive: true });

const child = spawn(fs.existsSync(tauriBin) ? tauriBin : 'tauri', args, {
  cwd: root,
  env: { ...process.env, CARGO_TARGET_DIR: resolvedTargetDir },
  stdio: 'inherit',
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
