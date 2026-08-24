export const SPINNER_HDR_ASSET_MIN_HEADROOM = 1;
export const SPINNER_HDR_ASSET_MAX_HEADROOM = 2.5;
export const SPINNER_HDR_ASSET_VERTEX_STRIDE = 10;

const MAGIC = [0x50, 0x53, 0x48, 0x31] as const;
const VERSION = 1;
const HEADER_BYTES = 16;

export type SpinnerHdrAsset = {
  frames: readonly Float32Array[];
  totalVertexCount: number;
};

function assertFrame(frame: Float32Array): void {
  if (frame.length % SPINNER_HDR_ASSET_VERTEX_STRIDE !== 0) {
    throw new RangeError(`HDR frame data must use a ${SPINNER_HDR_ASSET_VERTEX_STRIDE}-float vertex stride.`);
  }
}

export function decodeSpinnerHdrAsset(buffer: ArrayBuffer): SpinnerHdrAsset {
  if (buffer.byteLength < HEADER_BYTES + 2 * Uint32Array.BYTES_PER_ELEMENT) {
    throw new RangeError('HDR asset is truncated.');
  }
  const view = new DataView(buffer);
  if (MAGIC.some((byte, index) => view.getUint8(index) !== byte)) throw new RangeError('HDR asset magic is invalid.');
  if (view.getUint16(4, true) !== VERSION) throw new RangeError('HDR asset version is unsupported.');
  if (view.getUint16(6, true) !== SPINNER_HDR_ASSET_VERTEX_STRIDE) throw new RangeError('HDR asset vertex stride is invalid.');

  const frameCount = view.getUint32(8, true);
  const totalVertexCount = view.getUint32(12, true);
  if (frameCount === 0) throw new RangeError('HDR asset has no frames.');
  const dataOffset = HEADER_BYTES + (frameCount + 1) * Uint32Array.BYTES_PER_ELEMENT;
  const expectedBytes = dataOffset + totalVertexCount * SPINNER_HDR_ASSET_VERTEX_STRIDE * Float32Array.BYTES_PER_ELEMENT;
  if (expectedBytes !== buffer.byteLength) throw new RangeError('HDR asset byte length is invalid.');

  const frames: Float32Array[] = [];
  for (let index = 0; index < frameCount; index += 1) {
    const start = view.getUint32(HEADER_BYTES + index * 4, true);
    const end = view.getUint32(HEADER_BYTES + (index + 1) * 4, true);
    if (start > end || end > totalVertexCount) throw new RangeError('HDR asset frame offsets are invalid.');
    frames.push(
      new Float32Array(
        buffer,
        dataOffset + start * SPINNER_HDR_ASSET_VERTEX_STRIDE * Float32Array.BYTES_PER_ELEMENT,
        (end - start) * SPINNER_HDR_ASSET_VERTEX_STRIDE,
      ),
    );
  }
  return { frames, totalVertexCount };
}

export function interpolateSpinnerHdrFrame(frame: Float32Array, headroom: number, target?: Float32Array): Float32Array {
  assertFrame(frame);
  const vertexCount = frame.length / SPINNER_HDR_ASSET_VERTEX_STRIDE;
  const outputLength = vertexCount * 6;
  const output = target?.length === outputLength ? target : new Float32Array(outputLength);
  const normalizedHeadroom = Math.max(
    0,
    Math.min(1, (headroom - SPINNER_HDR_ASSET_MIN_HEADROOM) / (SPINNER_HDR_ASSET_MAX_HEADROOM - SPINNER_HDR_ASSET_MIN_HEADROOM)),
  );

  for (let vertex = 0; vertex < vertexCount; vertex += 1) {
    const source = vertex * SPINNER_HDR_ASSET_VERTEX_STRIDE;
    const destination = vertex * 6;
    output[destination] = frame[source] ?? 0;
    output[destination + 1] = frame[source + 1] ?? 0;
    for (let channel = 0; channel < 4; channel += 1) {
      const low = frame[source + 2 + channel] ?? 0;
      const high = frame[source + 6 + channel] ?? 0;
      output[destination + 2 + channel] = low + (high - low) * normalizedHeadroom;
    }
  }
  return output;
}
