export const UI_TOKENS = [
  'text.default',
  'text.muted',
  'text.hint',
  'text.on.inverse',
  'text.on.danger',
  'text.on.danger.subtle',
  'text.on.success',
  'text.on.success.subtle',
  'text.on.warning',
  'text.on.warning.subtle',
  'surface.canvas',
  'surface.default',
  'surface.inset',
  'surface.hover',
  'surface.active',
  'surface.inverse',
  'border.hairline',
  'border.default',
  'border.emphasis',
  'accent.default',
  'accent.subtle',
  'danger.default',
  'danger.subtle',
  'success.default',
  'success.subtle',
  'warning.default',
  'warning.subtle',
  'review.issue',
  'review.strength',
  'skeleton.base',
  'skeleton.shimmer',
  'scrim',
  'shadow.default',
  'palette.gray',
  'palette.red',
  'palette.orange',
  'palette.yellow',
  'palette.green',
  'palette.blue',
  'palette.purple',
] as const;

export const EDITOR_KEYS = [
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
  'text.bright',
  'text.black',
  'text.darkgray',
  'text.gray',
  'text.lightgray',
  'text.white',
  'bg.gray',
  'bg.red',
  'bg.orange',
  'bg.yellow',
  'bg.green',
  'bg.blue',
  'bg.purple',
  'selection',
  'ui.blockquote.message-sent',
  'ui.blockquote.message-received',
  'ui.callout.info',
  'ui.callout.success',
  'ui.callout.warning',
  'ui.callout.danger',
  'ui.search-match',
  'ui.search-match-active',
  'ui.comment-highlight',
  'ui.comment-highlight-active',
] as const;

export type UiToken = (typeof UI_TOKENS)[number];
export type EditorKey = (typeof EDITOR_KEYS)[number];

export type ProjectionSource = { from: 'ui'; key: UiToken } | { from: 'editor'; key: EditorKey };

export const PROJECTED_KEYS: Record<string, ProjectionSource> = {
  'ui.text.default': { from: 'ui', key: 'text.default' },
  'ui.text.muted': { from: 'ui', key: 'text.muted' },
  'ui.border.default': { from: 'ui', key: 'border.default' },
  'ui.surface.muted': { from: 'ui', key: 'surface.inset' },
};

export type PresetMode = 'light' | 'dark';

export type PresetSource = { name: string; repo: string; license: string; copyright: string; paletteSource: string; notes?: string };

export type Preset = {
  id: string;
  mode: PresetMode;
  label: string;
  source?: PresetSource;
  ui: Record<UiToken, string>;
  editor: Record<EditorKey, string>;
};

export const HOUSE_IDS = ['light-white', 'dark-black'] as const;

export type Roster = { light: string[]; dark: string[] };

export const HEX_PATTERN = /^#[0-9a-f]{6}(?:[0-9a-f]{2})?$/;

export const HEX6_PATTERN = /^#[0-9a-f]{6}$/;

export const OPAQUE_UI_TOKENS: readonly UiToken[] = [
  'surface.canvas',
  ...Object.values(PROJECTED_KEYS).flatMap((source) => (source.from === 'ui' ? [source.key] : [])),
];
