import { resolveColorToHex } from '../theme.ts';
import type { Modifier, ModifierType } from '@typie/editor-ffi/server';
import type { RunStyle } from './types.ts';

type Mods = Record<ModifierType, Modifier>;
function get<T extends ModifierType>(mods: Mods, type: T): Extract<Modifier, { type: T }> | undefined {
  return mods[type] as Extract<Modifier, { type: T }> | undefined;
}
export function resolveRunStyle(mods: Mods, defaults: { fontFamily: string; fontSizePt100: number }): RunStyle {
  const fontFamily = get(mods, 'font_family')?.value ?? defaults.fontFamily;
  const fontSizePt100 = get(mods, 'font_size')?.value ?? defaults.fontSizePt100;
  const bold = mods['bold'] != null;
  const explicitWeight = get(mods, 'font_weight')?.value;
  const fontWeight = explicitWeight ?? (bold ? 700 : 400);
  const textKey = get(mods, 'text_color')?.value;
  const bgKey = get(mods, 'background_color')?.value;
  return {
    bold,
    italic: mods['italic'] != null,
    underline: mods['underline'] != null,
    strikethrough: mods['strikethrough'] != null,
    fontFamily,
    fontSizePt100,
    fontWeight,
    textColorHex: textKey ? resolveColorToHex(`text.${textKey}`) : undefined,
    backgroundColorHex: bgKey ? resolveColorToHex(`bg.${bgKey}`) : undefined,
    letterSpacing: get(mods, 'letter_spacing')?.value ?? 0,
    link: get(mods, 'link')?.href,
    ruby: get(mods, 'ruby')?.text,
  };
}
