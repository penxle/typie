import { compositeOver, contrastApca, contrastWcag, withAlpha } from './color.ts';
import type { EditorKey, Preset, UiToken } from './schema.ts';

export const CONTRAST_GATE = false;
export const SELECTION_ALPHA = 0.3;

export type PairClass =
  | 'body'
  | 'state'
  | 'state-muted'
  | 'muted'
  | 'hint'
  | 'inverse'
  | 'signal'
  | 'status'
  | 'on'
  | 'review'
  | 'boundary'
  | 'editor-text'
  | 'editor-bg';
export type ColorRef = { from: 'ui'; key: UiToken } | { from: 'editor'; key: EditorKey };
export type ContrastPair = { fg: ColorRef; bg: ColorRef; kind: PairClass; bgAlpha?: number };
export type Floor = { wcag: number; apca: number };

export const FLOORS: Record<PairClass, Floor> = {
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
};

const ui = (key: UiToken): ColorRef => ({ from: 'ui', key });
const editor = (key: EditorKey): ColorRef => ({ from: 'editor', key });
const pair = (fg: ColorRef, bg: ColorRef, kind: PairClass): ContrastPair => ({ fg, bg, kind });

const READING: UiToken[] = ['surface.canvas', 'surface.default', 'surface.inset'];
const STATE: UiToken[] = ['surface.hover', 'surface.active'];
const STATUS: UiToken[] = ['danger.default', 'success.default', 'warning.default'];
const PALETTE: UiToken[] = [
  'palette.gray',
  'palette.red',
  'palette.orange',
  'palette.yellow',
  'palette.green',
  'palette.blue',
  'palette.purple',
];
const HUES: EditorKey[] = [
  'text.red',
  'text.orange',
  'text.amber',
  'text.yellow',
  'text.lime',
  'text.green',
  'text.emerald',
  'text.teal',
  'text.cyan',
  'text.sky',
  'text.blue',
  'text.indigo',
  'text.violet',
  'text.purple',
  'text.fuchsia',
  'text.pink',
  'text.rose',
];
const HIGHLIGHTS: EditorKey[] = [
  'bg.gray',
  'bg.red',
  'bg.orange',
  'bg.yellow',
  'bg.green',
  'bg.blue',
  'bg.purple',
  'ui.search-match',
  'ui.search-match-active',
  'ui.comment-highlight',
  'ui.comment-highlight-active',
];

export const CONTRAST_PAIRS: ContrastPair[] = [
  ...READING.map((bg) => pair(ui('text.default'), ui(bg), 'body')),
  ...STATE.map((bg) => pair(ui('text.default'), ui(bg), 'state')),
  ...STATE.map((bg) => pair(ui('text.muted'), ui(bg), 'state-muted')),
  ...READING.map((bg) => pair(ui('text.muted'), ui(bg), 'muted')),
  ...READING.map((bg) => pair(ui('text.hint'), ui(bg), 'hint')),
  pair(ui('text.on.inverse'), ui('surface.inverse'), 'inverse'),
  ...READING.map((bg) => pair(ui('accent.default'), ui(bg), 'signal')),
  ...STATUS.flatMap((fg) => (['surface.canvas', 'surface.default'] as UiToken[]).map((bg) => pair(ui(fg), ui(bg), 'status'))),
  pair(ui('text.on.danger'), ui('danger.default'), 'on'),
  pair(ui('text.on.success'), ui('success.default'), 'on'),
  pair(ui('text.on.warning'), ui('warning.default'), 'on'),
  pair(ui('text.on.danger.subtle'), ui('danger.subtle'), 'on'),
  pair(ui('text.on.success.subtle'), ui('success.subtle'), 'on'),
  pair(ui('text.on.warning.subtle'), ui('warning.subtle'), 'on'),
  pair(ui('text.default'), ui('accent.subtle'), 'on'),
  pair(ui('surface.default'), ui('accent.default'), 'on'),
  pair(ui('review.issue'), ui('surface.default'), 'review'),
  pair(ui('review.strength'), ui('surface.default'), 'review'),
  ...(['accent.default', ...STATUS] as UiToken[]).flatMap((fg) => READING.map((bg) => pair(ui(fg), ui(bg), 'boundary'))),
  pair(ui('surface.default'), ui('accent.default'), 'boundary'),
  ...PALETTE.map((fg) => pair(ui(fg), ui('surface.default'), 'boundary')),
  ...HUES.map((fg) => pair(editor(fg), ui('surface.default'), 'editor-text')),
  ...HIGHLIGHTS.map((bg) => pair(ui('text.default'), editor(bg), 'editor-bg')),
  { fg: ui('text.default'), bg: editor('selection'), kind: 'editor-bg', bgAlpha: SELECTION_ALPHA },
];

