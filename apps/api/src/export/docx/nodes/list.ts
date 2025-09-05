import { Paragraph, TextRun } from 'docx';
import { convertParagraph } from './paragraph';
import type { JSONContent } from '@tiptap/core';
import type { ConvertOptions } from '../types';

export function convertListToParagraphs(listNode: JSONContent, isOrdered: boolean, options: ConvertOptions = {}): Paragraph[] {
  return convertListToParagraphsWithDepth(listNode, isOrdered, 0, options);
}

const getFirstListItem = (node: JSONContent): JSONContent | null => {
  return (node.content || []).find((n) => n.type === 'list_item') ?? null;
};

function convertListToParagraphsWithDepth(
  listNode: JSONContent,
  isOrdered: boolean,
  listDepth: number,
  options: ConvertOptions = {},
  firstItemNodeId: string | null = null,
): Paragraph[] {
  const paragraphs: Paragraph[] = [];
  let orderedIndex = 1;

  if (!listNode.content) {
    return paragraphs;
  }

  if (firstItemNodeId === null) {
    const firstItem = getFirstListItem(listNode);
    if (firstItem) {
      firstItemNodeId = firstItem.attrs?.nodeId;
    }
  }

  listNode.content.forEach((item) => {
    if (item.type === 'list_item' && item.content) {
      const firstPara = item.content.find((n) => n.type === 'paragraph');
      const secondPara = item.content.filter((n) => n.type === 'paragraph')[1];
      if (firstPara) {
        const bullet = isOrdered ? `${orderedIndex}. ` : '• ';
        const para = firstPara;
        if (para.content && secondPara?.content) {
          para.content.push({ type: 'hard_break' }, ...secondPara.content);
        }

        const isFirstItem = firstItemNodeId === item.attrs?.nodeId;

        paragraphs.push(
          convertParagraph(para, options, {
            prefix: new TextRun({ text: bullet }),
            indent: {
              left: (options.baseIndent ?? 0) + listDepth * 360,
              hanging: 180,
            },
            spacing: {
              ...(!isFirstItem && { before: 0 }),
            },
          }),
        );
      }

      item.content.forEach((child) => {
        if (child.type === 'bullet_list' || child.type === 'ordered_list') {
          const nestedListParas = convertListToParagraphsWithDepth(
            child,
            child.type === 'ordered_list',
            listDepth + 1,
            options,
            firstItemNodeId,
          );
          paragraphs.push(...nestedListParas);
        }
      });

      orderedIndex++;
    }
  });

  return paragraphs;
}
