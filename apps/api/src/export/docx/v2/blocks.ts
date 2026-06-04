import { AlignmentType, BorderStyle, Paragraph, ShadingType, Table, TableCell, TableLayoutType, TableRow, TextRun, WidthType } from 'docx';
import { resolveColorToHex } from '../../core/theme.ts';
import {
  convertCallout,
  convertHorizontalRule,
  convertListItemParagraph,
  convertParagraph,
  convertPlaceholderTable,
  NO_BORDER,
  SUBTLE_BORDER,
  toBlockChildren,
} from '../blocks.ts';
import type { Alignment } from '@typie/editor-ffi/server';
import type { IParagraphOptions } from 'docx';
import type { DocDefaults } from '../blocks.ts';
import type { FileChild } from '../index.ts';

export function buildParagraphV2(
  align: Alignment,
  lineHeight: number,
  inlineChildren: IParagraphOptions['children'],
  indentTwips: number,
  defaults: DocDefaults,
): FileChild {
  return convertParagraph({ align, line_height: lineHeight }, inlineChildren, indentTwips, defaults);
}

export function buildListItemParagraphV2(
  align: Alignment,
  lineHeight: number,
  inlineChildren: IParagraphOptions['children'],
  listType: 'bullet' | 'ordered',
  level: number,
  numberingRef: string,
  defaults: DocDefaults,
): FileChild {
  return convertListItemParagraph(inlineChildren, listType, level, numberingRef, { align, line_height: lineHeight }, defaults);
}

export function buildHorizontalRuleV2(): FileChild {
  return convertHorizontalRule();
}

export function buildPlaceholderTableV2(text: string): FileChild {
  return convertPlaceholderTable(text);
}

export function buildCalloutV2(variant: string, children: FileChild[]): FileChild {
  return convertCallout({ variant }, children);
}

export function buildFoldV2(titleChildren: IParagraphOptions['children'], content: FileChild[]): FileChild {
  const titleParagraph = new Paragraph({
    children: [new TextRun({ text: '▶ ', bold: true }), ...(titleChildren ?? [])],
    run: { bold: true },
    spacing: { after: 0 },
  });

  return new Table({
    rows: [
      new TableRow({
        children: [
          new TableCell({
            children: [titleParagraph],
            shading: { fill: 'F3F4F9', type: ShadingType.CLEAR },
            borders: { top: SUBTLE_BORDER, bottom: SUBTLE_BORDER, left: SUBTLE_BORDER, right: SUBTLE_BORDER },
            margins: { top: 80, bottom: 80, left: 120, right: 120 },
          }),
        ],
      }),
      new TableRow({
        children: [
          new TableCell({
            children: toBlockChildren(content),
            borders: { top: NO_BORDER, bottom: SUBTLE_BORDER, left: SUBTLE_BORDER, right: SUBTLE_BORDER },
            margins: { top: 80, bottom: 80, left: 120, right: 120 },
          }),
        ],
      }),
    ],
    width: { size: 100, type: WidthType.PERCENTAGE },
  });
}

export function buildBlockquoteV2(variant: string, children: FileChild[]): FileChild[] {
  if (variant === 'message_sent' || variant === 'message_received') {
    const isSent = variant === 'message_sent';
    const hex =
      resolveColorToHex(isSent ? 'ui.blockquote.message-sent' : 'ui.blockquote.message-received') ?? (isSent ? '248BF5' : 'E5E5EA');

    return [
      new Table({
        alignment: isSent ? AlignmentType.END : AlignmentType.START,
        layout: TableLayoutType.AUTOFIT,
        width: { size: 0, type: WidthType.AUTO },
        borders: {
          top: NO_BORDER,
          bottom: NO_BORDER,
          left: NO_BORDER,
          right: NO_BORDER,
          insideHorizontal: NO_BORDER,
          insideVertical: NO_BORDER,
        },
        rows: [
          new TableRow({
            children: [
              new TableCell({
                children: toBlockChildren(children),
                shading: { fill: hex, type: ShadingType.CLEAR },
                borders: { top: NO_BORDER, bottom: NO_BORDER, left: NO_BORDER, right: NO_BORDER },
                margins: { top: 80, bottom: 80, left: 160, right: 160 },
              }),
            ],
          }),
        ],
      }),
    ];
  }

  const borderColor = variant === 'left_quote' ? '000000' : 'CCCCCC';
  return [
    new Table({
      width: { size: 100, type: WidthType.PERCENTAGE },
      borders: {
        top: NO_BORDER,
        bottom: NO_BORDER,
        left: NO_BORDER,
        right: NO_BORDER,
        insideHorizontal: NO_BORDER,
        insideVertical: NO_BORDER,
      },
      rows: [
        new TableRow({
          children: [
            new TableCell({
              children: toBlockChildren(children),
              borders: {
                top: NO_BORDER,
                bottom: NO_BORDER,
                left: { style: BorderStyle.SINGLE, size: 18, color: borderColor },
                right: NO_BORDER,
              },
              margins: { top: 40, bottom: 40, left: 200, right: 0 },
            }),
          ],
        }),
      ],
    }),
  ];
}
