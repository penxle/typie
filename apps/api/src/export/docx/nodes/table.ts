import { BorderStyle, Paragraph, Table, TableCell, TableLayoutType, TableRow, WidthType } from 'docx';
import { convertJSONContentToChildren } from '../docx';
import { pxToTwips } from '../utils/unit';
import type { JSONContent } from '@tiptap/core';
import type { ConvertOptions } from '../types';

export function convertTable(node: JSONContent, options: ConvertOptions = {}): Table {
  const rows: TableRow[] = [];

  if (!node.content) {
    return new Table({
      rows: [],
    });
  }

  node.content.forEach((row) => {
    if (row.type === 'table_row' && row.content) {
      const cells: TableCell[] = [];

      row.content.forEach((cell) => {
        let cellChildren: (Paragraph | Table)[] = [];
        if (cell.content) {
          const cellContent: JSONContent = {
            type: 'doc',
            content: cell.content,
          };

          cellChildren = convertJSONContentToChildren(cellContent, options);
        }

        if (cellChildren.length === 0) {
          cellChildren.push(new Paragraph({ text: '' }));
        }

        cells.push(
          new TableCell({
            children: cellChildren,
            borders: {
              top: { style: BorderStyle.SINGLE, size: 1, color: 'CCCCCC' },
              bottom: { style: BorderStyle.SINGLE, size: 1, color: 'CCCCCC' },
              left: { style: BorderStyle.SINGLE, size: 1, color: 'CCCCCC' },
              right: { style: BorderStyle.SINGLE, size: 1, color: 'CCCCCC' },
            },
            width: {
              size: 0,
              type: WidthType.AUTO,
            },
            margins: {
              marginUnitType: WidthType.DXA,
              top: pxToTwips(10),
              bottom: pxToTwips(10),
              left: pxToTwips(14),
              right: pxToTwips(14),
            },
            columnSpan: cell.attrs?.colspan || 1,
            rowSpan: cell.attrs?.rowspan || 1,
          }),
        );
      });

      rows.push(new TableRow({ children: cells }));
    }
  });

  return new Table({
    rows,
    width: {
      size: 100,
      type: WidthType.PERCENTAGE,
    },
    layout: TableLayoutType.AUTOFIT,
  });
}
