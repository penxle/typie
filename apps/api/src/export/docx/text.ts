import { ExternalHyperlink, TextRun } from 'docx';
import { findFontFamily, nearestWeight } from '../core/fonts';
import { resolveColorToHex } from '../core/theme';
import type { XmlComponent } from 'docx';
import type { Annotation, ExportFontFamily, Style, TextSegment } from '../core/types';
import type { createRubyRun } from './ruby';

type RubyRunFactory = typeof createRubyRun;

export type TextConvertContext = {
  createRubyRun: RubyRunFactory;
  defaultColor?: string;
  fonts: ExportFontFamily[];
  defaultFamilyName: string;
};

/** Rust font_size (pt × 100) → docx half-points (1pt = 2hp) */
const fontSizeToHalfPoints = (size: number): number => Math.round((size / 100) * 2);

const DEFAULT_FONT_SIZE = 1200; // pt × 100 (12pt)

/** Rust letter_spacing (em × 100) → twips. fontSizePt100: 같은 segment의 font_size 값 (pt × 100) */
const letterSpacingToTwips = (spacing: number, fontSizePt100: number): number => {
  const fontSizePt = fontSizePt100 / 100;
  return Math.round((spacing / 100) * fontSizePt * 20);
};

function buildRunOptions(styles: Style[], ctx: TextConvertContext) {
  const opts: Record<string, unknown> = {};

  // font_size를 먼저 찾아서 letter_spacing 변환에 사용
  const fontSizePt100 = styles.find((s): s is Extract<Style, { type: 'font_size' }> => s.type === 'font_size')?.size ?? DEFAULT_FONT_SIZE;

  let familyName: string | undefined;
  let weight = 400;
  let hasExplicitWeight = false;
  let hasFontStyle = false;

  for (const style of styles) {
    switch (style.type) {
      case 'bold': {
        opts.bold = true;
        if (!hasExplicitWeight) weight = 700;
        hasFontStyle = true;
        break;
      }
      case 'italic': {
        opts.italics = true;
        break;
      }
      case 'underline': {
        opts.underline = {};
        break;
      }
      case 'strikethrough': {
        opts.strike = true;
        break;
      }
      case 'font_size': {
        opts.size = fontSizeToHalfPoints(style.size);
        break;
      }
      case 'font_family': {
        familyName = style.family;
        hasFontStyle = true;
        break;
      }
      case 'font_weight': {
        weight = style.weight;
        hasExplicitWeight = true;
        opts.bold = style.weight >= 600;
        hasFontStyle = true;
        break;
      }
      case 'text_color': {
        const hex = resolveColorToHex(`text.${style.color}`);
        if (hex) opts.color = hex;
        break;
      }
      case 'background_color': {
        const hex = resolveColorToHex(`bg.${style.color}`);
        if (hex) opts.shading = { fill: hex };
        break;
      }
      case 'letter_spacing': {
        opts.characterSpacing = letterSpacingToTwips(style.spacing, fontSizePt100);
        break;
      }
    }
  }

  if (hasFontStyle) {
    const family = familyName ?? ctx.defaultFamilyName;
    const fam = findFontFamily(ctx.fonts, family);
    const font = fam ? nearestWeight(fam.weights, weight) : undefined;
    opts.font = font?.postScriptName ?? family;
  }

  return opts;
}

export function convertTextSegments(segments: TextSegment[], ctx: TextConvertContext): (TextRun | ExternalHyperlink | XmlComponent)[] {
  const result: (TextRun | ExternalHyperlink | XmlComponent)[] = [];

  for (const seg of segments) {
    const runOpts = buildRunOptions(seg.styles ?? [], ctx);
    if (ctx.defaultColor) {
      runOpts.color = ctx.defaultColor;
    }

    const annotations = seg.annotations ?? [];
    const link = annotations.find((a): a is Extract<Annotation, { type: 'link' }> => a.type === 'link');
    const ruby = annotations.find((a): a is Extract<Annotation, { type: 'ruby' }> => a.type === 'ruby');

    if (ruby) {
      result.push(ctx.createRubyRun(seg.text, ruby.text, runOpts));
      continue;
    }

    const textRun = new TextRun({ ...runOpts, text: seg.text });

    if (link) {
      result.push(new ExternalHyperlink({ children: [textRun], link: link.href }));
    } else {
      result.push(textRun);
    }
  }

  return result;
}
