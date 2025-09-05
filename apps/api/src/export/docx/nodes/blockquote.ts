import { Paragraph } from 'docx';
import { convertParagraph } from './paragraph';
import type { JSONContent } from '@tiptap/core';
import type { ConvertOptions } from '../types';

// NOTE: blockquote를 일반 paragraph들로 처리
// TODO: 뭔가 타이피 에디터와 비슷한 스타일로 처리하기
export function convertBlockquoteToParagraphs(node: JSONContent, options: ConvertOptions = {}): Paragraph[] {
  const paragraphs: Paragraph[] = [];

  if (!node.content) {
    return paragraphs;
  }

  node.content.forEach((para) => {
    if (para.type === 'paragraph') {
      const paragraph = convertParagraph(para, options);
      paragraphs.push(paragraph);
    }
  });

  return paragraphs;
}
