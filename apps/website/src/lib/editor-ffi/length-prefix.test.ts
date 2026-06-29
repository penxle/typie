import { describe, expect, it } from 'vitest';
import { encodeLengthPrefixedBlobs } from './length-prefix';

function decodeLengthPrefixedBlobs(data: Uint8Array): Uint8Array[] {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const count = view.getUint32(0, true);
  const result: Uint8Array[] = [];
  let offset = 4;
  for (let i = 0; i < count; i++) {
    const len = view.getUint32(offset, true);
    offset += 4;
    result.push(data.slice(offset, offset + len));
    offset += len;
  }
  return result;
}

describe('encodeLengthPrefixedBlobs', () => {
  it('empty list produces four-byte zero count', () => {
    expect(encodeLengthPrefixedBlobs([])).toEqual(new Uint8Array([0, 0, 0, 0]));
  });

  it('one blob round-trips', () => {
    const blob = new Uint8Array([1, 2, 3]);
    const decoded = decodeLengthPrefixedBlobs(encodeLengthPrefixedBlobs([blob]));
    expect(decoded).toHaveLength(1);
    expect(decoded[0]).toEqual(blob);
  });

  it('two blobs round-trip', () => {
    const a = new Uint8Array([10, 20]);
    const b = new Uint8Array([30, 40, 50, 60]);
    const decoded = decodeLengthPrefixedBlobs(encodeLengthPrefixedBlobs([a, b]));
    expect(decoded).toHaveLength(2);
    expect(decoded[0]).toEqual(a);
    expect(decoded[1]).toEqual(b);
  });
});
