// spell-checker:words HWPTAG
import { allocate, concat, encodeUTF16LE, HWPTAG, makeRecord } from './records.ts';
import type { IdTable } from './records.ts';

export type FontEntry = { name: string; postScriptName: string };

export type CharShapeEntry = {
  fontId: number;
  baseSize: number;
  bold: boolean;
  italic: boolean;
  underline: boolean;
  strikethrough: boolean;
  textColor: number;
  underlineColor: number;
  shadeColor: number;
  shadowColor: number;
  strikethroughColor: number;
  letterSpacing: number;
};

export type ParaShapeEntry = {
  alignment: number;
  lineSpacingType: number;
  lineSpacing: number;
  spaceBefore: number;
  spaceAfter: number;
  indent: number;
  leftMargin: number;
  rightMargin: number;
  headType: number;
  headLevel: number;
  numberingId: number;
};

export type BorderFillEntry = {
  leftType: number;
  rightType: number;
  topType: number;
  bottomType: number;
  leftWidth: number;
  rightWidth: number;
  topWidth: number;
  bottomWidth: number;
  leftColor: number;
  rightColor: number;
  topColor: number;
  bottomColor: number;
  fillType: number;
  fillColor: number;
};

export type BinDataEntry = { extension: string };

export type DocInfoTables = {
  fonts: IdTable<FontEntry>;
  charShapes: IdTable<CharShapeEntry>;
  paraShapes: IdTable<ParaShapeEntry>;
  borderFills: IdTable<BorderFillEntry>;
  binData: IdTable<BinDataEntry>;
  numberings: IdTable<{ format: string }>;
  bullets: IdTable<{ char: number }>;
};

/** DOCUMENT_PROPERTIES (26바이트): section_count=1 */
function buildDocumentProperties(): Uint8Array {
  const { buf, view } = allocate(26);
  view.setUint16(0, 1, true); // section_count = 1
  // 나머지 필드는 0 (시작번호 등)
  return buf;
}

/** ID_MAPPINGS (72바이트 = INT32 × 18) */
function buildIdMappings(tables: DocInfoTables): Uint8Array {
  const { buf, view } = allocate(72);
  view.setInt32(0, tables.binData.count, true); // [0] BinData
  for (let i = 1; i <= 7; i++) {
    view.setInt32(i * 4, tables.fonts.count, true); // [1-7] 폰트 (7개 언어)
  }
  view.setInt32(32, tables.borderFills.count, true); // [8] BorderFill
  view.setInt32(36, tables.charShapes.count, true); // [9] CharShape
  view.setInt32(40, 1, true); // [10] TabDef
  view.setInt32(44, tables.numberings.count, true); // [11] Numbering
  view.setInt32(48, tables.bullets.count, true); // [12] Bullet
  view.setInt32(52, tables.paraShapes.count, true); // [13] ParaShape
  view.setInt32(56, 1, true); // [14] Style
  // [15-17] MemoShape, TrackChange, TrackChangeAuthor = 0
  return buf;
}

/** FACE_NAME: flags=0x21, name + 기본 글꼴 */
function buildFaceName(name: string, defaultName: string): Uint8Array {
  const nameBytes = encodeUTF16LE(name);
  const defaultBytes = encodeUTF16LE(defaultName);
  const { buf, view } = allocate(3 + nameBytes.byteLength + 2 + defaultBytes.byteLength);
  view.setUint8(0, 0x21);
  view.setUint16(1, name.length, true);
  buf.set(nameBytes, 3);
  const offset = 3 + nameBytes.byteLength;
  view.setUint16(offset, defaultName.length, true);
  buf.set(defaultBytes, offset + 2);
  return buf;
}

/** BIN_DATA: flags(2) + binDataId(2) + extLen(2) + ext(UTF-16LE) */
function buildBinDataRecord(entry: BinDataEntry, index: number): Uint8Array {
  const extBytes = encodeUTF16LE(entry.extension);
  const { buf, view } = allocate(6 + extBytes.byteLength);
  view.setUint16(0, 0x00_01, true); // flags: EMBEDDING
  view.setUint16(2, index + 1, true); // binDataId (1-based)
  view.setUint16(4, entry.extension.length, true); // extension length
  buf.set(extBytes, 6);
  return buf;
}

/**
 * CHAR_SHAPE (74바이트) — 표 33
 * face_id[7](14) + font_scale[7](7) + char_spacing[7](7) + relative_size[7](7) + char_offset[7](7)
 * + base_size(4) + flags(4) + shadow_dx(1) + shadow_dy(1)
 * + text_color(4) + underline_color(4) + shade_color(4) + shadow_color(4)
 * + border_fill_id(2) + strikethrough_color(4)
 */
