import { describe, expect, it } from 'vitest';
import { assembleThemeJson, projectEditorColors } from './project.ts';
import { EDITOR_KEYS, UI_TOKENS } from './schema.ts';
import type { Preset } from './schema.ts';

const fill = (keys: readonly string[], hex: string) => Object.fromEntries(keys.map((key) => [key, hex]));

const byCodeUnit = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

const preset: Preset = {
  id: 'light-snow',
  mode: 'light',
  label: 'x',
  ui: {
    ...fill(UI_TOKENS, '#111111'),
    'text.default': '#010101',
    'text.muted': '#020202',
    'border.default': '#030303',
    'surface.inset': '#040404',
  } as Preset['ui'],
  editor: {
    ...fill(EDITOR_KEYS, '#222222'),
    'text.blue': '#0000ff',
    'text.green': '#00ff00',
    'text.amber': '#ffbf00',
    'text.red': '#ff0000',
  } as Preset['editor'],
};

describe('projectEditorColors', () => {
  it('adds the eight projected keys and keeps the 37 content keys', () => {
    const colors = projectEditorColors(preset);
    expect(Object.keys(colors)).toHaveLength(45);
    expect(colors['ui.text.default']).toBe('#010101');
    expect(colors['ui.text.muted']).toBe('#020202');
    expect(colors['ui.border.default']).toBe('#030303');
    expect(colors['ui.surface.muted']).toBe('#040404');
    expect(colors['ui.callout.info']).toBe('#0000ff');
    expect(colors['ui.callout.success']).toBe('#00ff00');
    expect(colors['ui.callout.warning']).toBe('#ffbf00');
    expect(colors['ui.callout.danger']).toBe('#ff0000');
    expect(colors['ui.accent']).toBeUndefined();
  });

  it('sorts keys by code unit', () => {
    const keys = Object.keys(projectEditorColors(preset));
    expect(keys).toEqual([...keys].toSorted(byCodeUnit));
  });
});

describe('assembleThemeJson', () => {
  it('keeps the four-layer container with empty shared layers', () => {
    const json = assembleThemeJson([preset]);
    expect(json.shared).toEqual({});
    expect(json.lightShared).toEqual({});
    expect(json.darkShared).toEqual({});
    expect(Object.keys(json.variants)).toEqual(['light-snow']);
  });
});
