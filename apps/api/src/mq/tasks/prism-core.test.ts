import assert from 'node:assert/strict';
import test from 'node:test';
import { askBody, pushKey, subjectTitle } from './prism-core.ts';

test('subjectTitle: 낫표로 감싸고, 없으면 새 대화', () => {
  assert.equal(subjectTitle('원고'), '「원고」');
  assert.equal(subjectTitle(''), '「새 대화」');
  assert.equal(subjectTitle(null), '「새 대화」');
});

test('askBody: 첫 질문 + 외 N개', () => {
  const q = (question: string) => ({ question, hint: '', multi: false, options: [] });
  assert.equal(askBody([q('A?')]), 'A?');
  assert.equal(askBody([q('A?'), q('B?'), q('C?')]), 'A? 외 2개');
});

test('askBody: 질문이 없으면 빈 문자열', () => {
  assert.equal(askBody([]), '');
});

test('pushKey: ask는 toolCallId 네임스페이스', () => {
  assert.equal(pushKey.ask('call_1'), 'prism:push:ask:call_1');
});
