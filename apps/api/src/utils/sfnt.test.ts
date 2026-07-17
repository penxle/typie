import assert from 'node:assert/strict';
import test from 'node:test';
import { sfntDirectoryLength, sfntHasTable } from './sfnt.ts';

const buildDirectory = (tags: string[]): Uint8Array => {
  const out = new Uint8Array(12 + 16 * tags.length);
  out[4] = tags.length >> 8;
  out[5] = tags.length & 0xff;
  for (const [i, tag] of tags.entries()) {
    for (let j = 0; j < 4; j++) {
      out[12 + 16 * i + j] = tag.codePointAt(j) ?? 0;
    }
  }
  return out;
};

test('sfntHasTable finds the COLR tag', () => {
  assert.equal(sfntHasTable(buildDirectory(['cmap', 'COLR', 'glyf']), 'COLR'), true);
});

test('sfntHasTable returns false when COLR is absent', () => {
  assert.equal(sfntHasTable(buildDirectory(['cmap', 'glyf', 'loca']), 'COLR'), false);
});

test('sfntHasTable returns null when the directory is truncated', () => {
  const full = buildDirectory(Array.from({ length: 70 }, (_, i) => (i === 65 ? 'COLR' : 'cmap')));
  const truncated = full.slice(0, 1024);
  assert.equal(sfntHasTable(truncated, 'COLR'), null);
  assert.equal(sfntDirectoryLength(truncated), 12 + 16 * 70);
  assert.equal(sfntHasTable(full, 'COLR'), true);
});

test('sfntHasTable returns null for buffers under 12 bytes', () => {
  assert.equal(sfntHasTable(new Uint8Array(4), 'COLR'), null);
});
