import { ExternalHyperlink, PageBreak, TextRun } from 'docx';
import { findFontFamily, nearestWeight } from '../../core/fonts.ts';
import { createRubyRun } from '../ruby.ts';
import type { XmlComponent } from 'docx';
import type { ExportFontFamily } from '../../core/types.ts';
import type { ParagraphV2, RunStyle } from '../../core/v2/types.ts';

export type TextConvertContextV2 = {
  fonts: ExportFontFamily[];
  defaultFamilyName: string;
  defaultColor?: string;
};

const fontSizeToHalfPoints = (size: number): number => Math.round((size / 100) * 2);

const letterSpacingToTwips = (spacing: number, fontSizePt100: number): number => {
  const fontSizePt = fontSizePt100 / 100;
  return Math.round((spacing / 100) * fontSizePt * 20);
};

export function buildRunOptionsV2(style: RunStyle, ctx: TextConvertContextV2): Record<string, unknown> {
  const fam = findFontFamily(ctx.fonts, style.fontFamily);
  const font = fam ? nearestWeight(fam.weights, style.fontWeight) : undefined;

  const opts: Record<string, unknown> = {
    size: fontSizeToHalfPoints(style.fontSizePt100),
    font: font?.postScriptName ?? style.fontFamily,
  };

  if (style.italic) opts.italics = true;
  if (style.underline) opts.underline = {};
  if (style.strikethrough) opts.strike = true;
  if (style.fontWeight >= 600) opts.bold = true;
  if (style.textColorHex) opts.color = style.textColorHex;
  if (style.backgroundColorHex) opts.shading = { fill: style.backgroundColorHex };
  if (style.letterSpacing) opts.characterSpacing = letterSpacingToTwips(style.letterSpacing, style.fontSizePt100);
  if (ctx.defaultColor) opts.color = ctx.defaultColor;

  return opts;
}

export function runsToComponents(p: ParagraphV2, ctx: TextConvertContextV2): (TextRun | ExternalHyperlink | XmlComponent | PageBreak)[] {
  const result: (TextRun | ExternalHyperlink | XmlComponent | PageBreak)[] = [];

  for (const inline of p.inlines) {
    switch (inline.type) {
      case 'run': {
        const { text, style } = inline.run;
        const opts = buildRunOptionsV2(style, ctx);

        if (style.ruby) {
          result.push(createRubyRun(text, style.ruby, opts));
          break;
        }

        const textRun = new TextRun({ ...opts, text });
        if (style.link) {
          result.push(new ExternalHyperlink({ children: [textRun], link: style.link }));
        } else {
          result.push(textRun);
        }
        break;
      }
      case 'hard_break': {
        result.push(new TextRun({ break: 1 }));
        break;
      }
      case 'page_break': {
        result.push(new PageBreak());
        break;
      }
      case 'tab': {
        result.push(new TextRun({ children: ['\t'] }));
        break;
      }
    }
  }

  return result;
}