function buildCharShapeRecord(entry: CharShapeEntry): Uint8Array {
  const { buf, view } = allocate(74);
  let offset = 0;

  // 언어별 글꼴 ID (7×WORD = 14바이트)
  for (let i = 0; i < 7; i++) {
    view.setUint16(offset, entry.fontId, true);
    offset += 2;
  }

  // 언어별 장평 (7×UINT8 = 7바이트), 기본값 100%
  for (let i = 0; i < 7; i++) {
    view.setUint8(offset, 100);
    offset++;
  }

  // 언어별 자간 (7×INT8 = 7바이트), entry.letterSpacing 사용
  for (let i = 0; i < 7; i++) {
    view.setInt8(offset, entry.letterSpacing);
    offset++;
  }

  // 언어별 상대 크기 (7×UINT8 = 7바이트), 기본값 100%
  for (let i = 0; i < 7; i++) {
    view.setUint8(offset, 100);
    offset++;
  }

  // 언어별 글자 위치 (7×INT8 = 7바이트), 기본값 0
  offset += 7;

  // base_size (INT32, 4바이트) — pt × 100
  view.setInt32(offset, entry.baseSize, true);
  offset += 4;

  // flags (UINT32, 4바이트)
  let flags = 0;
  if (entry.italic) flags |= 0x01; // bit 0
  if (entry.bold) flags |= 0x02; // bit 1
  if (entry.underline) {
    flags |= 0x04; // bit 2-3: 글자 아래 (1)
    // bit 4-7 밑줄 모양: 0 = 기본 실선 (레퍼런스 확인됨)
  }
  if (entry.strikethrough) {
    flags |= 1 << 18; // bit 18-20: 취소선
    flags |= 1 << 26; // bit 26-29: 취소선 모양 실선
  }
  view.setUint32(offset, flags, true);
  offset += 4;

  // shadow dx, dy (각 1바이트)
  offset += 2;

  // text_color (COLORREF, 4바이트)
  view.setUint32(offset, entry.textColor, true);
  offset += 4;

  // underline_color
  view.setUint32(offset, entry.underlineColor, true);
  offset += 4;

  // shade_color
  view.setUint32(offset, entry.shadeColor, true);
  offset += 4;

  // shadow_color
  view.setUint32(offset, entry.shadowColor, true);
  offset += 4;

  // border_fill_id (UINT16, 2바이트)
  offset += 2;

  // strikethrough_color (COLORREF, 4바이트)
  view.setUint32(offset, entry.strikethroughColor, true);

  return buf;
}

/** PARA_SHAPE (58바이트) — 표 43: 레퍼런스 파일 기준 58바이트 (trailing 4바이트 포함) */
function buildParaShapeRecord(entry: ParaShapeEntry): Uint8Array {
  const { buf, view } = allocate(58);
  let offset = 0;

  // attr1 (UINT32, 4바이트) — 표 44
  let attr1 = 0;
  // bit 0-1: 줄 간격 종류 (한글 2007 이하) — 0 = 글자에 따라
  attr1 |= entry.lineSpacingType & 0x03;
  // bit 2-4: 정렬 방법
  attr1 |= (entry.alignment & 0x07) << 2;
  // bit 5-6: 줄 나눔 기준 영문 단위 — 2 = 글자
  attr1 |= 0x02 << 5;
  // bit 7: 줄 나눔 기준 한글 단위 — 1 = 글자
  attr1 |= 0x01 << 7;
  // bit 23-24: 문단 머리 모양 종류
  attr1 |= (entry.headType & 0x03) << 23;
  // bit 25-27: 문단 수준
  attr1 |= (entry.headLevel & 0x07) << 25;
  view.setUint32(offset, attr1, true);
  offset += 4;

  // 왼쪽 여백 (INT32)
  view.setInt32(offset, entry.leftMargin, true);
  offset += 4;

  // 오른쪽 여백 (INT32)
  view.setInt32(offset, entry.rightMargin, true);
  offset += 4;

  // 들여쓰기/내어쓰기 (INT32) — 양수=들여쓰기, 음수=내어쓰기
  view.setInt32(offset, entry.indent, true);
  offset += 4;

  // 문단 위 간격 (INT32)
  view.setInt32(offset, entry.spaceBefore, true);
  offset += 4;

  // 문단 아래 간격 (INT32)
  view.setInt32(offset, entry.spaceAfter, true);
  offset += 4;

  // 줄 간격 old (INT32)
  view.setInt32(offset, entry.lineSpacing, true);
  offset += 4;

  // 탭 정의 ID (UINT16) — 0 (기본 탭)
  offset += 2;

  // 번호/글머리표 ID (UINT16)
  view.setUint16(offset, entry.numberingId, true);
  offset += 2;

  // 테두리/배경 ID (UINT16) — 0
  offset += 2;

  // 문단 테두리 간격 4방향 (INT16 × 4 = 8바이트)
  offset += 8;

  // attr2 (UINT32, 4바이트) — 표 45
  offset += 4;

  // attr3 (UINT32, 4바이트) — 표 46: 줄 간격 종류 (5.0.2.5+)
  view.setUint32(offset, entry.lineSpacingType & 0x1f, true);
  offset += 4;

  // 줄 간격 new (INT32, 5.0.2.5+)
  view.setInt32(offset, entry.lineSpacing, true);

  return buf;
}

