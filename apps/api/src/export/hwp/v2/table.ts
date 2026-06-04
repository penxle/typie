// spell-checker:words Hwpunit
import { buildSectionDef, makeEmptyParagraph, makeInlineObjectParagraph } from '../paragraph.ts';
import { hexToColorref, pxToHwpunit } from '../records.ts';
import { makeTableRecords } from '../table.ts';
import type { TableV2 } from '../../core/v2/types.ts';
import type { BorderFillEntry } from '../doc-info.ts';
import type { HwpConvertContext } from '../types.ts';

function borderType(style: TableV2<Uint8Array[]>['borderStyle']): number {
  switch (style) {
    case 'none': {
      return 0;
    }
    case 'dashed': {
      return 2;
    }
    case 'dotted': {
      return 3;
    }
    default: {
      return 1;
    }
  }
}

export function tableToRecordsV2(t: TableV2<Uint8Array[]>, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const tableWidth = pxToHwpunit(contentWidthPx * Math.min(t.proportion, 1));

  const rows: { cells: { paraRecords: Uint8Array[]; colWidth: number }[] }[] = [];
  for (const row of t.rows) {
    const cells: { paraRecords: Uint8Array[]; colWidth: number }[] = [];
    for (const cell of row.cells) {
      const paraRecords = cell.children.flat();
      if (paraRecords.length === 0) {
        paraRecords.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
      }
      cells.push({ paraRecords, colWidth: cell.colWidth ? pxToHwpunit(cell.colWidth) : 0 });
    }
    rows.push({ cells });
  }

  if (rows.length === 0) return [];

  const rowCount = rows.length;
  const colCount = Math.max(...rows.map((r) => r.cells.length));

  const bt = borderType(t.borderStyle);
  const borderColor = hexToColorref('CCCCCC');
  const bfEntry: BorderFillEntry = {
    leftType: bt,
    rightType: bt,
    topType: bt,
    bottomType: bt,
    leftWidth: bt === 0 ? 0 : 1,
    rightWidth: bt === 0 ? 0 : 1,
    topWidth: bt === 0 ? 0 : 1,
    bottomWidth: bt === 0 ? 0 : 1,
    leftColor: borderColor,
    rightColor: borderColor,
    topColor: borderColor,
    bottomColor: borderColor,
    fillType: 0,
    fillColor: 0,
  };
  const tableBorderFillId = ctx.tables.borderFills.intern(bfEntry, `table-v2-${t.borderStyle}`);

  const instanceId = ++ctx.instanceCounter;
  const sectionRecords = isFirst ? buildSectionDef(ctx) : undefined;

  return [
    ...makeInlineObjectParagraph(ctx, 0, 'tbl ', { sectionRecords }),
    ...makeTableRecords(rows, rowCount, colCount, tableWidth, tableBorderFillId, tableBorderFillId, instanceId, {
      left: 800,
      right: 800,
      top: 400,
      bottom: 400,
    }),
  ];
}
