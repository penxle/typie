// spell-checker:words HWPTAG HWPUNIT Horz
import { allocate, ctrlId, HWPTAG, makeRecord } from './records';

export type CellMargins = { left: number; right: number; top: number; bottom: number };

/**
 * 표 전체 레코드를 생성한다.
 * CTRL_HEADER(tbl) + TABLE + (LIST_HEADER + 셀 문단들) × 셀 수
 */
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
  const tableHeight = 0; // 한/글이 재계산
  const margins = cellMargins ?? { left: 566, right: 566, top: 284, bottom: 284 };

  const allRecords: Uint8Array[] = [];

  // CTRL_HEADER (tbl) + 개체 공통 속성 (46바이트)
  const { buf: ctrlBuf, view: ctrlView } = allocate(46);
  ctrlView.setUint32(0, ctrlId('tbl '), true);
  // attr: 글자처럼 취급(1), VertRelTo=para(2<<3), HorzRelTo=column(2<<8), TopAndBottom(3<<21)
  const tblAttr = 0x01 | (2 << 3) | (2 << 8) | (3 << 21) | (4 << 15) | (2 << 18);
  ctrlView.setUint32(4, tblAttr, true);
  ctrlView.setUint32(16, tableWidth, true);
  ctrlView.setUint32(20, tableHeight, true);
  ctrlView.setUint32(36, instanceId, true);
  ctrlView.setUint16(44, 0, true); // desc_len
  allRecords.push(makeRecord(HWPTAG.CTRL_HEADER, 1, ctrlBuf));

  // TABLE 레코드 (표 75)
  const tableRecordSize = 18 + rowCount * 2 + 4;
  const { buf: tblBuf, view: tblView } = allocate(tableRecordSize);
  let offset = 0;
  tblView.setUint32(offset, 0x01, true); // flags: 셀 단위로 나눔
  offset += 4;
  tblView.setUint16(offset, rowCount, true);
  offset += 2;
  tblView.setUint16(offset, colCount, true);
  offset += 2;
  tblView.setInt16(offset, 0, true); // cell_spacing
  offset += 2;
  // inner margins (4 × HWPUNIT16)
  tblView.setInt16(offset, margins.left, true);
  offset += 2;
  tblView.setInt16(offset, margins.right, true);
  offset += 2;
  tblView.setInt16(offset, margins.top, true);
  offset += 2;
  tblView.setInt16(offset, margins.bottom, true);
  offset += 2;
  // 각 행의 셀 수 (colCount per row)
  for (let i = 0; i < rowCount; i++) {
    tblView.setUint16(offset, colCount, true);
    offset += 2;
  }
  // border_fill_id (HWP 1-indexed)
  tblView.setUint16(offset, tableBorderFillId + 1, true);
  offset += 2;
  // zone_count
  tblView.setUint16(offset, 0, true);
  allRecords.push(makeRecord(HWPTAG.TABLE, 2, tblBuf));

  // 셀: LIST_HEADER + 셀 속성 + 문단들
  for (const [row, row_] of rows.entries()) {
    const rowCells = row_.cells;
    for (const [col, cell] of rowCells.entries()) {
      const cellWidth = cell.colWidth || Math.floor(tableWidth / colCount);
      const paraCount = countParagraphs(cell.paraRecords);

      // LIST_HEADER (8바이트) + 셀 속성 (26바이트) + 확장 필드 (13바이트) = 47바이트
      const { buf: listBuf, view: listView } = allocate(47);
      listView.setUint16(0, paraCount, true);
      listView.setUint16(2, 0, true); // unknown
      listView.setUint32(4, 0, true); // flags

      // 셀 속성 (표 80)
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
      // padding (4 × HWPUNIT16)
      listView.setInt16(cellOffset, margins.left, true);
      cellOffset += 2;
      listView.setInt16(cellOffset, margins.right, true);
      cellOffset += 2;
      listView.setInt16(cellOffset, margins.top, true);
      cellOffset += 2;
      listView.setInt16(cellOffset, margins.bottom, true);
      cellOffset += 2;
      // border_fill_id (HWP 1-indexed)
      const cellBfId = perRowBorderFillIds ? perRowBorderFillIds[row] : row === 0 ? tableBorderFillId : cellBorderFillId;
      listView.setUint16(cellOffset, cellBfId + 1, true);

      allRecords.push(makeRecord(HWPTAG.LIST_HEADER, 2, listBuf));
      setLastParagraphFlag(cell.paraRecords);
      allRecords.push(...cell.paraRecords);
    }
  }

  return allRecords;
}

/** 마지막 PARA_HEADER의 nchars bit 31을 설정 */
function setLastParagraphFlag(records: Uint8Array[]): void {
  for (let i = records.length - 1; i >= 0; i--) {
    const rec = records[i];
    if (rec.byteLength < 8) continue;
    const view = new DataView(rec.buffer, rec.byteOffset, rec.byteLength);
    const header = view.getUint32(0, true);
    const tagId = header & 0x3_ff;
    if (tagId === HWPTAG.PARA_HEADER) {
      const nchars = view.getUint32(4, true);
      view.setUint32(4, nchars | 0x80_00_00_00, true);
      return;
    }
  }
}

function countParagraphs(records: Uint8Array[]): number {
  let count = 0;
  for (const rec of records) {
    if (rec.byteLength >= 4) {
      const view = new DataView(rec.buffer, rec.byteOffset, rec.byteLength);
      const header = view.getUint32(0, true);
      const tagId = header & 0x3_ff;
      if (tagId === HWPTAG.PARA_HEADER) count++;
    }
  }
  return count;
}
