import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveCalloutFillToHex, resolveColorToHex } from './theme.ts';

test('공통 색상 키가 # 없는 hex로 해석된다', () => {
  assert.match(resolveColorToHex('text.black') ?? '', /^[0-9a-fA-F]{6}$/);
  assert.match(resolveColorToHex('text.white') ?? '', /^[0-9a-fA-F]{6}$/);
});

test('없는 키는 undefined', () => {
  assert.equal(resolveColorToHex('text.does-not-exist'), undefined);
});

test('v2 전용 키도 해석된다', () => {
  assert.match(resolveColorToHex('ui.search-match') ?? '', /^[0-9a-fA-F]{6}$/);
});

test('콜아웃 채움은 콜아웃 색을 종이 위에 알파 8/255로 합성한 값이다', () => {
  assert.equal(resolveCalloutFillToHex('info'), 'f9fbff');
  assert.equal(resolveCalloutFillToHex('danger'), 'fef9f9');
});

test('없는 콜아웃 variant는 undefined', () => {
  assert.equal(resolveCalloutFillToHex('unknown'), undefined);
});
