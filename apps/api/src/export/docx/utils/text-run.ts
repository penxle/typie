import { defaultValues } from '@typie/ui/tiptap/values-base';
import { TextRun } from 'docx';
import { normalizeColor } from './color';
import { ptToTwips, pxToHalfPt } from './unit';

type TextRunOptions = {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  fontSize?: number;
  fontFamily?: string;
  color?: string;
  backgroundColor?: string;
  letterSpacingPt?: number;
};

export function createStyledTextRun(options: TextRunOptions): TextRun {
  const { text, bold, italic, underline, strike, fontSize, fontFamily, color, backgroundColor, letterSpacingPt } = options;

  const normalizedColor = normalizeColor(color);
  const normalizedBgColor = normalizeColor(backgroundColor, true);

  return new TextRun({
    text,
    bold,
    italics: italic,
    underline: underline ? {} : undefined,
    strike,
    size: pxToHalfPt(fontSize ?? defaultValues.fontSize),
    font: fontFamily,
    color: normalizedColor,
    ...(normalizedBgColor && {
      shading: {
        fill: normalizedBgColor,
      },
    }),
    ...(letterSpacingPt &&
      letterSpacingPt !== 0 && {
        characterSpacing: Math.round(ptToTwips(letterSpacingPt)),
      }),
  });
}

export function createLinkTextRun(options: TextRunOptions): TextRun {
  const normalizedColor = normalizeColor(options.color);
  const normalizedBgColor = normalizeColor(options.backgroundColor, true);

  return new TextRun({
    text: options.text,
    bold: options.bold,
    italics: options.italic,
    underline: { type: 'single' },
    strike: options.strike,
    size: pxToHalfPt(options.fontSize ?? defaultValues.fontSize),
    font: options.fontFamily,
    color: normalizedColor || '0000FF',
    ...(normalizedBgColor && {
      shading: {
        fill: normalizedBgColor,
      },
    }),
    ...(options.letterSpacingPt &&
      options.letterSpacingPt !== 0 && {
        characterSpacing: Math.round(ptToTwips(options.letterSpacingPt)),
      }),
  });
}
