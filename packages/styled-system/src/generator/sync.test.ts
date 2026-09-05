import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { CONTRAST_GATE, failingPairs } from './contrast.ts';
import { generate, OUTPUT_PATHS, REPO_ROOT } from './index.ts';
import { loadThemes } from './load.ts';

describe('generated outputs', () => {
  it('match the committed files', async () => {
    const { outputs } = await generate(REPO_ROOT);
    for (const relative of Object.values(OUTPUT_PATHS)) {
      expect(fs.readFileSync(path.join(REPO_ROOT, relative), 'utf8'), relative).toBe(outputs[relative]);
    }
  });

  it.skipIf(!CONTRAST_GATE)('meet the contrast floors', () => {
    const { presets } = loadThemes(path.join(REPO_ROOT, 'assets/themes'));
    expect(failingPairs(presets)).toEqual([]);
  });

  it('reports no failures through generate() while the gate is off', async () => {
    const { failures } = await generate(REPO_ROOT);
    expect(CONTRAST_GATE).toBe(false);
    expect(failures).toEqual([]);
  });
});
