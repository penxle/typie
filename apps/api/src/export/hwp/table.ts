// spell-checker:words HWPTAG HWPUNIT Horz
import {
  buildSectionDef,
  collectInlineSegments,
  makeEmptyParagraph,
  makeInlineObjectParagraph,
  makeParagraph,
  setLastParagraphFlag,
} from './paragraph';
import { allocate, ctrlId, hexToColorref, HWPTAG, makeRecord, pxToHwpunit } from './records';
import { resolveParaShape } from './styles';
import type { NodeEntry } from '../core/types';
import type { BorderFillEntry } from './doc-info';
import type { HwpConvertContext, InlineSegment } from './types';

export type CellMargins = { left: number; right: number; top: number; bottom: number };

/** 표 전체 레코드: CTRL_HEADER(tbl) + TABLE + (LIST_HEADER + 셀 문단들) × 셀 수 */
export function makeTableRecords(
  rows: { cells: { paraRecords: Uint8Array[]; colWidth: number }[] }[],
  rowCount: number,
  colCount: number,
  tableWidth: number,
  tableBorderFillId: number,
  cellBorderFillId: number,
  instanceId: number,
  cellMargins?: CellMargins,
  perRowBorderFillIds?: number[],
): Uint8Array[] {
  const margins = cellMargins ?? { left: 566, right: 566, top: 284, bottom: 284 };
  const allRecords: Uint8Array[] = [];

  // CTRL_HEADER (tbl) + 개체 공통 속성 (46바이트)
  const { buf: ctrlBuf, view: ctrlView } = allocate(46);
  ctrlView.setUint32(0, ctrlId('tbl '), true);
  // attr: 글자처럼 취급(1), VertRelTo=para(2<<3), HorzRelTo=column(2<<8), TopAndBottom(3<<21)
  const tblAttr = 0x01 | (2 << 3) | (2 << 8) | (3 << 21) | (4 << 15) | (2 << 18);
  ctrlView.setUint32(4, tblAttr, true);
  ctrlView.setUint32(16, tableWidth, true);
  ctrlView.setUint32(20, 0, true); // tableHeight: 한/글이 재계산
  ctrlView.setUint32(36, instanceId, true);
  ctrlView.setUint16(44, 0, true);
  allRecords.push(makeRecord(HWPTAG.CTRL_HEADER, 1, ctrlBuf));

  // TABLE 레코드
  const tableRecordSize = 18 + rowCount * 2 + 4;
  const { buf: tblBuf, view: tblView } = allocate(tableRecordSize);
  let offset = 0;
  tblView.setUint32(offset, 0x01, true);
  offset += 4;
  tblView.setUint16(offset, rowCount, true);
  offset += 2;
  tblView.setUint16(offset, colCount, true);
  offset += 2;
  tblView.setInt16(offset, 0, true);
  offset += 2;
  tblView.setInt16(offset, margins.left, true);
  offset += 2;
  tblView.setInt16(offset, margins.right, true);
  offset += 2;
  tblView.setInt16(offset, margins.top, true);
  offset += 2;
  tblView.setInt16(offset, margins.bottom, true);
  offset += 2;
  for (let i = 0; i < rowCount; i++) {
    tblView.setUint16(offset, colCount, true);
    offset += 2;
  }
  tblView.setUint16(offset, tableBorderFillId + 1, true); // HWP 1-indexed
  offset += 2;
  tblView.setUint16(offset, 0, true); // zone_count
  allRecords.push(makeRecord(HWPTAG.TABLE, 2, tblBuf));

  // 셀: LIST_HEADER + 셀 속성 + 문단들
  for (const [row, row_] of rows.entries()) {
    for (const [col, cell] of row_.cells.entries()) {
      const cellWidth = cell.colWidth || Math.floor(tableWidth / colCount);
      const paraCount = countParagraphs(cell.paraRecords);

      // LIST_HEADER(8) + 셀 속성(26) + 확장 필드(13) = 47바이트
      const { buf: listBuf, view: listView } = allocate(47);
      listView.setUint16(0, paraCount, true);
      listView.setUint16(2, 0, true);
      listView.setUint32(4, 0, true);

      let cellOffset = 8;
      listView.setUint16(cellOffset, col, true);
      cellOffset += 2;
      listView.setUint16(cellOffset, row, true);
      cellOffset += 2;
      listView.setUint16(cellOffset, 1, true); // col_span
      cellOffset += 2;
      listView.setUint16(cellOffset, 1, true); // row_span
      cellOffset += 2;
      listView.setUint32(cellOffset, cellWidth, true);
      cellOffset += 4;
      listView.setUint32(cellOffset, 0, true); // cell_height (재계산)
      cellOffset += 4;
      listView.setInt16(cellOffset, margins.left, true);
      cellOffset += 2;
      listView.setInt16(cellOffset, margins.right, true);
      cellOffset += 2;
      listView.setInt16(cellOffset, margins.top, true);
      cellOffset += 2;
      listView.setInt16(cellOffset, margins.bottom, true);
      cellOffset += 2;
      const cellBfId = perRowBorderFillIds ? perRowBorderFillIds[row] : row === 0 ? tableBorderFillId : cellBorderFillId;
      listView.setUint16(cellOffset, cellBfId + 1, true); // HWP 1-indexed

      allRecords.push(makeRecord(HWPTAG.LIST_HEADER, 2, listBuf));
      setLastParagraphFlag(cell.paraRecords);
      allRecords.push(...cell.paraRecords);
    }
  }

  return allRecords;
}

