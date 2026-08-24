import assert from 'node:assert/strict';
import test from 'node:test';
import { TOOL_META } from '@typie/prism';
import { shouldPushAsk } from './prism-push-gate.ts';
import type { ToolPolicy } from '@typie/prism';

const POLICIES: ToolPolicy[] = ['READ_ONLY', 'STANDARD', 'FULL'];

const pushing = (policy: ToolPolicy) =>
  Object.keys(TOOL_META)
    .filter((tool) => shouldPushAsk(tool, policy))
    .toSorted((a, b) => a.localeCompare(b));

test('shouldPushAsk: 서버가 해소하는 도구는 어떤 정책에서도 푸시하지 않는다', () => {
  for (const policy of POLICIES) {
    assert.equal(shouldPushAsk('search-entities', policy), false, `search-entities @ ${policy}`);
    assert.equal(shouldPushAsk('read-document', policy), false, `read-document @ ${policy}`);
    assert.equal(shouldPushAsk('create-document', policy), false, `create-document @ ${policy}`);
    assert.equal(shouldPushAsk('list-open-documents', policy), false, `list-open-documents @ ${policy}`);
  }
});

test('shouldPushAsk: 정책별로 푸시하는 도구 전수', () => {
  assert.deepEqual(pushing('READ_ONLY'), ['ask-user', 'confirm-review']);
  assert.deepEqual(pushing('STANDARD'), ['ask-user', 'confirm-review', 'delete-entities', 'delete-goal', 'delete-note', 'update-sharing']);
  assert.deepEqual(pushing('FULL'), ['ask-user', 'confirm-review']);
});

test('shouldPushAsk: 파괴적 도구는 승인이 필요한 STANDARD에서만 푸시한다', () => {
  assert.equal(shouldPushAsk('delete-note', 'READ_ONLY'), false);
  assert.equal(shouldPushAsk('delete-note', 'STANDARD'), true);
  assert.equal(shouldPushAsk('delete-note', 'FULL'), false);
});

test('shouldPushAsk: 알 수 없는 도구는 사용자 해소로 보고 푸시한다', () => {
  for (const policy of POLICIES) {
    assert.equal(shouldPushAsk('unknown-tool', policy), true, `unknown-tool @ ${policy}`);
  }
});
