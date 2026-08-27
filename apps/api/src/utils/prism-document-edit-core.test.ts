import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  changedOf,
  documentIdOf,
  documentPath,
  messageOf,
  SAVE_TOO_LARGE_MESSAGE,
  SaveDocumentInput,
  TOO_LARGE_MESSAGE,
  XML_DETAIL_TYPES,
  XML_MESSAGES,
} from './prism-document-edit-core.ts';

test('경로는 documentId와 왕복한다', () => {
  assert.equal(documentPath('D0ABC'), 'documents/D0ABC.xml');
  assert.equal(documentIdOf('documents/D0ABC.xml'), 'D0ABC');
  assert.equal(documentIdOf('manuscript/v1.txt'), null);
  assert.equal(documentIdOf('documents/a/b.xml'), null);
});

test('저장 입력은 경로 규약과 빈 summary를 거른다', () => {
  assert.ok(SaveDocumentInput.safeParse({ path: 'documents/D0ABC.xml', summary: '오탈자 셋을 고쳤어요' }).success);
  assert.ok(!SaveDocumentInput.safeParse({ path: 'documents/D0ABC.txt', summary: 'x' }).success);
  assert.ok(!SaveDocumentInput.safeParse({ path: 'documents/D0ABC.xml', summary: ' '.repeat(3) }).success);
});

test('문면 표는 detail.type 62종을 빠짐없이 덮는다', () => {
  assert.equal(XML_DETAIL_TYPES.length, 62);
  const byName = (a: string, b: string) => a.localeCompare(b);
  assert.deepEqual(Object.keys(XML_MESSAGES).toSorted(byName), XML_DETAIL_TYPES.toSorted(byName));
});

test('상계 문면은 열기와 저장을 나눈다', () => {
  assert.notEqual(SAVE_TOO_LARGE_MESSAGE, TOO_LARGE_MESSAGE);
});

test('messageOf는 좌표와 dot을 접두·병기한다', () => {
  const info = {
    line: 12,
    column: 8,
    dot: undefined,
    detail: JSON.stringify({ type: 'close_without_open', name: 'bold', open: 'italic' }),
    message: 'x',
  };
  assert.equal(messageOf(info), '줄 12 열 8: `</bold>`가 닫을 요소가 없어요 — 지금 열린 요소는 <italic>이에요');

  const dotted = {
    line: 9,
    column: 1,
    dot: '1_5',
    detail: JSON.stringify({ type: 'opaque_id_changed', element: 'image', dot: '1_5' }),
    message: 'x',
  };
  assert.equal(messageOf(dotted), '줄 9 열 1: `<image>`의 `attr:id`는 바꿀 수 없어요 (dot 1_5)');

  const bare = {
    line: undefined,
    column: undefined,
    dot: undefined,
    detail: JSON.stringify({ type: 'base_not_in_history' }),
    message: 'x',
  };
  assert.equal(messageOf(bare), '이 파일은 지금 문서와 이어지지 않아요 — 문서를 다시 여세요');

  const filled = {
    line: 5,
    column: 5,
    dot: undefined,
    detail: JSON.stringify({ type: 'content_rule', parent: 'table_row', allowed: ['table_cell'], got: ['paragraph'], rule: 'TableCell+' }),
    message: 'x',
  };
  assert.equal(messageOf(filled), '줄 5 열 5: `<table_row>` 안에 올 수 있는 것은 <table_cell>이에요 — 지금은 <paragraph>이 있어요');

  const empty = {
    ...filled,
    detail: JSON.stringify({ type: 'content_rule', parent: 'table_row', allowed: ['table_cell'], got: [], rule: 'TableCell+' }),
  };
  assert.equal(messageOf(empty), '줄 5 열 5: `<table_row>` 안에 올 수 있는 것은 <table_cell>이에요 — 지금은 비어 있어요');

  const unknown = { line: undefined, column: undefined, dot: undefined, detail: '{"type":"???"}', message: 'fallback' };
  assert.equal(messageOf(unknown), '이 편집은 적용할 수 없어요 — 문서를 다시 열고 다시 시도하세요');

  const scalar = { line: undefined, column: undefined, dot: undefined, detail: 'null', message: 'fallback' };
  assert.equal(messageOf(scalar), '이 편집은 적용할 수 없어요 — 문서를 다시 열고 다시 시도하세요');
});

test('코드포인트는 생산자와 같은 네 자리 16진으로 적는다', () => {
  const control = {
    line: undefined,
    column: undefined,
    dot: undefined,
    detail: JSON.stringify({ type: 'forbidden_control_char', codepoint: 11 }),
    message: 'x',
  };
  assert.equal(messageOf(control), '쓸 수 없는 제어 문자(U+000B)가 있어요');

  const missing = { ...control, detail: JSON.stringify({ type: 'forbidden_control_char' }) };
  assert.equal(messageOf(missing), '쓸 수 없는 제어 문자(U+?)가 있어요');
});

test('changedOf는 집계 6필드를 두 묶음으로 사영한다', () => {
  assert.deepEqual(
    changedOf({
      error: undefined,
      bundle: new Uint8Array(),
      xml: '',
      blocks_inserted: 1,
      blocks_deleted: 2,
      blocks_moved: 3,
      blocks_updated: 4,
      chars_inserted: 5,
      chars_deleted: 6,
    }),
    { blocks: { inserted: 1, deleted: 2, moved: 3, updated: 4 }, chars: { inserted: 5, deleted: 6 } },
  );
});
