import assert from 'node:assert/strict';
import test from 'node:test';
import { pushCopy, pushKey, subjectTitle } from './prism-core.ts';

test('subjectTitle: 낫표로 감싸고, 없으면 새 대화', () => {
  assert.equal(subjectTitle('원고'), '「원고」');
  assert.equal(subjectTitle(''), '「새 대화」');
  assert.equal(subjectTitle(null), '「새 대화」');
});

test('pushKey: ask는 toolCallId 네임스페이스', () => {
  assert.equal(pushKey.ask('call_1'), 'prism:push:ask:call_1');
});

test('pushCopy: ask-user는 질문 문안', () => {
  const copy = pushCopy('ask-user', { questions: [{ question: '어떤 방향으로 갈까요?' }] }, '초고 검토');
  assert.match(copy.title, /질문/);
  assert.equal(copy.body, '어떤 방향으로 갈까요?');
});

test('pushCopy: ask-user 질문이 여럿이면 외 N개', () => {
  const copy = pushCopy('ask-user', { questions: [{ question: '첫째' }, { question: '둘째' }] }, null);
  assert.equal(copy.body, '첫째 외 1개');
});

test('pushCopy: ask-user 질문 목록이 비면 본문이 비지 않는다', () => {
  assert.equal(pushCopy('ask-user', { questions: [] }, '초고 검토').body, '열어서 확인해 주세요.');
  assert.equal(pushCopy('ask-user', { questions: [{ question: '' }] }, '초고 검토').body, '열어서 확인해 주세요.');
});

test('pushCopy: ask-user 데이터가 깨져도 본문이 비지 않는다', () => {
  assert.equal(pushCopy('ask-user', null, '초고 검토').body, '열어서 확인해 주세요.');
  assert.equal(pushCopy('ask-user', { questions: '아님' }, '초고 검토').body, '열어서 확인해 주세요.');
  assert.equal(pushCopy('ask-user', { questions: [{ hint: '질문 없음' }] }, '초고 검토').body, '열어서 확인해 주세요.');
});

test('pushCopy: confirm-review는 리뷰 시작 확인 문안', () => {
  const copy = pushCopy('confirm-review', {}, '초고 검토');
  assert.match(copy.title, /리뷰/);
  assert.ok(copy.body.length > 0);
});

test('pushCopy: 미등재 도구도 문안이 나온다', () => {
  const copy = pushCopy('some-new-tool', {}, '초고 검토');
  assert.ok(copy.title.length > 0);
  assert.ok(copy.body.length > 0);
});

test('pushCopy: 제목이 없으면 새 대화로 표기', () => {
  const copy = pushCopy('confirm-review', {}, null);
  assert.match(copy.title, /새 대화/);
});

test('pushCopy: 파괴적 도구도 폴백 문안을 받는다', () => {
  assert.deepEqual(pushCopy('delete-entities', {}, '초고 검토'), pushCopy('some-new-tool', {}, '초고 검토'));
});