const FLOOR_LEGEND = Object.entries(FLOORS)
  .map(([kind, floor]) => (floor.apca > 0 ? `${kind} WCAG ${floor.wcag} and APCA Lc ${floor.apca}` : `${kind} WCAG ${floor.wcag}`))
  .join(', ');

const GATE_NOTE = CONTRAST_GATE ? 'Gate: on. Every pair must meet its floor.' : 'Gate: off. Floors are reported, not enforced.';

const resolve = (preset: Preset, ref: ColorRef): string => (ref.from === 'ui' ? preset.ui[ref.key] : preset.editor[ref.key]);

export type Measurement = { pair: ContrastPair; fg: string; bg: string; wcag: number; apca: number; pass: boolean };

export const measurePreset = (preset: Preset): Measurement[] =>
  CONTRAST_PAIRS.map((candidate) => {
    const rawBg = resolve(preset, candidate.bg);
    const bg = compositeOver(candidate.bgAlpha ? withAlpha(rawBg, candidate.bgAlpha) : rawBg, preset.ui['surface.default']);
    const fg = compositeOver(resolve(preset, candidate.fg), bg);
    const wcag = contrastWcag(fg, bg);
    const apca = contrastApca(fg, bg);
    const floor = FLOORS[candidate.kind];
    return { pair: candidate, fg, bg, wcag, apca, pass: wcag >= floor.wcag && apca >= floor.apca };
  });

export const failingPairs = (presets: Preset[]): string[] =>
  presets.flatMap((preset) =>
    measurePreset(preset)
      .filter((row) => !row.pass)
      .map((row) => `${preset.id} ${row.pair.kind}: ${row.pair.fg.key} on ${row.pair.bg.key}`),
  );

export const renderContrastReport = (presets: Preset[]): string => {
  const measured = presets.map((preset) => ({ preset, rows: measurePreset(preset) }));
  const lines: string[] = [
    '# Contrast report',
    '',
    `Generated by \`pnpm --filter @typie/styled-system run generate\`. Floors: ${FLOOR_LEGEND}.`,
    '',
    GATE_NOTE,
    '',
    '## Summary',
    '',
    '| Preset | Pairs | Failing |',
    '| --- | --- | --- |',
    ...measured.map(({ preset, rows }) => `| ${preset.id} | ${rows.length} | ${rows.filter((row) => !row.pass).length} |`),
  ];
  for (const { preset, rows } of measured) {
    lines.push(
      '',
      `## ${preset.id}`,
      '',
      '| Pair | Class | Foreground | Background | WCAG | APCA | Result |',
      '| --- | --- | --- | --- | --- | --- | --- |',
    );
    for (const row of rows) {
      lines.push(
        `| ${row.pair.fg.key} on ${row.pair.bg.key} | ${row.pair.kind} | ${row.fg} | ${row.bg} | ${row.wcag.toFixed(2)} | ${row.apca.toFixed(1)} | ${row.pass ? 'pass' : 'fail'} |`,
      );
    }
  }
  return `${lines.join('\n')}\n`;
};
