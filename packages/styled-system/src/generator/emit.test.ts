import { describe, expect, it } from 'vitest';
import {
  autoConditionKey,
  autoSelector,
  conditionKey,
  emitConditions,
  emitNotices,
  emitPresets,
  emitSemanticColors,
  variantSelector,
} from './emit.ts';
import { EDITOR_KEYS, UI_TOKENS } from './schema.ts';
import type { Preset, Roster } from './schema.ts';

const fill = (keys: readonly string[], hex: string) => Object.fromEntries(keys.map((key) => [key, hex]));
const source = {
  name: 'Nord',
  repo: 'https://example.invalid/nord',
  license: 'MIT',
  copyright: 'Copyright (c) 2016 Someone',
  paletteSource: 'https://example.invalid/nord.scss',
};
const make = (id: string, ported = false): Preset => ({
  id,
  mode: id.startsWith('light-') ? 'light' : 'dark',
  label: id,
  ...(ported && { source: { ...source, notes: `${id} notes` } }),
  ui: fill(UI_TOKENS, id.startsWith('light-') ? '#fafafa' : '#101010') as Preset['ui'],
  editor: {
    ...fill(EDITOR_KEYS, '#222222'),
    'text.red': '#aa0000',
    'text.yellow': '#aaaa00',
    'text.green': '#00aa00',
    'text.blue': '#0000aa',
  } as Preset['editor'],
});
const presets = [make('light-white'), make('light-catppuccin-latte', true), make('dark-black'), make('dark-rose-pine', true)];
const roster: Roster = { light: ['light-white', 'light-catppuccin-latte'], dark: ['dark-black', 'dark-rose-pine'] };

describe('names', () => {
  it('derives condition keys and selectors from ids with several hyphens', () => {
    expect(conditionKey('light-catppuccin-latte')).toBe('lightCatppuccinLatte');
    expect(autoConditionKey('dark-rose-pine')).toBe('autoDarkRosePine');
    expect(variantSelector('light-catppuccin-latte')).toBe('[data-theme="light"][data-variant-light="catppuccin-latte"] &');
    expect(autoSelector('dark-rose-pine')).toBe('[data-theme="auto"][data-variant-dark="rose-pine"] &');
  });
});

describe('emitSemanticColors', () => {
  const output = emitSemanticColors({ roster, presets });

  it('emits every ui token including surface.active with base, dark, explicit and auto conditions', () => {
    for (const token of UI_TOKENS) expect(output).toContain(`'${token}': {`);
    expect(output).toContain("'surface.active': {");
    expect(output).toContain("base: '#fafafa', // oklch(");
    expect(output).toContain("_dark: '#101010', // oklch(");
    expect(output).toContain("_lightCatppuccinLatte: '#fafafa'");
    expect(output).toContain("_autoDarkRosePine: { _osDark: '#101010' }");
  });
});

describe('emitConditions', () => {
  it('emits one explicit and one auto condition per preset', () => {
    const output = emitConditions({ roster, presets });
    expect(output.match(/^ {2}[a-zA-Z]+: '/gm)).toHaveLength(8);
    expect(output).toContain(`lightCatppuccinLatte: '[data-theme="light"][data-variant-light="catppuccin-latte"] &'`);
    expect(output).toContain(`autoDarkRosePine: '[data-theme="auto"][data-variant-dark="rose-pine"] &'`);
  });
});

describe('emitPresets', () => {
  const output = emitPresets({ roster, presets });

  it('exports the variant tuples, labels, canvas colors and swatches without a hidden concept', () => {
    expect(output).toContain("export const LIGHT_VARIANTS = ['white', 'catppuccin-latte'] as const;");
    expect(output).toContain("export const DARK_VARIANTS = ['black', 'rose-pine'] as const;");
    expect(output).not.toContain('HIDDEN_');
    expect(output).not.toContain('PUBLIC_');
    expect(output).toContain('export type LightVariant = (typeof LIGHT_VARIANTS)[number];');
    expect(output).toContain("export const DEFAULT_DARK_VARIANT: DarkVariant = 'black';");
    expect(output).toContain("'dark-rose-pine': 'dark-rose-pine',");
    expect(output).toContain("'light-catppuccin-latte': '#fafafa',");
    expect(output).toContain('export const VARIANT_LABELS: Record<ThemeVariant, string> = {');
    expect(output).toContain('export const VARIANT_SWATCH: Record<ThemeVariant, readonly [string, string, string, string]> = {');
    expect(output).toContain("'dark-black': ['#aa0000', '#aaaa00', '#00aa00', '#0000aa'],");
    expect(output).toContain('export const VARIANT_SELECTION: Record<ThemeVariant, string> = {');
    expect(output.match(/^export /gm)).toHaveLength(11);
  });
});

describe('emitNotices', () => {
  it('lists every ported preset with its source and the MIT text once, skipping house presets', () => {
    const output = emitNotices({ roster, presets });
    expect(output).toContain('## Nord');
    expect(output.match(/^## Nord$/gm)).toHaveLength(2);
    expect(output).toContain('- Repository: https://example.invalid/nord');
    expect(output).toContain('- Copyright: Copyright (c) 2016 Someone');
    expect(output).toContain('- Notes: dark-rose-pine notes');
    expect(output).not.toContain('light-white');
    expect(output.match(/Permission is hereby granted/g)).toHaveLength(1);
  });

  it('says so when no ported preset exists', () => {
    const output = emitNotices({
      roster: { light: ['light-white'], dark: ['dark-black'] },
      presets: [make('light-white'), make('dark-black')],
    });
    expect(output).toContain('No third-party palettes are in use.');
    expect(output).not.toContain('Permission is hereby granted');
  });
});
