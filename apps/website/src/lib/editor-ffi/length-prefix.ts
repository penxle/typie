export function encodeLengthPrefixedBlobs(blobs: Uint8Array[]): Uint8Array {
  let totalSize = 4;
  for (const blob of blobs) {
    totalSize += 4 + blob.byteLength;
  }
  const buf = new Uint8Array(totalSize);
  const view = new DataView(buf.buffer);
  view.setUint32(0, blobs.length, true);
  let offset = 4;
  for (const blob of blobs) {
    view.setUint32(offset, blob.byteLength, true);
    offset += 4;
    buf.set(blob, offset);
    offset += blob.byteLength;
  }
  return buf;
}
