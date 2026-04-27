#!/usr/bin/env node

import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const hash = crypto.createHash('sha256').update(root).digest('hex').slice(0, 12);
const targetDir = path.join(os.tmpdir(), `localpush-cargo-${hash}`);

if (process.argv.includes('--mkdir')) {
  fs.mkdirSync(targetDir, { recursive: true });
}

process.stdout.write(targetDir);
