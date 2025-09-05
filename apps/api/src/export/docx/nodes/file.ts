import { AlignmentType, TextRun } from 'docx';
import { createParagraph } from '../utils/utils';
import type { JSONContent } from '@tiptap/core';
import type { Paragraph } from 'docx';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
export function convertFile(node: JSONContent, options: ConvertOptions = {}): Paragraph {
  const name = node.attrs?.name || '파일';

  return createParagraph(
    {
      children: [
        new TextRun({
          text: `[파일: ${name}]`,
          italics: true,
        }),
      ],
      alignment: AlignmentType.CENTER,
    },
    options,
  );
}
