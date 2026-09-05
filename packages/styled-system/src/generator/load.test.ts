import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { checkRoster, loadThemes, parsePreset } from './load.ts';
import { EDITOR_KEYS, OPAQUE_UI_TOKENS, UI_TOKENS } from './schema.ts';
import type { Preset } from './schema.ts';

const fill = (keys: readonly string[], hex: string) => Object.fromEntries(keys.map((key) => [key, hex]));

const raw = (id: string, overrides: Record<string, unknown> = {}) => ({
  id,
  mode: id.startsWith('light-') ? 'light' : 'dark',
  label: 'x',
  source: { name: 'n', repo: 'r', license: 'MIT', copyright: 'c', paletteSource: 'p' },
  ui: fill(UI_TOKENS, '#112233'),
  editor: fill(EDITOR_KEYS, '#445566'),
  ...overrides,
});

describe('parsePreset', () => {
  it('accepts a complete preset', () => {
    const preset = parsePreset(raw('light-white', { source: undefined }), 'light-white.json');
    expect(preset.id).toBe('light-white');
    expect(preset.mode).toBe('light');
    expect(Object.keys(preset.ui)).toHaveLength(40);
    expect(Object.keys(preset.editor)).toHaveLength(41);
    expect(preset.source).toBeUndefined();
    expect(parsePreset(raw('light-snow'), 'light-snow.json').source?.name).toBe('n');
  });

  it('rejects an id that differs from the file name', () => {
    expect(() => parsePreset(raw('light-snow'), 'light-butter.json')).toThrow(/id: expected light-butter/);
  });

  it('rejects a mode that does not match the id prefix', () => {
    expect(() => parsePreset(raw('light-snow', { mode: 'dark' }), 'light-snow.json')).toThrow(/mode: dark does not match/);
  });

  it('rejects a missing ui key', () => {
    const ui: Record<string, string> = fill(UI_TOKENS, '#112233');
    delete ui['surface.hover'];
    expect(() => parsePreset(raw('dark-navy', { ui }), 'dark-navy.json')).toThrow(/ui: missing surface\.hover/);
  });

  it('rejects an unexpected editor key', () => {
    const editor = { ...fill(EDITOR_KEYS, '#445566'), 'ui.accent': '#000000' };
    expect(() => parsePreset(raw('dark-navy', { editor }), 'dark-navy.json')).toThrow(/editor: unexpected ui\.accent/);
  });

  it('rejects a non-hex value and uppercase hex', () => {
    const ui = { ...fill(UI_TOKENS, '#112233'), scrim: 'oklch(0 0 0 / 32%)' };
    expect(() => parsePreset(raw('dark-navy', { ui }), 'dark-navy.json')).toThrow(/ui\.scrim: hex expected/);
    const upper = { ...fill(UI_TOKENS, '#112233'), scrim: '#ABCDEF' };
    expect(() => parsePreset(raw('dark-navy', { ui: upper }), 'dark-navy.json')).toThrow(/ui\.scrim: hex expected/);
  });

  it('accepts eight-digit hex', () => {
    const ui = { ...fill(UI_TOKENS, '#112233'), scrim: '#11223380' };
    expect(parsePreset(raw('dark-navy', { ui }), 'dark-navy.json').ui.scrim).toBe('#11223380');
  });

  it('rejects eight-digit hex on keys that leave css', () => {
    const editor = { ...fill(EDITOR_KEYS, '#445566'), selection: '#99ccff80' };
    expect(() => parsePreset(raw('light-snow', { editor }), 'light-snow.json')).toThrow(/editor\.selection: six-digit hex expected/);
    const ui = { ...fill(UI_TOKENS, '#112233'), 'surface.canvas': '#f9fafd80', 'text.default': '#17181c80' };
    expect(() => parsePreset(raw('light-snow', { ui }), 'light-snow.json')).toThrow(/ui\.surface\.canvas: six-digit hex expected/);
    expect(() => parsePreset(raw('light-snow', { ui }), 'light-snow.json')).toThrow(/ui\.text\.default: six-digit hex expected/);
  });

  it('lists exactly the five opaque ui tokens', () => {
    const sorted = [...OPAQUE_UI_TOKENS].toSorted((left, right) => left.localeCompare(right));
    expect(sorted).toEqual(['border.default', 'surface.canvas', 'surface.inset', 'text.default', 'text.muted']);
  });

  it('rejects an empty label and an unknown field', () => {
    expect(() => parsePreset(raw('light-snow', { label: ' ' }), 'light-snow.json')).toThrow(/label/);
    expect(() => parsePreset(raw('light-snow', { profile: 'standard' }), 'light-snow.json')).toThrow(/unexpected field profile/);
    expect(() => parsePreset(raw('light-snow', { hidden: false }), 'light-snow.json')).toThrow(/unexpected field hidden/);
  });

  it('requires a complete source block on ported presets and allows omitting it on house presets', () => {
    expect(() => parsePreset(raw('light-white'), 'light-white.json')).not.toThrow();
    expect(() => parsePreset(raw('dark-black'), 'dark-black.json')).not.toThrow();
    expect(() => parsePreset(raw('light-white', { source: undefined }), 'light-white.json')).not.toThrow();
    expect(() => parsePreset(raw('dark-black', { source: undefined }), 'dark-black.json')).not.toThrow();
    const source = { name: 'n', repo: 'r', license: 'MIT', copyright: 'c', paletteSource: 'p' };
    expect(parsePreset(raw('light-snow', { source }), 'light-snow.json').source).toEqual(source);
    expect(parsePreset(raw('light-snow', { source: { ...source, notes: 'x' } }), 'light-snow.json').source?.notes).toBe('x');
    expect(() => parsePreset(raw('light-snow', { source: undefined }), 'light-snow.json')).toThrow(/source: required for ported presets/);
    expect(() => parsePreset(raw('light-snow', { source: { ...source, license: 'Apache-2.0' } }), 'light-snow.json')).toThrow(
      /source\.license: MIT expected/,
    );
    expect(() => parsePreset(raw('light-snow', { source: { ...source, copyright: ' ' } }), 'light-snow.json')).toThrow(/source\.copyright/);
    expect(() => parsePreset(raw('light-snow', { source: { ...source, extra: 1 } }), 'light-snow.json')).toThrow(
      /source: unexpected extra/,
    );
  });
});

