import { AlignmentType, BorderStyle, Table, TableCell, TableRow, WidthType } from 'docx';
import { toBlockChildren } from './blocks';
import type { IBorderOptions, ITableBordersOptions } from 'docx';
import type { FileChild } from './index';

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

function mapTableAlignment(align: string): (typeof AlignmentType)[keyof typeof AlignmentType] | undefined {
  switch (align) {
    case 'center': {
      return AlignmentType.CENTER;
    }
    case 'right': {
      return AlignmentType.END;
    }
    default: {
      return undefined;
    }
  }
}

export function convertTable(node: { border_style?: string; align?: string; proportion?: number }, rows: TableRow[]): Table {
  const border = mapBorderStyle(node.border_style ?? 'solid');
  const borders: ITableBordersOptions = {
    top: border,
    bottom: border,
    left: border,
    right: border,
    insideHorizontal: border,
    insideVertical: border,
  };

  const alignment = node.align ? mapTableAlignment(node.align) : undefined;
  const proportion = node.proportion ?? 1;
  const widthPercent = Math.round(proportion * 100);

  return new Table({
    rows,
    borders,
    alignment,
    width: { size: widthPercent, type: WidthType.PERCENTAGE },
  });
}

export function convertTableRow(cells: TableCell[]): TableRow {
  return new TableRow({ children: cells });
}

export function convertTableCell(node: { col_width?: number | null }, children: FileChild[]): TableCell {
  return new TableCell({
    children: toBlockChildren(children),
    width: node.col_width ? { size: node.col_width, type: WidthType.DXA } : undefined,
    margins: { top: 40, bottom: 40, left: 80, right: 80 },
  });
}
