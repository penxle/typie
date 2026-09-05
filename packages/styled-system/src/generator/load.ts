import fs from 'node:fs';
import path from 'node:path';
import { EDITOR_KEYS, HEX_PATTERN, HEX6_PATTERN, HOUSE_IDS, OPAQUE_UI_TOKENS, UI_TOKENS } from './schema.ts';
import type { Preset, PresetMode, PresetSource, Roster } from './schema.ts';

const MODES = new Set(['light', 'dark']);
const FIELDS = new Set(['id', 'mode', 'label', 'source', 'ui', 'editor']);
const SOURCE_REQUIRED = ['name', 'repo', 'license', 'copyright', 'paletteSource'] as const;
const SOURCE_FIELDS = new Set<string>([...SOURCE_REQUIRED, 'notes']);
const HOUSE = new Set<string>(HOUSE_IDS);
const OPAQUE_UI = new Set<string>(OPAQUE_UI_TOKENS);
const OPAQUE_EDITOR = new Set<string>(EDITOR_KEYS);

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);
const isText = (value: unknown): value is string => typeof value === 'string' && value.trim() !== '';

const checkColorMap = (value: unknown, expected: readonly string[], opaque: ReadonlySet<string>, field: string, problems: string[]) => {
  if (!isRecord(value)) {
    problems.push(`${field}: object expected`);
    return;
  }
  const keys = new Set(Object.keys(value));
  for (const key of expected) {
    if (!keys.has(key)) problems.push(`${field}: missing ${key}`);
  }
  for (const key of keys) {
    if (!expected.includes(key)) problems.push(`${field}: unexpected ${key}`);
  }
  for (const [key, hex] of Object.entries(value)) {
    if (typeof hex !== 'string' || !HEX_PATTERN.test(hex)) {
      problems.push(`${field}.${key}: hex expected, got ${String(hex)}`);
    } else if (opaque.has(key) && !HEX6_PATTERN.test(hex)) {
      problems.push(`${field}.${key}: six-digit hex expected, got ${hex}`);
    }
  }
};

const checkSource = (value: unknown, id: string, problems: string[]): PresetSource | undefined => {
  if (value === undefined) {
    if (!HOUSE.has(id)) problems.push('source: required for ported presets');
    return undefined;
  }
  if (!isRecord(value)) {
    problems.push('source: object expected');
    return undefined;
  }
  for (const key of SOURCE_REQUIRED) {
    if (!isText(value[key])) problems.push(`source.${key}: non-empty string expected`);
  }
  if (isText(value.license) && value.license !== 'MIT') problems.push('source.license: MIT expected');
  if (value.notes !== undefined && !isText(value.notes)) problems.push('source.notes: non-empty string expected');
  for (const key of Object.keys(value)) {
    if (!SOURCE_FIELDS.has(key)) problems.push(`source: unexpected ${key}`);
  }
  return value as PresetSource;
};

export const parsePreset = (raw: unknown, fileName: string): Preset => {
  if (!isRecord(raw)) throw new Error(`${fileName}: object expected`);
  const problems: string[] = [];
  const expectedId = path.basename(fileName, '.json');
  if (raw.id !== expectedId) problems.push(`id: expected ${expectedId}, got ${String(raw.id)}`);
  if (typeof raw.mode !== 'string' || !MODES.has(raw.mode)) problems.push('mode: light or dark expected');
  else if (!expectedId.startsWith(`${raw.mode}-`)) problems.push(`mode: ${raw.mode} does not match id prefix`);
  if (!isText(raw.label)) problems.push('label: non-empty string expected');
  const source = checkSource(raw.source, expectedId, problems);
  checkColorMap(raw.ui, UI_TOKENS, OPAQUE_UI, 'ui', problems);
  checkColorMap(raw.editor, EDITOR_KEYS, OPAQUE_EDITOR, 'editor', problems);
  for (const key of Object.keys(raw)) {
    if (!FIELDS.has(key)) problems.push(`unexpected field ${key}`);
  }
  if (problems.length > 0) throw new Error(`${fileName}:\n  ${problems.join('\n  ')}`);
  return {
    id: expectedId,
    mode: raw.mode as PresetMode,
    label: raw.label as string,
    ...(source && { source }),
    ui: raw.ui as Preset['ui'],
    editor: raw.editor as Preset['editor'],
  };
};

export const checkRoster = (raw: unknown, presets: Preset[]): Roster => {
  if (!isRecord(raw) || !Array.isArray(raw.light) || !Array.isArray(raw.dark)) {
    throw new Error('roster.json: { light: string[], dark: string[] } expected');
  }
  const roster: Roster = { light: raw.light.map(String), dark: raw.dark.map(String) };
  const listed = [...roster.light, ...roster.dark];
  const ids = new Set(presets.map((preset) => preset.id));
  const problems: string[] = [];
  for (const id of listed) {
    if (!ids.has(id)) problems.push(`roster lists ${id} but ${id}.json is missing`);
  }
  for (const id of ids) {
    if (!listed.includes(id)) problems.push(`${id}.json exists but roster does not list it`);
  }
  if (new Set(listed).size !== listed.length) problems.push('roster has duplicate ids');
  for (const id of roster.light) {
    if (!id.startsWith('light-')) problems.push(`roster.light contains ${id}`);
  }
  for (const id of roster.dark) {
    if (!id.startsWith('dark-')) problems.push(`roster.dark contains ${id}`);
  }
  if (problems.length > 0) throw new Error(`roster.json:\n  ${problems.join('\n  ')}`);
  return roster;
};

export type ThemeSource = { roster: Roster; presets: Preset[] };

const readJson = (file: string): unknown => JSON.parse(fs.readFileSync(file, 'utf8'));

export const loadThemes = (themesDir: string): ThemeSource => {
  const names = fs
    .readdirSync(themesDir)
    .filter((name) => name.endsWith('.json') && name !== 'roster.json')
    .toSorted((left, right) => left.localeCompare(right));
  const presets = names.map((name) => parsePreset(readJson(path.join(themesDir, name)), name));
  const roster = checkRoster(readJson(path.join(themesDir, 'roster.json')), presets);
  const byId = new Map(presets.map((preset) => [preset.id, preset]));
  const ordered = [...roster.light, ...roster.dark].flatMap((id) => {
    const preset = byId.get(id);
    return preset ? [preset] : [];
  });
  return { roster, presets: ordered };
};
