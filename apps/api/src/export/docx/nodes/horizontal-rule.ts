import { defaultValues } from '@typie/ui/tiptap/values-base';
import { AlignmentType, Paragraph, TextRun } from 'docx';
import { pxToHalfPt } from '../utils/unit';
import { createParagraph } from '../utils/utils';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
export function convertHorizontalRule(options: ConvertOptions = {}): Paragraph {
  return createParagraph(
    {
      children: [
        new TextRun({
          text: '─'.repeat(10), // U+2500 (Box Drawings Light Horizontal)
          font: defaultValues.fontFamily,
          size: pxToHalfPt(defaultValues.fontSize),
        }),
      ],
      alignment: AlignmentType.CENTER,
    },
    options,
  );
}