function countParagraphs(records: Uint8Array[]): number {
  let count = 0;
  for (const rec of records) {
    if (rec.byteLength >= 4) {
      const view = new DataView(rec.buffer, rec.byteOffset, rec.byteLength);
      const header = view.getUint32(0, true);
      if ((header & 0x3_ff) === HWPTAG.PARA_HEADER) count++;
    }
  }
  return count;
}

// --- 표 변환기 ---

export function convertTableNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const proportion = (entry as { proportion?: number }).proportion ?? 1;
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const tableWidth = pxToHwpunit(contentWidthPx * Math.min(proportion, 1));

  const rows: { cells: { paraRecords: Uint8Array[]; colWidth: number }[] }[] = [];
  for (const rowId of entry.children ?? []) {
    const rowEntry = ctx.nodes[rowId];
    if (!rowEntry || rowEntry.type !== 'table_row') continue;

    const cells: { paraRecords: Uint8Array[]; colWidth: number }[] = [];
    for (const cellId of rowEntry.children ?? []) {
      const cellEntry = ctx.nodes[cellId];
      if (!cellEntry || cellEntry.type !== 'table_cell') continue;
      const cellRecords = convertCellContent(cellEntry, ctx);
      const colWidth = (cellEntry.col_width as number | null) ?? 0;
      cells.push({ paraRecords: cellRecords, colWidth });
    }
    rows.push({ cells });
  }

  if (rows.length === 0) return [];

  const rowCount = rows.length;
  const colCount = Math.max(...rows.map((r) => r.cells.length));

  const borderColor = hexToColorref('CCCCCC');
  const bfEntry: BorderFillEntry = {
    leftType: 1,
    rightType: 1,
    topType: 1,
    bottomType: 1,
    leftWidth: 1,
    rightWidth: 1,
    topWidth: 1,
    bottomWidth: 1,
    leftColor: borderColor,
    rightColor: borderColor,
    topColor: borderColor,
    bottomColor: borderColor,
    fillType: 0,
    fillColor: 0,
  };
  const tableBorderFillId = ctx.tables.borderFills.intern(bfEntry, 'table-default');

  return buildInlineTable({
    rows,
    rowCount,
    colCount,
    tableWidth,
    tableBorderFillId,
    cellBorderFillId: tableBorderFillId,
    ctx,
    isFirst,
    cellMargins: { left: 800, right: 800, top: 400, bottom: 400 },
  });
}

function convertCellContent(cellEntry: NodeEntry, ctx: HwpConvertContext): Uint8Array[] {
  const records: Uint8Array[] = [];
  for (const childId of cellEntry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;
    if (childEntry.type === 'paragraph') {
      const segments = collectInlineSegments(childEntry, ctx);
      const paraShapeId = resolveParaShape(ctx, {
        align: childEntry.align as string | undefined,
        lineHeight: childEntry.line_height as number | undefined,
      });
      records.push(...makeParagraph(segments, paraShapeId, ctx.defaultCharShapeId, 2));
    }
  }
  if (records.length === 0) {
    records.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
  }
  return records;
}

// --- 표 시뮬레이션 유틸 (blockquote/callout/fold/embed → 표) ---

