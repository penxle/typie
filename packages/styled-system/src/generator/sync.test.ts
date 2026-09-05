import fs from 'node:fs';
import path from 'node:path';
import { converter, differenceEuclidean } from 'culori';
import { describe, expect, it } from 'vitest';
import { CONTRAST_GATE, failingPairs } from './contrast.ts';
import { generate, OUTPUT_PATHS, REPO_ROOT } from './index.ts';
import { loadThemes } from './load.ts';
import { EDITOR_KEYS } from './schema.ts';

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

  it('keeps the colour menu hues distinct in every preset', () => {
    const { presets } = loadThemes(path.join(REPO_ROOT, 'assets/themes'));
    const hues = EDITOR_KEYS.filter(
      (key) => key.startsWith('text.') && !['black', 'darkgray', 'gray', 'lightgray', 'white', 'bright'].includes(key.slice(5)),
    );
    expect(hues).toHaveLength(17);
    const distance = differenceEuclidean('oklab');
    const oklch = converter('oklch');
    for (const preset of presets) {
      for (const [index, first] of hues.entries()) {
        for (const second of hues.slice(index + 1)) {
          expect(distance(preset.editor[first], preset.editor[second]), `${preset.id} ${first}/${second}`).toBeGreaterThanOrEqual(0.02);
        }
        const second = hues[(index + 1) % hues.length];
        const from = oklch(preset.editor[first]);
        const to = oklch(preset.editor[second]);
        const forward = ((((to?.h ?? 0) - (from?.h ?? 0)) % 360) + 360) % 360;
        expect(forward, `${preset.id} ${first}→${second} hue order`).toBeGreaterThan(0);
        expect(forward, `${preset.id} ${first}→${second} hue order`).toBeLessThan(180);
      }
    }
  });

  it('reports no failures through generate() while the gate is off', async () => {
    const { failures } = await generate(REPO_ROOT);
    expect(CONTRAST_GATE).toBe(false);
    expect(failures).toEqual([]);
  });
});
