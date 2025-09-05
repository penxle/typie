import { AlignmentType, TextRun } from 'docx';
import { createParagraph } from '../utils/utils';
import type { JSONContent } from '@tiptap/core';
import type { Paragraph } from 'docx';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
export function convertEmbed(node: JSONContent, options: ConvertOptions = {}): Paragraph {
  const url = node.attrs?.url || '임베드';

  return createParagraph(
    {
      children: [
        new TextRun({
          text: `[임베드: ${url}]`,
          italics: true,
        }),
      ],
      alignment: AlignmentType.CENTER,
    },
    options,
  );
}
