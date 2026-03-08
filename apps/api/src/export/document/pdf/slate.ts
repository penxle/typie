export type ExternalData =
  | { type: 'image'; id?: string; proportion: number; uploadId?: string }
  | { type: 'file'; id?: string; uploadId?: string }
  | { type: 'embed'; id?: string }
  | { type: 'archived'; id?: string };

export type ExternalElement = {
  pageIdx: number;
  nodeId: string;
  bounds: { x: number; y: number; width: number; height: number };
  data: ExternalData;
};

export type FontRequest = {
  family: string;
  weight: number;
  codepoints: number[];
};

export class SlateReader {
  #memory: WebAssembly.Memory;
  #offsets: Record<string, number>;
  #slatePtr: number;
  #slabPtr: number;
  #buffer: ArrayBuffer;
  #view: DataView;
  #dirtyLo = 0;

  constructor(memory: WebAssembly.Memory, offsets: Record<string, number>, slatePtr: number, slabPtr: number) {
    this.#memory = memory;
    this.#offsets = offsets;
    this.#slatePtr = slatePtr;
    this.#slabPtr = slabPtr;
    this.#buffer = memory.buffer;
    this.#view = new DataView(memory.buffer);
  }

  refresh(slatePtr: number, slabPtr: number): void {
    this.#slatePtr = slatePtr;
    this.#slabPtr = slabPtr;
    if (this.#buffer !== this.#memory.buffer) {
      this.#buffer = this.#memory.buffer;
      this.#view = new DataView(this.#buffer);
    }
    this.#dirtyLo = this.#view.getUint32(this.#slatePtr + this.#offsets.dirty, true);
  }

  isDirty(bit: number): boolean {
    return (this.#dirtyLo & (1 << bit)) !== 0;
  }

  #u32(field: string): number {
    return this.#view.getUint32(this.#slatePtr + this.#offsets[field], true);
  }

  get pagesCount(): number {
    return this.#u32('pages_count');
  }

  readFontRequests(): FontRequest[] {
    const count = this.#u32('font_requests_count');
    const offset = this.#u32('font_requests_offset');
    return readFontRequests(this.#view, this.#slabPtr + offset, count);
  }

  readExternalElements(): ExternalElement[] {
    const count = this.#u32('external_elements_count');
    const offset = this.#u32('external_elements_offset');
    return readExternalElements(this.#view, this.#slabPtr + offset, count);
  }
}

// --- Internal parsing ---

function align4(n: number): number {
  return (n + 3) & ~3;
}

function readStr(view: DataView, offset: number): { value: string; end: number } {
  const byteLen = view.getUint32(offset, true);
  const bytes = new Uint8Array(view.buffer, offset + 4, byteLen);
  const value = new TextDecoder().decode(bytes);
  return { value, end: offset + 4 + align4(byteLen) };
}

function readFontRequests(view: DataView, offset: number, count: number): FontRequest[] {
  const requests: FontRequest[] = [];
  let pos = offset;
  for (let i = 0; i < count; i++) {
    const { value: family, end: afterFamily } = readStr(view, pos);
    pos = afterFamily;

    const weight = view.getUint32(pos, true);
    pos += 4;

    const cpCount = view.getUint32(pos, true);
    pos += 4;

    const codepoints: number[] = [];
    for (let j = 0; j < cpCount; j++) {
      codepoints.push(view.getUint32(pos + j * 4, true));
    }
    pos += cpCount * 4;

    requests.push({ family, weight, codepoints });
  }
  return requests;
}

const DATA_TAG_IMAGE = 0;
const DATA_TAG_FILE = 1;
const DATA_TAG_EMBED = 2;
const DATA_TAG_ARCHIVED = 3;

function readExternalElements(view: DataView, offset: number, count: number): ExternalElement[] {
  const elements: ExternalElement[] = [];
  let pos = offset;

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

    let data: ExternalData;
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

    elements.push({ pageIdx, nodeId, bounds, data });
  }

  return elements;
}
