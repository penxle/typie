import { TextRun } from 'docx';
import { createParagraph } from '../utils/utils';
import { createHardBreak } from './hard-break';
import type { JSONContent } from '@tiptap/core';
import type { Paragraph } from 'docx';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
// TODO: syntax highlighting 적용하기
export function convertHtmlBlock(node: JSONContent, options: ConvertOptions = {}): Paragraph {
  const html = node.content?.[0]?.text || '';

  const lines = html.split('\n');
  const textRuns = lines
    .flatMap((line, index) => [new TextRun({ text: line }), index < lines.length - 1 ? createHardBreak() : null])
    .filter((textRun) => textRun !== null);
  const children = [new TextRun({ text: `[HTML]` }), createHardBreak(), ...textRuns];

  return createParagraph(
    {
      children,
      style: 'Code',
    },
    options,
  );
}
