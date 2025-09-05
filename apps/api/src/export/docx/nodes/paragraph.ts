import { defaultValues } from '@typie/ui/tiptap/values-base';
import { AlignmentType, ExternalHyperlink, Paragraph, TextRun } from 'docx';
import { match } from 'ts-pattern';
import { Ruby, RubyRun } from '../marks/ruby';
import { normalizeColor } from '../utils/color';
import { processMarks } from '../utils/marks';
import { createLinkTextRun, createStyledTextRun } from '../utils/text-run';
import { emToPt } from '../utils/unit';
import { createParagraph } from '../utils/utils';
import { createHardBreak } from './hard-break';
import type { JSONContent } from '@tiptap/core';
import type { IParagraphOptions } from 'docx';
import type { ConvertOptions, Mark, TextStyles } from '../types';
import type { FontMapper } from '../utils/font-mapping';

export function convertParagraph(
  node: JSONContent,
  options: ConvertOptions,
  paragraphOptions: IParagraphOptions & { prefix?: TextRun } = {},
): Paragraph {
  const textAlign = node.attrs?.textAlign || defaultValues.textAlign;
  const lineHeight = node.attrs?.lineHeight || defaultValues.lineHeight;
  const letterSpacing = node.attrs?.letterSpacing || defaultValues.letterSpacing;

  const letterSpacingPt = emToPt(letterSpacing);

  const textRuns = convertInlineContentToTextRuns(node.content || [], options.fontMapper, letterSpacingPt);

  const alignment = match(textAlign)
    .with('center', () => AlignmentType.CENTER)
    .with('right', () => AlignmentType.RIGHT)
    .with('justify', () => AlignmentType.JUSTIFIED)
    .otherwise(() => AlignmentType.LEFT);

  const children = paragraphOptions.prefix ? [paragraphOptions.prefix, ...textRuns] : textRuns;

  return createParagraph(
    {
      children: children.length > 0 ? children : [new TextRun('')],
      alignment,
      ...paragraphOptions,
    },
    options,
    lineHeight,
  );
}

export function convertInlineContentToTextRuns(
  content: JSONContent[],
  fontMapper?: FontMapper,
  letterSpacingPt?: number,
): (TextRun | ExternalHyperlink | RubyRun)[] {
  return content.flatMap((inline) => {
    return match(inline)
      .with({ type: 'text' }, (node) => {
        const styles: TextStyles = processMarks((node.marks as Mark[]) || [], fontMapper);

        if (styles.linkHref) {
          const isBold = styles.fontWeight ? styles.fontWeight >= 700 : false;
          return new ExternalHyperlink({
            link: styles.linkHref,
            children: [
              createLinkTextRun({
                text: node.text || '',
                ...styles,
                bold: isBold,
                letterSpacingPt,
              }),
            ],
          });
        }

        if (styles.rubyText) {
          const ruby = new Ruby(node.text || '', styles.rubyText, {
            ...styles,
            color: normalizeColor(styles.color),
          });

          return new RubyRun(ruby);
        }

        const isBold = styles.fontWeight ? styles.fontWeight >= 700 : false;

        return createStyledTextRun({
          text: node.text || '',
          ...styles,
          bold: isBold,
          letterSpacingPt,
        });
      })
      .with({ type: 'hard_break' }, () => createHardBreak())
      .otherwise(() => new TextRun({ text: '' }));
  });
}
