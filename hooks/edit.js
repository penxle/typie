#!/usr/bin/env node

import { exec } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { promisify } from 'node:util';

const execAsync = promisify(exec);

try {
  const input = readFileSync(0, 'utf8');
  const hookData = JSON.parse(input);

  const filePath = hookData.tool_input.file_path;
  if (!filePath) {
    process.exit(0);
  }

  if (/\.(ts|tsx|js|jsx|svelte)$/.test(filePath)) {
    try {
      await execAsync(`pnpm -w eslint --fix "${filePath}"`);
    } catch (err) {
      console.error(err.stderr || err.stdout);
      process.exit(2);
    }
  }

  try {
    await execAsync(`pnpm -w prettier --write "${filePath}"`);
  } catch (err) {
    console.error(err);
  }

  process.exit(0);
} catch (err) {
  console.error(err);
  process.exit(1);
}