describe('checkRoster', () => {
  const presets = ['light-white', 'light-snow', 'dark-black'].map((id) => parsePreset(raw(id), `${id}.json`)) as Preset[];

  it('accepts a roster that matches the preset set', () => {
    expect(checkRoster({ light: ['light-white', 'light-snow'], dark: ['dark-black'] }, presets)).toEqual({
      light: ['light-white', 'light-snow'],
      dark: ['dark-black'],
    });
  });

  it('rejects a listed id without a file', () => {
    expect(() => checkRoster({ light: ['light-white', 'light-snow', 'light-mint'], dark: ['dark-black'] }, presets)).toThrow(
      /roster lists light-mint but light-mint\.json is missing/,
    );
  });

  it('rejects a file that the roster does not list', () => {
    expect(() => checkRoster({ light: ['light-white'], dark: ['dark-black'] }, presets)).toThrow(/light-snow\.json exists/);
  });

  it('rejects duplicates and wrong-mode placement', () => {
    expect(() => checkRoster({ light: ['light-white', 'light-white', 'light-snow'], dark: ['dark-black'] }, presets)).toThrow(
      /roster has duplicate ids/,
    );
    expect(() => checkRoster({ light: ['light-white', 'light-snow', 'dark-black'], dark: [] }, presets)).toThrow(
      /roster\.light contains dark-black/,
    );
  });
});

describe('loadThemes', () => {
  let dir: string;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'themes-'));
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  const seed = (ids: string[], roster: unknown) => {
    for (const id of ids) {
      fs.writeFileSync(path.join(dir, `${id}.json`), JSON.stringify(raw(id)));
    }
    fs.writeFileSync(path.join(dir, 'roster.json'), JSON.stringify(roster));
  };

  it('returns presets in roster order rather than file name order', () => {
    seed(['light-white', 'light-butter', 'dark-black'], { light: ['light-white', 'light-butter'], dark: ['dark-black'] });
    const source = loadThemes(dir);
    expect(source.presets.map((preset) => preset.id)).toEqual(['light-white', 'light-butter', 'dark-black']);
    expect(source.roster).toEqual({ light: ['light-white', 'light-butter'], dark: ['dark-black'] });
  });

  it('rejects a roster whose light is not an array', () => {
    seed(['light-white', 'dark-black'], { light: 'light-white', dark: ['dark-black'] });
    expect(() => loadThemes(dir)).toThrow(/roster\.json: \{ light: string\[\], dark: string\[\] \} expected/);
  });
});
