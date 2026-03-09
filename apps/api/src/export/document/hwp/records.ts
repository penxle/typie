// spell-checker:words HWPTAG HWPUNIT rrggbb bbggrr
import { deflateRawSync } from 'node:zlib';

// HWP 태그 ID 상수 (HWPTAG_BEGIN = 0x010)
export const HWPTAG = {
  DOCUMENT_PROPERTIES: 0x0_10,
  ID_MAPPINGS: 0x0_11,
  BIN_DATA: 0x0_12,
  FACE_NAME: 0x0_13,
  BORDER_FILL: 0x0_14,
  CHAR_SHAPE: 0x0_15,
  TAB_DEF: 0x0_16,
  NUMBERING: 0x0_17,
  BULLET: 0x0_18,
  PARA_SHAPE: 0x0_19,
  STYLE: 0x0_1a,
  PARA_HEADER: 0x0_42,
  PARA_TEXT: 0x0_43,
  PARA_CHAR_SHAPE: 0x0_44,
  PARA_LINE_SEG: 0x0_45,
  PARA_RANGE_TAG: 0x0_46,
  CTRL_HEADER: 0x0_47,
  LIST_HEADER: 0x0_48,
  PAGE_DEF: 0x0_49,
  FOOTNOTE_SHAPE: 0x0_4a,
  PAGE_BORDER_FILL: 0x0_4b,
  SHAPE_COMPONENT: 0x0_4c,
  TABLE: 0x0_4d,
  SHAPE_COMPONENT_LINE: 0x0_4e,
  SHAPE_COMPONENT_RECTANGLE: 0x0_4f,
  SHAPE_COMPONENT_PICTURE: 0x0_55,
} as const;

/** 레코드 헤더 (4바이트) + 데이터를 결합한 바이너리 생성 */
export function makeRecord(tagId: number, level: number, data: Uint8Array): Uint8Array {
  const size = data.byteLength;
  const needsExtended = size >= 0xf_ff;
  const headerSize = needsExtended ? 8 : 4;
  const result = new Uint8Array(headerSize + size);
  const view = new DataView(result.buffer);

  const sizeField = needsExtended ? 0xf_ff : size;
  const header = (tagId & 0x3_ff) | ((level & 0x3_ff) << 10) | ((sizeField & 0xf_ff) << 20);
  view.setUint32(0, header, true);

  if (needsExtended) {
    view.setUint32(4, size, true);
  }

  result.set(data, headerSize);
  return result;
}

/** 여러 레코드/버퍼를 하나의 Uint8Array로 합침 */
export function concat(...buffers: Uint8Array[]): Uint8Array {
  const totalLength = buffers.reduce((sum, b) => sum + b.byteLength, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const buf of buffers) {
    result.set(buf, offset);
    offset += buf.byteLength;
  }
  return result;
}

/** zlib raw deflate 압축 */
export function compressStream(data: Uint8Array): Uint8Array {
  return new Uint8Array(deflateRawSync(data));
}

/** UTF-16LE 인코딩 */
export function encodeUTF16LE(str: string): Uint8Array {
  const buf = new Uint8Array(str.length * 2);
  const view = new DataView(buf.buffer);
  for (let i = 0; i < str.length; i++) {
    view.setUint16(i * 2, str.codePointAt(i) ?? 0, true);
  }
  return buf;
}

/** px → HWPUNIT (1px = 75 HWPUNIT = 75/7200 inch) */
export const pxToHwpunit = (px: number): number => Math.round(px * 75);

/** #rrggbb hex → HWP COLORREF (0x00bbggrr) */
export function hexToColorref(hex: string): number {
  const clean = hex.replace('#', '');
  const r = Number.parseInt(clean.slice(0, 2), 16);
  const g = Number.parseInt(clean.slice(2, 4), 16);
  const b = Number.parseInt(clean.slice(4, 6), 16);
  return (b << 16) | (g << 8) | r;
}

/** ctrl_id 문자열 → UINT32 (MAKE_4CHID 매크로: 첫 문자가 상위 바이트) */
export function ctrlId(str: string): number {
  return (
    ((str.codePointAt(0) ?? 0) << 24) | ((str.codePointAt(1) ?? 0) << 16) | ((str.codePointAt(2) ?? 0) << 8) | (str.codePointAt(3) ?? 0)
  );
}

/** 고정 크기 버퍼를 할당하고 DataView와 함께 반환 */
export function allocate(size: number): { buf: Uint8Array; view: DataView } {
  const buf = new Uint8Array(size);
  const view = new DataView(buf.buffer);
  return { buf, view };
}

/** 중복 제거하며 0-based ID를 부여하는 테이블 */
export class IdTable<T> {
  private map = new Map<string, number>();
  private items: T[] = [];

  intern(item: T, key: string): number {
    const existing = this.map.get(key);
    if (existing !== undefined) return existing;
    const id = this.items.length;
    this.map.set(key, id);
    this.items.push(item);
    return id;
  }

  getId(key: string): number | undefined {
    return this.map.get(key);
  }

  get count(): number {
    return this.items.length;
  }

  getAll(): T[] {
    return this.items;
  }
}