/** BORDER_FILL — 레퍼런스 HWP 파일 분석 기반
 * 인터리브 순서: (종류+굵기+색상) × 4면 + 대각선 + 채우기
 * 주의: 종류 값은 스펙(표 25)과 다름 — 0=없음, 1=실선 (스펙은 0=실선)
 */
function buildBorderFillRecord(entry: BorderFillEntry): Uint8Array {
  // no fill: 32 + 4(fillType) + 4(extended) = 40
  // solid fill: 32 + 4(fillType) + 12(solid) + 4(extended) + 1(extra) = 53
  const fillInfoSize = entry.fillType === 0 ? 8 : 21;
  const totalSize = 32 + fillInfoSize;
  const { buf, view } = allocate(totalSize);
  let offset = 0;

  // flags (UINT16)
  offset += 2;

  // 4방향 테두리선: 인터리브 순서 — (종류+굵기+색상) × 4면
  // left
  view.setUint8(offset, entry.leftType);
  view.setUint8(offset + 1, entry.leftWidth);
  view.setUint32(offset + 2, entry.leftColor, true);
  offset += 6;
  // right
  view.setUint8(offset, entry.rightType);
  view.setUint8(offset + 1, entry.rightWidth);
  view.setUint32(offset + 2, entry.rightColor, true);
  offset += 6;
  // top
  view.setUint8(offset, entry.topType);
  view.setUint8(offset + 1, entry.topWidth);
  view.setUint32(offset + 2, entry.topColor, true);
  offset += 6;
  // bottom
  view.setUint8(offset, entry.bottomType);
  view.setUint8(offset + 1, entry.bottomWidth);
  view.setUint32(offset + 2, entry.bottomColor, true);
  offset += 6;

  // 대각선: 한/글 기본값 type=1
  view.setUint8(offset, 1); // diag type
  offset += 2;

  // 대각선 색상 (COLORREF)
  offset += 4;

  // 채우기 정보
  view.setUint32(offset, entry.fillType, true);
  offset += 4;

  if (entry.fillType & 0x01) {
    // 단색 채우기
    view.setUint32(offset, entry.fillColor, true); // 배경색
    offset += 4;
    view.setUint32(offset, 0x00_00_00_00, true); // 무늬색
    offset += 4;
    view.setInt32(offset, -1, true); // 무늬 종류 (-1 = 없음)
  }

  return buf;
}

/** TAB_DEF: 기본 탭 (flags=0, count=0) */
function buildTabDefRecord(): Uint8Array {
  const { buf } = allocate(8);
  // flags(4) + count(2) = 6바이트 최소, 8바이트 align
  // 모두 0
  return buf;
}

/** NUMBERING: 순서 목록용 기본 번호 정의 */
function buildNumberingRecord(): Uint8Array {
  // 7개 레벨 × (속성(4) + 너비보정(2) + 거리(2) + charShapeId(4) + formatLen(2) + format + startNum(2))
  const parts: Uint8Array[] = [];
  for (let level = 0; level < 7; level++) {
    // 속성(4바이트) + 너비보정(2) + 거리(2) + charShapeId(4) = 12바이트
    const { buf: levelBuf, view: levelView } = allocate(12);
    levelView.setUint32(0, 0, true); // 속성: 정렬=왼쪽, auto=false
    levelView.setInt16(4, 0, true); // 너비보정
    levelView.setInt16(6, 0, true); // 거리
    levelView.setUint32(8, 0, true); // charShapeId

    // format string: "^1." → ^은 번호 자리표시자
    const formatStr = `\u0005${level + 1},1,1,1,1,1,1`;
    const formatBytes = encodeUTF16LE(formatStr);
    const { buf: fmtBuf, view: fmtView } = allocate(2 + formatBytes.byteLength + 2);
    fmtView.setUint16(0, formatStr.length, true);
    fmtBuf.set(formatBytes, 2);
    fmtView.setUint16(2 + formatBytes.byteLength, 1, true); // start number = 1

    parts.push(levelBuf, fmtBuf);
  }

  // 확장 레벨 (8~10, 3회 반복): 빈 문자열
  for (let i = 0; i < 3; i++) {
    const { buf: extBuf } = allocate(2);
    parts.push(extBuf);
  }

  return concat(...parts);
}

