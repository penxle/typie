import { findFontFamily, nearestWeight } from '../../core/fonts.ts';
import { hexToColorref } from '../records.ts';
import type { RunStyle } from '../../core/v2/types.ts';
import type { CharShapeEntry } from '../doc-info.ts';
import type { HwpConvertContext } from '../types.ts';

const BLACK_COLORREF = 0x00_00_00_00;

export function resolveCharShapeV2(style: RunStyle, ctx: HwpConvertContext): number {
  const baseSize = style.fontSizePt100;
  const bold = style.fontWeight >= 600;
  const textColor = style.textColorHex ? hexToColorref(style.textColorHex) : BLACK_COLORREF;
  const shadeColor = style.backgroundColorHex ? hexToColorref(style.backgroundColorHex) : 0xff_ff_ff_ff;
  const letterSpacing = Math.round(style.letterSpacing);

  const family = style.fontFamily || ctx.defaultFamilyName;
  const fam = findFontFamily(ctx.fonts, family);
  const font = fam ? nearestWeight(fam.weights, style.fontWeight) : undefined;
  const fontId = ctx.tables.fonts.intern(
    { name: font?.localizedName ?? font?.name ?? family, postScriptName: font?.name ?? family },
    font?.postScriptName ?? family,
  );

  const entry: CharShapeEntry = {
    fontId,
    baseSize,
    bold,
    italic: style.italic,
    underline: style.underline,
    strikethrough: style.strikethrough,
    textColor,
    underlineColor: textColor,
    shadeColor,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: textColor,
    letterSpacing,
  };
  const key = `${fontId}:${baseSize}:${bold}:${style.italic}:${style.underline}:${style.strikethrough}:${textColor}:${shadeColor}:${letterSpacing}`;
  return ctx.tables.charShapes.intern(entry, key);
}
