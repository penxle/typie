import { defaultValues } from '@typie/ui/tiptap/values-base';
import { LineRuleType, Paragraph } from 'docx';
import { ptToTwips, pxToPt, remToPt } from './unit';
import type { IParagraphOptions } from 'docx';
import type { ConvertOptions } from '../types';

export function createParagraph(
  paragraphOptions: IParagraphOptions,
  options: Pick<ConvertOptions, 'bodyAttrs' | 'baseIndent'>,
  lineHeight?: number,
): Paragraph {
  const baseFontSizePt = pxToPt(defaultValues.fontSize);
  const lineSpacingPt = baseFontSizePt * (lineHeight ?? defaultValues.lineHeight);
  const blockGapPt = remToPt(options.bodyAttrs?.blockGap || 0);
  const paragraphIndentPt = remToPt(options.bodyAttrs?.paragraphIndent || 0);

  const defaultSpacing = {
    before: Math.round(ptToTwips(blockGapPt)),
    line: Math.round(ptToTwips(lineSpacingPt)),
    lineRule: LineRuleType.AT_LEAST,
  };

  const { spacing: paragraphSpacing, ...restOptions } = paragraphOptions;
  const spacing = { ...defaultSpacing, ...paragraphSpacing };

  const defaultIndent = {
    ...(paragraphIndentPt !== 0 && { firstLine: Math.round(ptToTwips(paragraphIndentPt)) }),
    ...(options.baseIndent !== undefined && { left: options.baseIndent }),
  };

  return new Paragraph({
    style: 'Normal',
    spacing,
    ...(defaultIndent && { indent: defaultIndent }),
    ...restOptions,
  });
}
