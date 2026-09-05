import { EDITOR_KEYS, PROJECTED_KEYS } from './schema.ts';
import type { Preset } from './schema.ts';

const byCodeUnit = ([a]: [string, string], [b]: [string, string]) => (a < b ? -1 : a > b ? 1 : 0);

export const projectEditorColors = (preset: Preset): Record<string, string> => {
  const entries: [string, string][] = EDITOR_KEYS.map((key) => [key, preset.editor[key]]);
  for (const [target, source] of Object.entries(PROJECTED_KEYS)) {
    entries.push([target, source.from === 'ui' ? preset.ui[source.key] : preset.editor[source.key]]);
  }
  return Object.fromEntries(entries.toSorted(byCodeUnit));
};

export type ThemeJson = {
  shared: Record<string, string>;
  lightShared: Record<string, string>;
  darkShared: Record<string, string>;
  variants: Record<string, Record<string, string>>;
};

export const assembleThemeJson = (presets: Preset[]): ThemeJson => ({
  shared: {},
  lightShared: {},
  darkShared: {},
  variants: Object.fromEntries(presets.map((preset) => [preset.id, projectEditorColors(preset)])),
});
