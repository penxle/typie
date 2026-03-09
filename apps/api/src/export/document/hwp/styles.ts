// spell-checker:words HWPUNIT
import { resolveFontEntry } from '../font';
import { resolveColorToHex } from '../theme';
import { hexToColorref } from './records';
import type { CharShapeEntry, ParaShapeEntry } from './doc-info';
import type { HwpConvertContext, InlineSegment, Style } from './types';

const BLACK_COLORREF = 0x00_00_00_00;

export function resolveCharShape(styles: Style[], ctx: HwpConvertContext): number {
  let familyName: string | undefined;
  let weight = 400;
  let hasExplicitWeight = false;
  let baseSize = ctx.defaultFontSizePt100;
  let bold = false;
  let italic = false;
  let underline = false;
  let strikethrough = false;
  let textColor = BLACK_COLORREF;
  let shadeColor = 0xff_ff_ff_ff;
  let letterSpacing = 0;

  for (const style of styles) {
    switch (style.type) {
      case 'bold': {
        bold = true;
        if (!hasExplicitWeight) weight = 700;
        break;
      }
      case 'italic': {
        italic = true;
        break;
      }
      case 'underline': {
        underline = true;
        break;
      }
      case 'strikethrough': {
        strikethrough = true;
        break;
      }
      case 'font_size': {
        baseSize = style.size;
        break;
      }
      case 'font_family': {
        familyName = style.family;
        break;
      }
      case 'font_weight': {
        weight = style.weight;
        hasExplicitWeight = true;
        bold = style.weight >= 600;
        break;
      }
      case 'text_color': {
        const hex = resolveColorToHex(`text.${style.color}`);
        if (hex) textColor = hexToColorref(hex);
        break;
      }
      case 'background_color': {
        const hex = resolveColorToHex(`bg.${style.color}`);
        if (hex) shadeColor = hexToColorref(hex);
        break;
      }
      case 'letter_spacing': {
        // em × 100 → HWP 자간 (-50 ~ +50 %)
        letterSpacing = Math.round(style.spacing);
        break;
      }
    }
  }

  const family = familyName ?? ctx.defaultFamilyName;
  const resolved = resolveFontEntry(ctx.fontNameMap, family, weight);
  const fontId = ctx.tables.fonts.intern(
    { name: resolved?.faceName ?? family, postScriptName: resolved?.faceDefault ?? family },
    resolved?.postScriptName ?? family,
  );

  const entry: CharShapeEntry = {
    fontId,
    baseSize,
    bold,
    italic,
    underline,
    strikethrough,
    textColor,
    underlineColor: textColor,
    shadeColor,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: textColor,
    letterSpacing,
  };
  const key = `${fontId}:${baseSize}:${bold}:${italic}:${underline}:${strikethrough}:${textColor}:${shadeColor}:${letterSpacing}`;
  return ctx.tables.charShapes.intern(entry, key);
}

export function mapAlignment(align: string): number {
  switch (align) {
    case 'justify': {
      return 0;
    }
    case 'left': {
      return 1;
    }
    case 'right': {
      return 2;
    }
    case 'center': {
      return 3;
    }
    default: {
      return 0;
    }
  }
}

/** HWPUNIT 단위로 텍스트 너비 추정 (가장 긴 줄 기준) */
export function estimateTextWidthHwp(paragraphs: { segments: InlineSegment[] }[], fontSizeHwp: number): number {
  let maxLineWidth = 0;
  for (const p of paragraphs) {
    let lineWidth = 0;
    for (const seg of p.segments) {
      for (const ch of seg.text) {
        if (ch === '\n') {
          maxLineWidth = Math.max(maxLineWidth, lineWidth);
          lineWidth = 0;
        } else {
          const code = ch.codePointAt(0) ?? 0;
          // CJK / 한글 / 전각: 1em, 그 외: 0.5em
          const isCjk =
            (code >= 0x30_00 && code <= 0x9f_ff) ||
            (code >= 0xac_00 && code <= 0xd7_af) ||
            (code >= 0xf9_00 && code <= 0xfa_ff) ||
            (code >= 0xff_00 && code <= 0xff_ef);
          lineWidth += isCjk ? fontSizeHwp : Math.floor(fontSizeHwp * 0.55);
        }
      }
    }
    maxLineWidth = Math.max(maxLineWidth, lineWidth);
  }
  return maxLineWidth;
}

export function resolveParaShape(
  ctx: HwpConvertContext,
  opts: {
    align?: string;
    lineHeight?: number;
    indent?: number;
    spaceBefore?: number;
    spaceAfter?: number;
    headType?: number;
    headLevel?: number;
    numberingId?: number;
  },
): number {
  const alignment = opts.align ? mapAlignment(opts.align) : 0;
  const lineHeight = opts.lineHeight ?? ctx.defaultLineHeight;
  const lineSpacing = lineHeight;
  const entry: ParaShapeEntry = {
    alignment,
    lineSpacingType: 0,
    lineSpacing,
    spaceBefore: opts.spaceBefore ?? 0,
    spaceAfter: opts.spaceAfter ?? ctx.blockGapHwp,
    indent: opts.indent ?? 0,
    leftMargin: 0,
    rightMargin: 0,
    headType: opts.headType ?? 0,
    headLevel: opts.headLevel ?? 0,
    numberingId: opts.numberingId ?? 0,
  };
  const key = `${alignment}:${lineSpacing}:${entry.spaceBefore}:${entry.spaceAfter}:${entry.indent}:${entry.headType}:${entry.headLevel}:${entry.numberingId}`;
  return ctx.tables.paraShapes.intern(entry, key);
}

/** base charShape의 50% 크기로 Ruby charShape 생성 */
export function resolveRubyCharShape(baseCharShapeId: number, ctx: HwpConvertContext): number {
  const allCharShapes = ctx.tables.charShapes.getAll() as CharShapeEntry[];
  const baseEntry = allCharShapes[baseCharShapeId];
  const baseSize = baseEntry?.baseSize ?? ctx.defaultFontSizePt100;
  const fontId = baseEntry?.fontId ?? ctx.defaultFontId;
  const rubySize = Math.round(baseSize * 0.5);
  const entry: CharShapeEntry = {
    fontId,
    baseSize: rubySize,
    bold: false,
    italic: false,
    underline: false,
    strikethrough: false,
    textColor: 0x00_00_00_00,
    underlineColor: 0x00_00_00_00,
    shadeColor: 0xff_ff_ff_ff,
    shadowColor: 0x00_b2_b2_b2,
    strikethroughColor: 0x00_00_00_00,
    letterSpacing: 0,
  };
  return ctx.tables.charShapes.intern(entry, `ruby:${fontId}:${rubySize}`);
}
