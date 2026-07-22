import assert from 'node:assert/strict';
import test from 'node:test';
import { hangulRatio, promptUpdateSchema, verifyInternalKey } from './internal.ts';

test('verifyInternalKey: 정확한 Bearer 키만 통과', () => {
  assert.equal(verifyInternalKey('Bearer secret-key', 'secret-key'), true);
  assert.equal(verifyInternalKey('Bearer wrong', 'secret-key'), false);
  assert.equal(verifyInternalKey(undefined, 'secret-key'), false);
  assert.equal(verifyInternalKey('secret-key', 'secret-key'), true);
});

test('verifyInternalKey: 길이가 달라도 예외 없이 false', () => {
  assert.equal(verifyInternalKey('Bearer s', 'secret-key'), false);
});

test('hangulRatio: 공백 제외 한글 비율', () => {
  assert.equal(hangulRatio('안녕하세요 반갑습니다'), 1);
  assert.equal(hangulRatio('한글ab'), 0.5);
  assert.equal(hangulRatio(' '.repeat(3)), 0);
});

test('promptUpdateSchema: 유효 페이로드 통과, effort null 허용', () => {
  const payload = { model: 'm', effort: null, systemPrompt: 's', toolDescriptions: { tool: 'x' } };
  assert.deepEqual(promptUpdateSchema.parse(payload), payload);
});

test('promptUpdateSchema: systemPrompt 누락 거부', () => {
  assert.throws(() => promptUpdateSchema.parse({ model: 'm', effort: null, toolDescriptions: {} }));
});
