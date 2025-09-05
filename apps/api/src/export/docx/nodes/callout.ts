import { Paragraph } from 'docx';
import { convertParagraph } from './paragraph';
import type { JSONContent } from '@tiptap/core';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
export function convertCalloutToParagraphs(node: JSONContent, options: ConvertOptions = {}): Paragraph[] {
  const paragraphs: Paragraph[] = [];

  // const calloutType = node.attrs?.type || 'info';
  // const emoji = match(calloutType)
  //   .with('info', () => 'ℹ️')
  //   .with('warning', () => '⚠️')
  //   .with('error', () => '❌')
  //   .with('success', () => '✅')
  //   .otherwise(() => '📝');

  // paragraphs.push(
  //   createParagraph(
  //     {
  //       children: [
  //         new TextRun({
  //           text: `${emoji}`,
  //         }),
  //       ],
  //     },
  //     options.bodyAttrs,
  //   ),
  // );

  if (node.content) {
    node.content.forEach((para) => {
      if (para.type === 'paragraph') {
        const paragraph = convertParagraph(para, options);
        paragraphs.push(paragraph);
      }
    });
  }

  return paragraphs;
}
