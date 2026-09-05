import * as prettier from 'prettier';
import { oklchComment } from './color.ts';
import { renderContrastReport } from './contrast.ts';
import { assembleThemeJson } from './project.ts';
import { UI_TOKENS } from './schema.ts';
import type { ThemeSource } from './load.ts';
import type { Preset } from './schema.ts';

const capitalize = (value: string) => `${value[0].toUpperCase()}${value.slice(1)}`;
const variantName = (id: string) => id.slice(id.indexOf('-') + 1);

export const conditionKey = (id: string): string => id.replaceAll(/-([a-z])/g, (_, letter: string) => letter.toUpperCase());
export const autoConditionKey = (id: string): string => `auto${capitalize(conditionKey(id))}`;
export const variantSelector = (id: string): string => {
  const mode = id.startsWith('light-') ? 'light' : 'dark';
  return `[data-theme="${mode}"][data-variant-${mode}="${variantName(id)}"] &`;
};
export const autoSelector = (id: string): string => {
  const mode = id.startsWith('light-') ? 'light' : 'dark';
  return `[data-theme="auto"][data-variant-${mode}="${variantName(id)}"] &`;
};

const defaults = ({ roster, presets }: ThemeSource): { light: Preset; dark: Preset } => {
  const light = presets.find((preset) => preset.id === roster.light[0]);
  const dark = presets.find((preset) => preset.id === roster.dark[0]);
  if (!light || !dark) throw new Error('roster must list at least one light and one dark preset');
  return { light, dark };
};

export const emitSemanticColors = (source: ThemeSource): string => {
  const { light, dark } = defaults(source);
  const lines = [
    "import { defineSemanticTokens } from '@pandacss/dev';",
    '',
    'export const semanticColors = defineSemanticTokens.colors({',
  ];
  for (const token of UI_TOKENS) {
    lines.push(`  '${token}': {`, '    value: {');
    const push = (key: string, hex: string, media?: 'osLight' | 'osDark') => {
      lines.push(`      ${key}: ${media ? `{ _${media}: '${hex}' }` : `'${hex}'`}, // ${oklchComment(hex)}`);
    };
    push('base', light.ui[token]);
    push('_dark', dark.ui[token]);
    for (const preset of source.presets) push(`_${conditionKey(preset.id)}`, preset.ui[token]);
    for (const preset of source.presets)
      push(`_${autoConditionKey(preset.id)}`, preset.ui[token], preset.mode === 'light' ? 'osLight' : 'osDark');
    lines.push('    },', '  },');
  }
  lines.push('});', '');
  return lines.join('\n');
};

export const emitConditions = (source: ThemeSource): string => {
  const lines = ['export const variantConditions = {'];
  for (const preset of source.presets) lines.push(`  ${conditionKey(preset.id)}: '${variantSelector(preset.id)}',`);
  for (const preset of source.presets) lines.push(`  ${autoConditionKey(preset.id)}: '${autoSelector(preset.id)}',`);
  lines.push('};', '');
  return lines.join('\n');
};

const tuple = (values: string[]) => `[${values.map((value) => `'${value}'`).join(', ')}] as const`;

export const emitPresets = (source: ThemeSource): string => {
  const { presets } = source;
  const quoted = presets.find((preset) => preset.label.includes("'"));
  if (quoted) throw new Error(`${quoted.id}: label must not contain a single quote`);
  const names = (mode: 'light' | 'dark') => presets.filter((preset) => preset.mode === mode).map((preset) => variantName(preset.id));
  const lines = [
    `export const LIGHT_VARIANTS = ${tuple(names('light'))};`,
    `export const DARK_VARIANTS = ${tuple(names('dark'))};`,
    '',
    'export type LightVariant = (typeof LIGHT_VARIANTS)[number];',
    'export type DarkVariant = (typeof DARK_VARIANTS)[number];',
    'export type ThemeVariant = `light-${LightVariant}` | `dark-${DarkVariant}`;',
    '',
    `export const DEFAULT_LIGHT_VARIANT: LightVariant = '${variantName(source.roster.light[0])}';`,
    `export const DEFAULT_DARK_VARIANT: DarkVariant = '${variantName(source.roster.dark[0])}';`,
    '',
    'export const VARIANT_LABELS: Record<ThemeVariant, string> = {',
    ...presets.map((preset) => `  '${preset.id}': '${preset.label}',`),
    '};',
    '',
    'export const VARIANT_CANVAS: Record<ThemeVariant, string> = {',
    ...presets.map((preset) => `  '${preset.id}': '${preset.ui['surface.canvas']}',`),
    '};',
    '',
    'export const VARIANT_SELECTION: Record<ThemeVariant, string> = {',
    ...presets.map((preset) => `  '${preset.id}': '${preset.editor.selection}',`),
    '};',
    '',
  ];
  return lines.join('\n');
};

const MIT_TEXT = [
  'Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:',
  '',
  'The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.',
  '',
  'THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.',
];

export const emitNotices = (source: ThemeSource): string => {
  const ported = source.presets.filter((preset) => preset.source);
  const lines = [
    '# Third-party palette notices',
    '',
    'Generated by `pnpm --filter @typie/styled-system run generate` from the `source` blocks in `assets/themes/*.json`.',
    '',
  ];
  if (ported.length === 0) {
    lines.push('No third-party palettes are in use.', '');
    return lines.join('\n');
  }
  for (const preset of ported) {
    const info = preset.source as NonNullable<Preset['source']>;
    lines.push(
      `## ${info.name}`,
      '',
      `- Preset: ${preset.id}`,
      `- Repository: ${info.repo}`,
      `- Palette source: ${info.paletteSource}`,
      `- License: ${info.license}`,
      `- Copyright: ${info.copyright}`,
      ...(info.notes ? [`- Notes: ${info.notes}`] : []),
      '',
    );
  }
  lines.push('## MIT License', '', ...MIT_TEXT, '');
  return lines.join('\n');
};

export const emitThemeJson = (source: ThemeSource): string => `${JSON.stringify(assembleThemeJson(source.presets), null, 2)}\n`;

export const emitContrastReport = (source: ThemeSource): string => renderContrastReport(source.presets);

export const formatOutput = async (content: string, filepath: string): Promise<string> => {
  const config = await prettier.resolveConfig(filepath);
  return prettier.format(content, { ...config, filepath });
};
