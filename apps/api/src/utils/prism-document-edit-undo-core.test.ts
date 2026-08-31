import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changedAfter, headsEqual, undoTarget } from './prism-document-edit-undo-core.ts';

test('headsEqual: 같은 바이트열은 같다', () => {
  assert.equal(headsEqual(new Uint8Array([1, 2, 3]), new Uint8Array([1, 2, 3])), true);
  assert.equal(headsEqual(new Uint8Array(), new Uint8Array()), true);
});

test('headsEqual: 길이나 내용이 다르면 다르다', () => {
  assert.equal(headsEqual(new Uint8Array([1, 2]), new Uint8Array([1, 2, 3])), false);
  assert.equal(headsEqual(new Uint8Array([1, 2, 3]), new Uint8Array([1, 2, 4])), false);
});

test('headsEqual: null은 무엇과도 같지 않다', () => {
  assert.equal(headsEqual(null, new Uint8Array([1])), false);
  assert.equal(headsEqual(new Uint8Array([1]), null), false);
  assert.equal(headsEqual(null, null), false);
});

test('changedAfter: live가 checkpoint와 같을 때만 변경 없음', () => {
  assert.equal(changedAfter(new Uint8Array([9, 9]), new Uint8Array([9, 9])), false);
  assert.equal(changedAfter(new Uint8Array([9, 8]), new Uint8Array([9, 9])), true);
});

test('changedAfter: 캐시 미스는 보수적으로 변경 있음', () => {
  assert.equal(changedAfter(null, new Uint8Array([9, 9])), true);
});

test('undoTarget: 되돌리지 않은 편집은 저장 직전으로, 되돌린 편집은 저장 직후로', () => {
  const before = new Uint8Array([1]);
  const after = new Uint8Array([2]);
  assert.deepEqual(undoTarget({ beforeHeads: before, afterHeads: after, undone: false }), before);
  assert.deepEqual(undoTarget({ beforeHeads: before, afterHeads: after, undone: true }), after);
});
