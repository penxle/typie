import { Paragraph, Table, TextRun } from 'docx';
import { convertJSONContentToChildren } from '../docx';
import { createParagraph } from '../utils/utils';
import type { JSONContent } from '@tiptap/core';
import type { ConvertOptions } from '../types';

// TODO: shape, drawing 사용하여 처리하기
// TODO: VBA 매크로로 접기 처리하기??
export function convertFold(node: JSONContent, options: ConvertOptions = {}): (Paragraph | Table)[] {
  const { depth = 0 } = options;
  const children: (Paragraph | Table)[] = [];
  const title = node.attrs?.title || '접기';

  const titleIndent = depth * 200;

  children.push(
    createParagraph(
      {
        children: [
          new TextRun({
            text: `▼ ${title}`,
          }),
        ],
        indent: {
          left: titleIndent,
        },
      },
      options,
    ),
  );

  if (node.content) {
    const foldContent: JSONContent = {
      type: 'doc',
      content: node.content,
    };

    const contentIndent = (depth + 1) * 200;

    const contentChildren = convertJSONContentToChildren(foldContent, {
      ...options,
      depth: depth + 1,
      baseIndent: contentIndent,
    });

    children.push(...contentChildren);
  }

  return children;
}
