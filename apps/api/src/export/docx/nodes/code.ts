import { AlignmentType, TextRun } from 'docx';
import { createParagraph } from '../utils/utils';
import { createHardBreak } from './hard-break';
import type { JSONContent } from '@tiptap/core';
import type { Paragraph } from 'docx';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
// TODO: syntax highlighting 적용하기
export function convertCodeBlock(node: JSONContent, options: ConvertOptions = {}): Paragraph {
  const code = node.content?.[0]?.text || '';
  const language = node.attrs?.language || 'text';

  const lines = code.split('\n');
  const textRuns = lines
    .flatMap((textRun, index) => [new TextRun({ text: textRun }), index < lines.length - 1 ? createHardBreak() : null])
    .filter((textRun) => textRun !== null);
  const children = [new TextRun({ text: `[${language}]` }), createHardBreak(), ...textRuns];

  return createParagraph(
    {
      children,
      // style: 'Code', // TODO: code style
      alignment: AlignmentType.LEFT,
    },
    options,
  );
}
