import assert from 'node:assert/strict';
import test from 'node:test';
import { TOOL_META } from '@typie/prism';
import { pushCopy, pushKey, shouldPushAsk, subjectTitle } from './prism-push-core.ts';
import type { ToolPolicy } from '@typie/prism';

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

const POLICIES: ToolPolicy[] = ['READ_ONLY', 'STANDARD', 'FULL'];

const pushing = (policy: ToolPolicy) =>
  Object.keys(TOOL_META)
    .filter((tool) => shouldPushAsk(tool, policy))
    .toSorted((a, b) => a.localeCompare(b));

test('shouldPushAsk: 서버가 해소하는 도구는 어떤 정책에서도 푸시하지 않는다', () => {
  for (const policy of POLICIES) {
    assert.equal(shouldPushAsk('search-entities', policy), false, `search-entities @ ${policy}`);
    assert.equal(shouldPushAsk('read-document', policy), false, `read-document @ ${policy}`);
    assert.equal(shouldPushAsk('create-documents', policy), false, `create-documents @ ${policy}`);
    assert.equal(shouldPushAsk('list-open-documents', policy), false, `list-open-documents @ ${policy}`);
  }
});

test('shouldPushAsk: 정책별로 푸시하는 도구 전수', () => {
  assert.deepEqual(pushing('READ_ONLY'), ['ask-user', 'confirm-review']);
  assert.deepEqual(pushing('STANDARD'), [
    'ask-user',
    'confirm-review',
    'delete-entities',
    'delete-goals',
    'delete-notes',
    'save-document',
    'update-sharing',
  ]);
  assert.deepEqual(pushing('FULL'), ['ask-user', 'confirm-review']);
});

test('shouldPushAsk: 파괴적 도구는 승인이 필요한 STANDARD에서만 푸시한다', () => {
  assert.equal(shouldPushAsk('delete-notes', 'READ_ONLY'), false);
  assert.equal(shouldPushAsk('delete-notes', 'STANDARD'), true);
  assert.equal(shouldPushAsk('delete-notes', 'FULL'), false);
});

test('shouldPushAsk: 알 수 없는 도구는 사용자 해소로 보고 푸시한다', () => {
  for (const policy of POLICIES) {
    assert.equal(shouldPushAsk('unknown-tool', policy), true, `unknown-tool @ ${policy}`);
  }
});
