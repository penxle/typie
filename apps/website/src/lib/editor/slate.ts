import type { ExternalElement, Mark, MarkType, Position, Rect, SelectionStats, TextAlign, TextBound } from './types';

export type TableOverlay = {
  pageIdx: number;
  tableId: string;
  bounds: Rect;
  borderStyle: string;
  align: string;
  colWidths: number[];
  colPositions: number[];
  rowHeights: number[];
  rowPositions: number[];
  startRowIndex: number;
  totalRows: number;
  isFocused: boolean;
};

export const DIRTY_SETTINGS = 0;
export const DIRTY_LAYOUT = 1;
export const DIRTY_CURSOR = 2;
export const DIRTY_SELECTION = 3;
export const DIRTY_FORMATTING = 4;
export const DIRTY_POINTER = 5;
export const DIRTY_PLACEHOLDER = 7;
export const DIRTY_EXTERNAL_ELEMENTS = 8;
export const DIRTY_ENABLED_ACTIONS = 9;
export const DIRTY_LINK_OVERLAYS = 10;
export const DIRTY_TRACKED_ITEMS = 11;
export const DIRTY_TABLE_OVERLAYS = 14;
export const DIRTY_DOC_CHANGED = 15;
export const DIRTY_RENDER_REQUIRED = 16;
export const DIRTY_FONT_REQUIRED = 17;
export const DIRTY_FALLBACK_FONT_REQUIRED = 18;
export const DIRTY_EXITED_DOCUMENT_START = 19;
export const DIRTY_HTML_PASTED = 20;

export const POINTER_STATE_IDLE = 0;
export const POINTER_STATE_PRESSED = 1;
export const POINTER_STATE_DRAGGING_CONTENT = 2;
export const POINTER_STATE_DRAGGING_EXTERNAL = 3;
export const POINTER_STATE_DRAGGING_SELECTION = 4;

const MARK_TYPE_NAMES: MarkType[] = [
  'background_color',
  'text_color',
  'font_size',
  'font_family',
  'font_weight',
  'italic',
  'letter_spacing',
  'link',
  'ruby',
  'strikethrough',
  'underline',
];

const POINTER_STYLES = ['default', 'text', 'pointer'] as const;

const AFFINITY_MAP = ['upstream', 'downstream'] as const;

type SlateOffsets = Record<string, number>;

export class SlateReader {
  #memory: WebAssembly.Memory;
  #offsets: SlateOffsets;
  #slatePtr: number;
  #slabPtr: number;
  #buffer: ArrayBuffer;
  #view: DataView;
  #slabView: DataView;
  #dirtyLo = 0;
  #dirtyHi = 0;

  constructor(memory: WebAssembly.Memory, offsets: SlateOffsets, slatePtr: number, slabPtr: number) {
    this.#memory = memory;
    this.#offsets = offsets;
    this.#slatePtr = slatePtr;
    this.#slabPtr = slabPtr;
    this.#buffer = memory.buffer;
    this.#view = new DataView(memory.buffer);
    this.#slabView = new DataView(memory.buffer);
  }

  refresh(slatePtr: number, slabPtr: number): void {
    this.#slatePtr = slatePtr;
    this.#slabPtr = slabPtr;
    if (this.#buffer !== this.#memory.buffer) {
      this.#buffer = this.#memory.buffer;
      this.#view = new DataView(this.#buffer);
      this.#slabView = new DataView(this.#buffer);
    }
    const lo = this.#view.getUint32(this.#slatePtr + this.#offsets.dirty, true);
    const hi = this.#view.getUint32(this.#slatePtr + this.#offsets.dirty + 4, true);
    this.#dirtyLo = lo;
    this.#dirtyHi = hi;
  }

