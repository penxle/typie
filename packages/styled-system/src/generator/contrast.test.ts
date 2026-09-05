import { describe, expect, it } from 'vitest';
import { compositeOver, contrastApca, contrastWcag, normalizeHex, oklchComment, withAlpha } from './color.ts';
import { CONTRAST_PAIRS, failingPairs, FLOORS, measurePreset, renderContrastReport, SELECTION_ALPHA } from './contrast.ts';
import { EDITOR_KEYS, UI_TOKENS } from './schema.ts';
import type { Preset } from './schema.ts';

const fill = (keys: readonly string[], hex: string) => Object.fromEntries(keys.map((key) => [key, hex]));

describe('color', () => {
  it('normalizes css colors to lowercase hex and keeps alpha as eight digits', () => {
    expect(normalizeHex('#FFF')).toBe('#ffffff');
    expect(normalizeHex('oklch(0.210 0.008 280)')).toBe('#17181c');
    expect(normalizeHex('oklch(0 0 0 / 32%)')).toBe('#00000052');
  });

  it('throws with the offending value when a color cannot be parsed', () => {
    expect(() => normalizeHex('not-a-color')).toThrow('not-a-color');
  });

  it('renders an oklch comment with three decimals and integer hue', () => {
    expect(oklchComment('#17181c')).toMatch(/^oklch\(0\.2\d\d 0\.00\d 2\d\d\)$/);
    expect(oklchComment('#00000052')).toMatch(/ \/ 32%\)$/);
  });

  it('composites alpha over a background', () => {
    expect(compositeOver('#ff000080', '#000000')).toBe('#800000');
    expect(compositeOver('#ff0000', '#000000')).toBe('#ff0000');
  });

  it('wires wcag and apca to the libraries', () => {
    expect(contrastWcag('#000000', '#ffffff')).toBeCloseTo(21, 1);
    expect(contrastApca('#000000', '#ffffff')).toBeGreaterThan(100);
    expect(contrastApca('#ffffff', '#000000')).toBeGreaterThan(100);
  });

  it('attaches an alpha as eight-digit hex', () => {
    expect(withAlpha('#ff0000', 0.3)).toBe('#ff00004d');
    expect(withAlpha('#ff0000', 1)).toBe('#ff0000ff');
  });
});

describe('contrast', () => {
  const dark = '#000000';
  const light = '#ffffff';
  const preset: Preset = {
    id: 'light-white',
    mode: 'light',
    label: 'x',
    ui: {
      ...fill(UI_TOKENS, dark),
      'surface.canvas': light,
      'surface.default': light,
      'surface.inset': light,
      'surface.hover': light,
      'surface.active': light,
      'accent.subtle': light,
      'danger.subtle': light,
      'success.subtle': light,
      'warning.subtle': light,
      'text.on.inverse': light,
      'text.on.danger': light,
      'text.on.success': light,
      'text.on.warning': light,
    } as Preset['ui'],
    editor: {
      ...fill(EDITOR_KEYS, dark),
      ...fill(
        [
          'bg.gray',
          'bg.red',
          'bg.orange',
          'bg.yellow',
          'bg.green',
          'bg.blue',
          'bg.purple',
          'selection',
          'ui.search-match',
          'ui.search-match-active',
          'ui.comment-highlight',
          'ui.comment-highlight-active',
        ],
        light,
      ),
    } as Preset['editor'],
  };

  it('defines eighty-two pairs across thirteen classes', () => {
    expect(CONTRAST_PAIRS).toHaveLength(82);
    const counts: Record<string, number> = {};
    for (const pair of CONTRAST_PAIRS) counts[pair.kind] = (counts[pair.kind] ?? 0) + 1;
    expect(counts).toEqual({
      body: 3,
      state: 2,
      'state-muted': 2,
      muted: 3,
      hint: 3,
      inverse: 1,
      signal: 3,
      status: 6,
      on: 8,
      review: 2,
      boundary: 20,
      'editor-text': 17,
      'editor-bg': 12,
    });
  });

  it('pins the floor of every class', () => {
    expect(FLOORS).toEqual({
      body: { wcag: 4.5, apca: 75 },
      state: { wcag: 4.5, apca: 60 },
      'state-muted': { wcag: 3, apca: 45 },
      muted: { wcag: 4.5, apca: 60 },
      hint: { wcag: 3, apca: 45 },
      inverse: { wcag: 4.5, apca: 60 },
      signal: { wcag: 4.5, apca: 60 },
      status: { wcag: 4.5, apca: 60 },
      on: { wcag: 4.5, apca: 60 },
      review: { wcag: 4.5, apca: 60 },
      boundary: { wcag: 3, apca: 0 },
      'editor-text': { wcag: 3, apca: 45 },
      'editor-bg': { wcag: 4.5, apca: 60 },
    });
    expect(SELECTION_ALPHA).toBe(0.3);
  });

  it('measures every pair of a black-on-white preset as passing', () => {
    const rows = measurePreset(preset);
    expect(rows).toHaveLength(82);
    expect(rows.every((row) => row.pass)).toBe(true);
    expect(failingPairs([preset])).toEqual([]);
  });

  it('composites the selection at the renderer alpha before measuring', () => {
    const row = measurePreset(preset).find((candidate) => candidate.pair.bg.key === 'selection');
    expect(row?.bg).toBe(compositeOver(withAlpha(light, SELECTION_ALPHA), light));
  });

  it('reports a failing pair with its preset and keys', () => {
    const faint = { ...preset, ui: { ...preset.ui, 'text.hint': '#dddddd' } };
    expect(failingPairs([faint])).toContain('light-white hint: text.hint on surface.canvas');
  });

  it('renders a markdown report whose legend and gate sentence derive from the tables', () => {
    const report = renderContrastReport([preset]);
    expect(report).toContain('## light-white');
    expect(report).toContain('| light-white | 82 | 0 |');
    expect(report).toContain('body WCAG 4.5 and APCA Lc 75');
    expect(report).toContain('editor-bg WCAG 4.5 and APCA Lc 60');
    expect(report).toContain('Gate: off. Floors are reported, not enforced.');
  });
});
