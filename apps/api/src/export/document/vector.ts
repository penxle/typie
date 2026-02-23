import { parseVectorPageBinary } from './vector-codec';
import type { Editor } from '@typie/editor';

export type VectorPathCommand =
  | { type: 'moveTo'; x: number; y: number }
  | { type: 'lineTo'; x: number; y: number }
  | { type: 'quadTo'; cx: number; cy: number; x: number; y: number }
  | { type: 'cubicTo'; c1x: number; c1y: number; c2x: number; c2y: number; x: number; y: number }
  | { type: 'closePath' };

export type VectorOp =
  | { type: 'fillPath'; path: VectorPathCommand[]; color: [number, number, number, number]; fillRule: 'winding' | 'evenOdd' }
  | {
      type: 'strokePath';
      path: VectorPathCommand[];
      color: [number, number, number, number];
      width: number;
      lineCap: 'butt' | 'round' | 'square';
      lineJoin: 'miter' | 'round' | 'bevel';
    };

export type VectorExternalData =
  | { type: 'image'; id?: string; proportion: number; uploadId?: string }
  | { type: 'file'; id?: string; uploadId?: string }
  | { type: 'embed'; id?: string }
  | { type: 'archived'; id?: string };

export type VectorExternalElement = {
  nodeId: string;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  data: VectorExternalData;
};

export type VectorPage = {
  width: number;
  height: number;
  ops: VectorOp[];
  externalElements: VectorExternalElement[];
};

const DATA_TAG_IMAGE = 0;
const DATA_TAG_FILE = 1;
const DATA_TAG_EMBED = 2;
const DATA_TAG_ARCHIVED = 3;

const align4 = (n: number): number => (n + 3) & ~3;

const readStr = (view: DataView, offset: number): { value: string; end: number } => {
  const byteLen = view.getUint32(offset, true);
  const bytes = new Uint8Array(view.buffer, offset + 4, byteLen);
  const value = new TextDecoder().decode(bytes);
  return { value, end: offset + 4 + align4(byteLen) };
};

const readExternalElementsByPage = (
  editor: Editor,
  pageCount: number,
  offsets: Record<string, number>,
  memory: WebAssembly.Memory,
): Map<number, VectorExternalElement[]> => {
  const externalByPage = new Map<number, VectorExternalElement[]>();
  const view = new DataView(memory.buffer);
  const slatePtr = editor.getSlatePtr();
  const slabPtr = editor.getSlabPtr();

  const count = view.getUint32(slatePtr + offsets.external_elements_count, true);
  const offset = view.getUint32(slatePtr + offsets.external_elements_offset, true);
  let pos = slabPtr + offset;

  for (let i = 0; i < count; i++) {
    const pageIdx = view.getUint32(pos, true);
    pos += 4;

    const { value: nodeId, end: afterNodeId } = readStr(view, pos);
    pos = afterNodeId;

    const bounds = {
      x: view.getFloat32(pos, true),
      y: view.getFloat32(pos + 4, true),
      width: view.getFloat32(pos + 8, true),
      height: view.getFloat32(pos + 12, true),
    };
    pos += 16;

    pos += 4; // isSelected

    const dataTag = view.getUint32(pos, true);
    pos += 4;

    let data: VectorExternalData;
    if (dataTag === DATA_TAG_IMAGE) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      const { value: uploadId, end: afterUploadId } = readStr(view, pos);
      pos = afterUploadId;
      const proportion = view.getFloat32(pos, true);
      pos += 4;
      data = { type: 'image', id: id || undefined, uploadId: uploadId || undefined, proportion };
    } else if (dataTag === DATA_TAG_FILE) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      const { value: uploadId, end: afterUploadId } = readStr(view, pos);
      pos = afterUploadId;
      data = { type: 'file', id: id || undefined, uploadId: uploadId || undefined };
    } else if (dataTag === DATA_TAG_EMBED) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      data = { type: 'embed', id: id || undefined };
    } else if (dataTag === DATA_TAG_ARCHIVED) {
      const { value: id, end: afterId } = readStr(view, pos);
      pos = afterId;
      data = { type: 'archived', id: id || undefined };
    } else {
      throw new Error(`Unknown external data tag: ${dataTag}`);
    }

    if (pageIdx >= pageCount) {
      continue;
    }

    const list = externalByPage.get(pageIdx) ?? [];
    list.push({ nodeId, bounds, data });
    externalByPage.set(pageIdx, list);
  }

  return externalByPage;
};

export function exportDocumentVectorPages(
  editor: Editor,
  pageCount: number,
  offsets: Record<string, number>,
  memory: WebAssembly.Memory,
): VectorPage[] {
  const pages: VectorPage[] = [];
  const externalByPage = readExternalElementsByPage(editor, pageCount, offsets, memory);

  for (let i = 0; i < pageCount; i++) {
    const bytes = editor.exportPageVector(i);
    if (!bytes) {
      throw new Error(`Missing vector page payload for page index ${i}`);
    }

    const page = parseVectorPageBinary(bytes);
    pages.push({
      width: page.width,
      height: page.height,
      ops: page.ops,
      externalElements: externalByPage.get(i) ?? [],
    });
  }

  return pages;
}
