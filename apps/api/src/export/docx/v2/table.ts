import { BorderStyle, ShadingType, Table, TableCell, TableRow, WidthType } from 'docx';
import { toBlockChildren } from '../blocks.ts';
import type { IBorderOptions, ITableBordersOptions } from 'docx';
import type { TableV2 } from '../../core/v2/types.ts';
import type { FileChild } from '../index.ts';

function mapBorderStyle(style: string): IBorderOptions {
  switch (style) {
    case 'dashed': {
      return { style: BorderStyle.DASHED, size: 1, color: 'CCCCCC' };
    }
    case 'dotted': {
      return { style: BorderStyle.DOTTED, size: 1, color: 'CCCCCC' };
    }
    case 'none': {
      return { style: BorderStyle.NONE, size: 0 };
    }
    default: {
      return { style: BorderStyle.SINGLE, size: 1, color: 'CCCCCC' };
    }
  }
}

export function convertTableV2(t: TableV2<FileChild[]>): Table {
  const border = mapBorderStyle(t.borderStyle);
  const borders: ITableBordersOptions = {
    top: border,
    bottom: border,
    left: border,
    right: border,
    insideHorizontal: border,
    insideVertical: border,
  };

  const rows = t.rows.map(
    (row) =>
      new TableRow({
        children: row.cells.map(
          (cell) =>
            new TableCell({
              children: toBlockChildren(cell.children.flat()),
              width: cell.colWidth ? { size: cell.colWidth, type: WidthType.DXA } : undefined,
              shading: cell.backgroundColorHex ? { fill: cell.backgroundColorHex, type: ShadingType.CLEAR } : undefined,
              margins: { top: 40, bottom: 40, left: 80, right: 80 },
            }),
        ),
      }),
  );

  const widthPercent = Math.round(t.proportion * 100);

  return new Table({
    rows,
    borders,
    width: { size: widthPercent, type: WidthType.PERCENTAGE },
  });
}