export function makeSimpleTableFromParagraphs(
  paragraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[],
  ctx: HwpConvertContext,
  isFirst: boolean,
  bfEntry: BorderFillEntry,
  cellMargins?: CellMargins,
  opts?: { tableWidthRatio?: number; tableAlign?: string; contentAlign?: string },
): Uint8Array[] {
  const borderFillId = ctx.tables.borderFills.intern(bfEntry, JSON.stringify(bfEntry));
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const ratio = opts?.tableWidthRatio ?? 1;
  const tableWidth = pxToHwpunit(contentWidthPx * Math.min(ratio, 1));

  const cellRecords: Uint8Array[] = [];
  for (const p of paragraphs) {
    const paraShapeId = resolveParaShape(ctx, {
      align: opts?.contentAlign ?? p.align,
      lineHeight: p.lineHeight,
    });
    cellRecords.push(...makeParagraph(p.segments, paraShapeId, ctx.defaultCharShapeId, 2));
  }
  if (cellRecords.length === 0) {
    cellRecords.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
  }

  const rows = [{ cells: [{ paraRecords: cellRecords, colWidth: tableWidth }] }];
  return buildInlineTable({
    rows,
    rowCount: 1,
    colCount: 1,
    tableWidth,
    tableBorderFillId: borderFillId,
    cellBorderFillId: borderFillId,
    ctx,
    isFirst,
    cellMargins,
    tableAlign: opts?.tableAlign,
  });
}

export function makeTwoRowTable(
  titleSegments: InlineSegment[],
  contentParagraphs: { segments: InlineSegment[]; align?: string; lineHeight?: number }[],
  titleBorderFillId: number,
  contentBorderFillId: number,
  ctx: HwpConvertContext,
  isFirst: boolean,
  cellMargins?: CellMargins,
): Uint8Array[] {
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const tableWidth = pxToHwpunit(contentWidthPx);

  const titleParaShapeId = resolveParaShape(ctx, {});
  const titleRecords = makeParagraph(titleSegments, titleParaShapeId, ctx.defaultCharShapeId, 2);

  const contentRecords: Uint8Array[] = [];
  for (const p of contentParagraphs) {
    const psId = resolveParaShape(ctx, { align: p.align, lineHeight: p.lineHeight });
    contentRecords.push(...makeParagraph(p.segments, psId, ctx.defaultCharShapeId, 2));
  }
  if (contentRecords.length === 0) {
    contentRecords.push(...makeEmptyParagraph(ctx.defaultParaShapeId, ctx.defaultCharShapeId, 2));
  }

  const rows = [
    { cells: [{ paraRecords: titleRecords, colWidth: tableWidth }] },
    { cells: [{ paraRecords: contentRecords, colWidth: tableWidth }] },
  ];

  const emptyBfId = ctx.tables.borderFills.intern(
    {
      leftType: 0,
      rightType: 0,
      topType: 0,
      bottomType: 0,
      leftWidth: 0,
      rightWidth: 0,
      topWidth: 0,
      bottomWidth: 0,
      leftColor: 0,
      rightColor: 0,
      topColor: 0,
      bottomColor: 0,
      fillType: 0,
      fillColor: 0,
    },
    'table-no-fill',
  );

  return buildInlineTable({
    rows,
    rowCount: 2,
    colCount: 1,
    tableWidth,
    tableBorderFillId: emptyBfId,
    cellBorderFillId: emptyBfId,
    ctx,
    isFirst,
    cellMargins,
    perRowBorderFillIds: [titleBorderFillId, contentBorderFillId],
  });
}

type InlineTableOpts = {
  rows: { cells: { paraRecords: Uint8Array[]; colWidth: number }[] }[];
  rowCount: number;
  colCount: number;
  tableWidth: number;
  tableBorderFillId: number;
  cellBorderFillId: number;
  ctx: HwpConvertContext;
  isFirst: boolean;
  cellMargins?: CellMargins;
  tableAlign?: string;
  perRowBorderFillIds?: number[];
};

/** 인라인 개체 문단으로 감싼 표 레코드 생성 */
function buildInlineTable(opts: InlineTableOpts): Uint8Array[] {
  const {
    rows,
    rowCount,
    colCount,
    tableWidth,
    tableBorderFillId,
    cellBorderFillId,
    ctx,
    isFirst,
    cellMargins,
    tableAlign,
    perRowBorderFillIds,
  } = opts;
  const instanceId = ++ctx.instanceCounter;

  const sectionRecords = isFirst ? buildSectionDef(ctx) : undefined;
  const paraShapeId = tableAlign ? resolveParaShape(ctx, { align: tableAlign }) : undefined;
  return [
    ...makeInlineObjectParagraph(ctx, 0, 'tbl ', { sectionRecords, paraShapeId }),
    ...makeTableRecords(
      rows,
      rowCount,
      colCount,
      tableWidth,
      tableBorderFillId,
      cellBorderFillId,
      instanceId,
      cellMargins,
      perRowBorderFillIds,
    ),
  ];
}
