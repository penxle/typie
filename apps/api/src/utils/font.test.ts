import assert from 'node:assert/strict';
import test from 'node:test';
import { chunkCodepoints, isNonEmptyHead } from './font.ts';

test('chunkCodepoints caps chunk count at 255 without a strategy', () => {
  const codepoints = Array.from({ length: 60_000 }, (_, i) => i + 1);
  const { chunks } = chunkCodepoints('NoStrategy', codepoints, {});
  assert.ok(chunks.length <= 255);
  assert.equal(chunks.flat().length, codepoints.length);
});

test('chunkCodepoints falls back to sequential slicing when strategy chunks exceed the budget', () => {
  const groups = Array.from({ length: 300 }, (_, g) => Array.from({ length: 10 }, (_, i) => g * 10 + i + 1));
  const codepoints = groups.flat();
  const { chunks, strategy } = chunkCodepoints('FooKR', codepoints, { korean: groups });
  assert.ok(chunks.length <= 255);
  assert.equal(strategy, null);
  assert.equal(chunks.flat().length, codepoints.length);
});

test('chunkCodepoints uses the matched strategy chunks when they stay within budget', () => {
  const groups = Array.from({ length: 10 }, (_, g) => Array.from({ length: 300 }, (_, i) => g * 300 + i + 1));
  const codepoints = groups.flat();
  const { chunks, strategy } = chunkCodepoints('FooKR', codepoints, { korean: groups });
  assert.equal(strategy, 'korean');
  assert.deepEqual(chunks, groups);
  assert.equal(chunks.flat().length, codepoints.length);
});

test('isNonEmptyHead returns false for a missing object', () => {
  assert.equal(isNonEmptyHead(null), false);
});

test('isNonEmptyHead returns false for a zero-length object', () => {
  assert.equal(isNonEmptyHead({ ContentLength: 0 }), false);
});

test('isNonEmptyHead returns false when ContentLength is missing', () => {
  assert.equal(isNonEmptyHead({}), false);
});

test('isNonEmptyHead returns true for a non-empty object', () => {
  assert.equal(isNonEmptyHead({ ContentLength: 10 }), true);
});