  isDirty(bit: number): boolean {
    if (bit < 32) return (this.#dirtyLo & (1 << bit)) !== 0;
    return (this.#dirtyHi & (1 << (bit - 32))) !== 0;
  }

  get hasDirty(): boolean {
    return this.#dirtyLo !== 0 || this.#dirtyHi !== 0;
  }

  #u32(field: string): number {
    return this.#view.getUint32(this.#slatePtr + this.#offsets[field], true);
  }

  #i32(field: string): number {
    return this.#view.getInt32(this.#slatePtr + this.#offsets[field], true);
  }

  #f32(field: string): number {
    return this.#view.getFloat32(this.#slatePtr + this.#offsets[field], true);
  }

  #nodeId(field: string): string {
    const base = this.#slatePtr + this.#offsets[field];
    let hex = '';
    for (let i = 0; i < 16; i++) {
      hex += this.#view
        .getUint8(base + i)
        .toString(16)
        .padStart(2, '0');
    }
    return hex;
  }

  readSettings(): { paragraphIndent: number; blockGap: number } {
    return {
      paragraphIndent: this.#f32('paragraph_indent'),
      blockGap: this.#f32('block_gap'),
    };
  }

  readLayout(): {
    pages: { width: number; height: number }[];
    layoutMode:
      | {
          type: 'paginated';
          pageWidth: number;
          pageHeight: number;
          pageMarginTop: number;
          pageMarginBottom: number;
          pageMarginLeft: number;
          pageMarginRight: number;
        }
      | { type: 'continuous'; maxWidth: number };
  } {
    const count = this.#u32('pages_count');
    const raw = readF32Array(this.#slabView, this.#slabPtr + this.#u32('pages_offset'), count * 2);
    const pages: { width: number; height: number }[] = [];
    for (let i = 0; i < count; i++) pages.push({ width: raw[i * 2], height: raw[i * 2 + 1] });

    let lmPos = this.#slabPtr + this.#u32('layout_mode_offset');
    const tag = this.#slabView.getUint32(lmPos, true);
    lmPos += 4;

    const layoutMode =
      tag === 0
        ? {
            type: 'paginated' as const,
            pageWidth: this.#slabView.getFloat32(lmPos, true),
            pageHeight: this.#slabView.getFloat32(lmPos + 4, true),
            pageMarginTop: this.#slabView.getFloat32(lmPos + 8, true),
            pageMarginBottom: this.#slabView.getFloat32(lmPos + 12, true),
            pageMarginLeft: this.#slabView.getFloat32(lmPos + 16, true),
            pageMarginRight: this.#slabView.getFloat32(lmPos + 20, true),
          }
        : {
            type: 'continuous' as const,
            maxWidth: this.#slabView.getFloat32(lmPos, true),
          };

    return {
      pages,
      layoutMode,
    };
  }

  readCursor(): {
    pageIdx: number;
    bounds: Rect | null;
    visible: boolean;
  } {
    const pageIdx = this.#i32('cursor_page_idx');
    if (pageIdx < 0) {
      return { pageIdx: -1, bounds: null, visible: false };
    }

    const bounds: Rect = {
      x: this.#f32('cursor_x'),
      y: this.#f32('cursor_y'),
      width: this.#f32('cursor_width'),
      height: this.#f32('cursor_height'),
    };

    return {
      pageIdx,
      bounds,
      visible: this.#u32('cursor_visible') !== 0,
    };
  }

  readSelection(): {
    collapsed: boolean;
    anchor: Position;
    head: Position;
    stats: SelectionStats;
    fromHandle: { pageIdx: number; bounds: Rect } | null;
    toHandle: { pageIdx: number; bounds: Rect } | null;
  } {
    const cmp = this.#i32('selection_cmp');
    const collapsed = cmp === 0;

    const anchorAffinity = this.#u32('selection_anchor_affinity');
    const headAffinity = this.#u32('selection_head_affinity');

    const anchor: Position = {
      nodeId: this.#nodeId('selection_anchor_node_id'),
      offset: this.#u32('selection_anchor_offset'),
      affinity: AFFINITY_MAP[anchorAffinity] ?? 'downstream',
    };

    const head: Position = {
      nodeId: this.#nodeId('selection_head_node_id'),
      offset: this.#u32('selection_head_offset'),
      affinity: AFFINITY_MAP[headAffinity] ?? 'downstream',
    };

    const alignVal = this.#i32('formatting_uniform_align');
    const uniformAlign: TextAlign | undefined =
      alignVal === 0 ? 'left' : alignVal === 1 ? 'center' : alignVal === 2 ? 'right' : alignVal === 3 ? 'justify' : undefined;

    const lh = this.#f32('formatting_uniform_line_height');
    const uniformLineHeight = lh < 0 || Number.isNaN(lh) ? undefined : lh;

    const anchorPageIdx = this.#i32('selection_anchor_page_idx');
    const fromHandle =
      anchorPageIdx < 0
        ? null
        : {
            pageIdx: anchorPageIdx,
            bounds: {
              x: this.#f32('selection_anchor_x'),
              y: this.#f32('selection_anchor_y'),
              width: this.#f32('selection_anchor_width'),
              height: this.#f32('selection_anchor_height'),
            },
          };

    const headPageIdx = this.#i32('selection_head_page_idx');
    const toHandle =
      headPageIdx < 0
        ? null
        : {
            pageIdx: headPageIdx,
            bounds: {
              x: this.#f32('selection_head_x'),
              y: this.#f32('selection_head_y'),
              width: this.#f32('selection_head_width'),
              height: this.#f32('selection_head_height'),
            },
          };

    return {
      collapsed,
      anchor,
      head,
      stats: {
        uniformAlign,
        uniformLineHeight,
      },
      fromHandle,
      toHandle,
    };
  }

  readActiveMarks(): { uniformMarks: Mark[]; mixedMarks: MarkType[] } {
    const count = this.#u32('formatting_uniform_marks_count');
    const offset = this.#u32('formatting_uniform_marks_offset');
    const uniformMarks = readUniformMarks(this.#slabView, this.#slabPtr + offset, count);

    const bitfield = this.#u32('formatting_mixed_marks_bitfield');
    const mixedMarks: MarkType[] = [];
    for (const [i, MARK_TYPE_NAME] of MARK_TYPE_NAMES.entries()) {
      if (bitfield & (1 << i)) {
        mixedMarks.push(MARK_TYPE_NAME);
      }
    }

    return { uniformMarks, mixedMarks };
  }

  readPointerStyle(): string {
    return POINTER_STYLES[this.#u32('pointer_style')] ?? 'default';
  }

  readPointerState(): number {
    return this.#u32('pointer_state');
  }

  readPlaceholder(): { visible: boolean; bounds: Rect | null } {
    const visible = this.#u32('placeholder_visible') !== 0;
    if (!visible) {
      return { visible: false, bounds: null };
    }
    return {
      visible: true,
      bounds: {
        x: this.#f32('placeholder_x'),
        y: this.#f32('placeholder_y'),
        width: this.#f32('placeholder_width'),
        height: this.#f32('placeholder_height'),
      },
    };
  }

  readEnabledActions(): string[] {
    const count = this.#u32('enabled_actions_count');
    const offset = this.#u32('enabled_actions_offset');
    return readStringArray(this.#slabView, this.#slabPtr + offset, count);
  }

  readExternalElements(): ExternalElement[] {
    const count = this.#u32('external_elements_count');
    const offset = this.#u32('external_elements_offset');
    return readExternalElements(this.#slabView, this.#slabPtr + offset, count);
  }

  readLinkOverlays(): { pageIdx: number; href: string; bounds: TextBound[] }[] {
    const count = this.#u32('link_overlays_count');
    const offset = this.#u32('link_overlays_offset');
    return readLinkOverlays(this.#slabView, this.#slabPtr + offset, count);
  }

  readTrackedItems(): TrackedItemOverlay[] {
    const count = this.#u32('tracked_items_count');
    const offset = this.#u32('tracked_items_offset');
    return readTrackedItems(this.#slabView, this.#slabPtr + offset, count);
  }

  readTableOverlays(): TableOverlay[] {
    const count = this.#u32('table_overlays_count');
    const offset = this.#u32('table_overlays_offset');
    return readTableOverlays(this.#slabView, this.#slabPtr + offset, count);
  }

  readFontRequests(): { family: string; weight: number; codepoints: number[] }[] {
    const count = this.#u32('font_requests_count');
    const offset = this.#u32('font_requests_offset');
    return readFontRequests(this.#slabView, this.#slabPtr + offset, count);
  }

  readFallbackCodepoints(): number[] {
    const count = this.#u32('fallback_codepoints_count');
    const offset = this.#u32('fallback_codepoints_offset');
    return readU32Array(this.#slabView, this.#slabPtr + offset, count);
  }

  readHtmlPasted(): { text: string; from: Position; to: Position } {
    const offset = this.#u32('html_pasted_offset');
    return readHtmlPasted(this.#slabView, this.#slabPtr + offset);
  }
}

function align4(n: number): number {
  return (n + 3) & ~3;
}

export function readStr(view: DataView, offset: number): { value: string; end: number } {
  const byteLen = view.getUint32(offset, true);
  const bytes = new Uint8Array(view.buffer, offset + 4, byteLen);
  const value = new TextDecoder().decode(bytes);
  return { value, end: offset + 4 + align4(byteLen) };
}

export function readF32Array(view: DataView, offset: number, count: number): number[] {
  const result: number[] = [];
  for (let i = 0; i < count; i++) {
    result.push(view.getFloat32(offset + i * 4, true));
  }
  return result;
}

export function readU32Array(view: DataView, offset: number, count: number): number[] {
  const result: number[] = [];
  for (let i = 0; i < count; i++) {
    result.push(view.getUint32(offset + i * 4, true));
  }
  return result;
}

function readTextBounds(view: DataView, offset: number, count: number): { bounds: TextBound[]; end: number } {
  const bounds: TextBound[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    bounds.push({
      x: view.getFloat32(pos, true),
      y: view.getFloat32(pos + 4, true),
      width: view.getFloat32(pos + 8, true),
      height: view.getFloat32(pos + 12, true),
      ascent: view.getFloat32(pos + 16, true),
    });
    pos += 20;
  }
  return { bounds, end: pos };
}

function readStringArray(view: DataView, offset: number, count: number): string[] {
  const result: string[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const { value, end } = readStr(view, pos);
    result.push(value);
    pos = end;
  }
  return result;
}

function readUniformMarks(view: DataView, offset: number, count: number): Mark[] {
  const marks: Mark[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const typeTag = view.getUint32(pos, true);
    const valueKind = view.getUint32(pos + 4, true);
    pos += 8;

    const markType = MARK_TYPE_NAMES[typeTag] ?? 'background_color';

    let strValue = '';
    let numValue = 0;

    if (valueKind === 0) {
      // unit
    } else if (valueKind === 1) {
      numValue = view.getFloat32(pos, true);
      pos += 4;
    } else if (valueKind === 2) {
      numValue = view.getUint32(pos, true);
      pos += 4;
    } else if (valueKind === 3) {
      const { value: sv, end } = readStr(view, pos);
      strValue = sv;
      pos = end;
    }

    let mark: Mark;
    switch (typeTag) {
      case 0: {
        mark = { type: 'background_color', key: strValue };
        break;
      }
      case 1: {
        mark = { type: 'text_color', key: strValue };
        break;
      }
      case 2: {
        mark = { type: 'font_size', size: numValue };
        break;
      }
      case 3: {
        mark = { type: 'font_family', family: strValue };
        break;
      }
      case 4: {
        mark = { type: 'font_weight', weight: numValue };
        break;
      }
      case 5: {
        mark = { type: 'italic' } as unknown as Mark;
        break;
      }
      case 6: {
        mark = { type: 'letter_spacing', spacing: numValue };
        break;
      }
      case 7: {
        mark = { type: 'link', href: strValue };
        break;
      }
      case 8: {
        mark = { type: 'ruby', text: strValue };
        break;
      }
      case 9: {
        mark = { type: 'strikethrough' } as unknown as Mark;
        break;
      }
      case 10: {
        mark = { type: 'underline' } as unknown as Mark;
        break;
      }
      default: {
        mark = { type: markType } as unknown as Mark;
        break;
      }
    }

    marks.push(mark);
  }
  return marks;
}

function readExternalElements(view: DataView, offset: number, count: number): ExternalElement[] {
  const elements: ExternalElement[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const pageIdx = view.getUint32(pos, true);
    pos += 4;

    const { value: nodeId, end: afterNodeId } = readStr(view, pos);
    pos = afterNodeId;

    const bounds: Rect = {
      x: view.getFloat32(pos, true),
      y: view.getFloat32(pos + 4, true),
      width: view.getFloat32(pos + 8, true),
      height: view.getFloat32(pos + 12, true),
    };
    pos += 16;

    const isSelected = view.getUint32(pos, true) !== 0;
    pos += 4;

    const dataTag = view.getUint32(pos, true);
    pos += 4;

    let data: ExternalElement['data'];
    if (dataTag === 0) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      const { value: uploadId, end: afterUploadId } = readStr(view, pos);
      pos = afterUploadId;
      const proportion = view.getFloat32(pos, true);
      pos += 4;
      data = { type: 'image', id: id || undefined, uploadId: uploadId || undefined, proportion };
    } else if (dataTag === 1) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      const { value: uploadId, end: afterUploadId } = readStr(view, pos);
      pos = afterUploadId;
      data = { type: 'file', id: id || undefined, uploadId: uploadId || undefined };
    } else if (dataTag === 2) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      data = { type: 'embed', id: id || undefined };
    } else {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      data = { type: 'archived', id: id || undefined };
    }

    elements.push({ pageIdx, nodeId, bounds, data, isSelected });
  }
  return elements;
}

function readLinkOverlays(view: DataView, offset: number, count: number): { pageIdx: number; href: string; bounds: TextBound[] }[] {
  const overlays: { pageIdx: number; href: string; bounds: TextBound[] }[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const pageIdx = view.getUint32(pos, true);
    pos += 4;

    const { value: href, end: afterHref } = readStr(view, pos);
    pos = afterHref;

    const boundsCount = view.getUint32(pos, true);
    pos += 4;

    const { bounds, end: afterBounds } = readTextBounds(view, pos, boundsCount);
    pos = afterBounds;

    overlays.push({ pageIdx, href, bounds });
  }
  return overlays;
}

export type TrackedItemOverlay = {
  pageIdx: number;
  group: number;
  id: string;
  nodeId: string;
  startOffset: number;
  endOffset: number;
  bounds: TextBound[];
};

function readTrackedItems(view: DataView, offset: number, count: number): TrackedItemOverlay[] {
  const overlays: TrackedItemOverlay[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const pageIdx = view.getUint32(pos, true);
    pos += 4;

    const group = view.getUint32(pos, true);
    pos += 4;

    const { value: id, end: afterId } = readStr(view, pos);
    pos = afterId;

    const { nodeId, end: afterNodeId } = readNodeIdFromSlab(view, pos);
    pos = afterNodeId;

    const startOffset = view.getUint32(pos, true);
    const endOffset = view.getUint32(pos + 4, true);
    pos += 8;

    const boundsCount = view.getUint32(pos, true);
    pos += 4;

    const { bounds, end: afterBounds } = readTextBounds(view, pos, boundsCount);
    pos = afterBounds;

    overlays.push({ pageIdx, group, id, nodeId, startOffset, endOffset, bounds });
  }
  return overlays;
}

function readTableOverlays(view: DataView, offset: number, count: number): TableOverlay[] {
  const overlays: TableOverlay[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const pageIdx = view.getUint32(pos, true);
    pos += 4;

    const { value: tableId, end: afterTableId } = readStr(view, pos);
    pos = afterTableId;

    const bounds: Rect = {
      x: view.getFloat32(pos, true),
      y: view.getFloat32(pos + 4, true),
      width: view.getFloat32(pos + 8, true),
      height: view.getFloat32(pos + 12, true),
    };
    pos += 16;

    const { value: borderStyle, end: afterBorderStyle } = readStr(view, pos);
    pos = afterBorderStyle;

    const { value: align, end: afterAlign } = readStr(view, pos);
    pos = afterAlign;

    const startRowIndex = view.getUint32(pos, true);
    const totalRows = view.getUint32(pos + 4, true);
    const isFocused = view.getUint32(pos + 8, true) !== 0;
    pos += 12;

    const cwCnt = view.getUint32(pos, true);
    pos += 4;
    const colWidths = readF32Array(view, pos, cwCnt);
    pos += cwCnt * 4;

    const cpCnt = view.getUint32(pos, true);
    pos += 4;
    const colPositions = readF32Array(view, pos, cpCnt);
    pos += cpCnt * 4;

    const rhCnt = view.getUint32(pos, true);
    pos += 4;
    const rowHeights = readF32Array(view, pos, rhCnt);
    pos += rhCnt * 4;

    const rpCnt = view.getUint32(pos, true);
    pos += 4;
    const rowPositions = readF32Array(view, pos, rpCnt);
    pos += rpCnt * 4;

    overlays.push({
      pageIdx,
      tableId,
      bounds,
      borderStyle,
      align,
      colWidths,
      colPositions,
      rowHeights,
      rowPositions,
      startRowIndex,
      totalRows,
      isFocused,
    });
  }
  return overlays;
}

function readFontRequests(view: DataView, offset: number, count: number): { family: string; weight: number; codepoints: number[] }[] {
  const requests: { family: string; weight: number; codepoints: number[] }[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const { value: family, end: afterFamily } = readStr(view, pos);
    pos = afterFamily;

    const weight = view.getUint32(pos, true);
    pos += 4;

    const cpCount = view.getUint32(pos, true);
    pos += 4;

    const codepoints = readU32Array(view, pos, cpCount);
    pos += cpCount * 4;

    requests.push({ family, weight, codepoints });
  }
  return requests;
}

function readNodeIdFromSlab(view: DataView, offset: number): { nodeId: string; end: number } {
  const byteLen = view.getUint32(offset, true);
  let hex = '';
  for (let i = 0; i < byteLen; i++) {
    hex += view
      .getUint8(offset + 4 + i)
      .toString(16)
      .padStart(2, '0');
  }
  return { nodeId: hex, end: offset + 4 + align4(byteLen) };
}

function readHtmlPasted(view: DataView, offset: number): { text: string; from: Position; to: Position } {
  let pos = offset;

  const { value: text, end: afterText } = readStr(view, pos);
  pos = afterText;

  const { nodeId: fromNodeId, end: afterFromNode } = readNodeIdFromSlab(view, pos);
  pos = afterFromNode;

  const fromOffset = view.getUint32(pos, true);
  const fromAffinity = view.getUint32(pos + 4, true);
  pos += 8;

  const { nodeId: toNodeId, end: afterToNode } = readNodeIdFromSlab(view, pos);
  pos = afterToNode;

  const toOffset = view.getUint32(pos, true);
  const toAffinity = view.getUint32(pos + 4, true);

  return {
    text,
    from: {
      nodeId: fromNodeId,
      offset: fromOffset,
      affinity: AFFINITY_MAP[fromAffinity] ?? 'downstream',
    },
    to: {
      nodeId: toNodeId,
      offset: toOffset,
      affinity: AFFINITY_MAP[toAffinity] ?? 'downstream',
    },
  };
}