/** BULLET: 글머리표 */
function buildBulletRecord(entry: { char: number }): Uint8Array {
  const { buf, view } = allocate(20);
  // 문단 머리 정보 (8바이트) — 기본값
  // 글머리표 문자 (WCHAR, 2바이트)
  view.setUint16(8, entry.char, true);
  // 나머지 필드 (이미지 글머리표 ID(4) + 속성(4) + 체크문자(2)) = 0
  return buf;
}

/** STYLE: "바탕글"/"Normal" 하나만 등록 */
function buildStyleRecord(paraShapeId: number, charShapeId: number): Uint8Array {
  const korName = '바탕글';
  const engName = 'Normal';
  const korBytes = encodeUTF16LE(korName);
  const engBytes = encodeUTF16LE(engName);

  const size = 2 + korBytes.byteLength + 2 + engBytes.byteLength + 8;
  const { buf, view } = allocate(size);
  let offset = 0;

  // 한글 이름 길이 + 내용
  view.setUint16(offset, korName.length, true);
  offset += 2;
  buf.set(korBytes, offset);
  offset += korBytes.byteLength;

  // 영문 이름 길이 + 내용
  view.setUint16(offset, engName.length, true);
  offset += 2;
  buf.set(engBytes, offset);
  offset += engBytes.byteLength;

  // 속성: type=0 (문단 스타일)
  view.setUint8(offset, 0);
  offset++;

  // 다음 스타일 ID
  view.setUint8(offset, 0);
  offset++;

  // 언어 ID (한글: 1042)
  view.setInt16(offset, 1042, true);
  offset += 2;

  // 문단 모양 ID
  view.setUint16(offset, paraShapeId, true);
  offset += 2;

  // 글자 모양 ID
  view.setUint16(offset, charShapeId, true);

  return buf;
}

/** DocInfo 스트림 전체를 조립 */
export function buildDocInfoStream(tables: DocInfoTables): Uint8Array {
  const records: Uint8Array[] = [
    makeRecord(HWPTAG.DOCUMENT_PROPERTIES, 0, buildDocumentProperties()),
    makeRecord(HWPTAG.ID_MAPPINGS, 0, buildIdMappings(tables)),
  ];

  // Level 1: BIN_DATA
  const binDataEntries = tables.binData.getAll();
  for (const [i, binDataEntry] of binDataEntries.entries()) {
    records.push(makeRecord(HWPTAG.BIN_DATA, 1, buildBinDataRecord(binDataEntry, i)));
  }

  // Level 1: FACE_NAME — language-major 순서 (한글 font#0..N, 영문 font#0..N, ...)
  const fontEntries = tables.fonts.getAll();
  for (let lang = 0; lang < 7; lang++) {
    for (const entry of fontEntries) {
      const psName = entry.postScriptName;
      records.push(makeRecord(HWPTAG.FACE_NAME, 1, buildFaceName(entry.name, psName)));
    }
  }

  // Level 1: BORDER_FILL
  for (const entry of tables.borderFills.getAll()) {
    records.push(makeRecord(HWPTAG.BORDER_FILL, 1, buildBorderFillRecord(entry)));
  }

  // Level 1: CHAR_SHAPE
  for (const entry of tables.charShapes.getAll()) {
    records.push(makeRecord(HWPTAG.CHAR_SHAPE, 1, buildCharShapeRecord(entry)));
  }

  // Level 1: TAB_DEF
  records.push(makeRecord(HWPTAG.TAB_DEF, 1, buildTabDefRecord()));

  // Level 1: NUMBERING
  for (let i = 0; i < tables.numberings.count; i++) {
    records.push(makeRecord(HWPTAG.NUMBERING, 1, buildNumberingRecord()));
  }

  // Level 1: BULLET
  for (const entry of tables.bullets.getAll()) {
    records.push(makeRecord(HWPTAG.BULLET, 1, buildBulletRecord(entry)));
  }

  // Level 1: PARA_SHAPE
  for (const entry of tables.paraShapes.getAll()) {
    records.push(makeRecord(HWPTAG.PARA_SHAPE, 1, buildParaShapeRecord(entry)));
  }

  // Level 1: STYLE — "바탕글"/"Normal" 1개, paraShapeId=0, charShapeId=0 참조
  records.push(makeRecord(HWPTAG.STYLE, 1, buildStyleRecord(0, 0)));

  return concat(...records);
}
