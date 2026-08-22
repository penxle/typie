import assert from 'node:assert/strict';
import test from 'node:test';
import { routeSessionFrame } from './prism-session-stream.ts';
import type { Context, StreamFrame } from '@typie/prism';

const defaultContext = (): Context => ({ agent: { id: 'a', name: 'x' }, run: 2 });

const ev = (kind: string, data: Record<string, unknown>, context: Context | null = defaultContext()): StreamFrame => ({
  type: 'event',
  event: { seq: 1, kind, occurredAt: 1, context, data },
});

test('routeSessionFrame: workflow invocation·run 종결·워크플로 종결을 분류한다', () => {
  assert.deepEqual(
    routeSessionFrame(
      ev('invocation.started', { ordinal: 0, target: { kind: 'workflow', id: 'workflow_1', app: 'feedback', name: 'high', ref: 'r1' } }),
    ),
    { kind: 'workflow-started', workflowId: 'workflow_1', app: 'feedback', name: 'high', ref: 'r1', startedAt: 1 },
  );
  assert.equal(routeSessionFrame(ev('invocation.started', { ordinal: 0, target: { kind: 'agent', id: 'agent_1' } })), null);
  assert.deepEqual(routeSessionFrame(ev('run.completed', {})), { kind: 'run-terminal', runSeq: 2 });
  assert.deepEqual(routeSessionFrame(ev('run.canceled', {})), { kind: 'run-terminal', runSeq: 2 });
  assert.deepEqual(routeSessionFrame(ev('workflow.failed', { reason: 'x' })), { kind: 'workflow-terminal' });
  assert.equal(routeSessionFrame(ev('turn.completed', {})), null);
  assert.equal(routeSessionFrame({ type: 'heartbeat' }), null);
});

test('routeSessionFrame: assistant.titled는 title 문자열일 때만 titled 경로로 분류된다', () => {
  assert.deepEqual(routeSessionFrame(ev('assistant.titled', { title: '바다를 향한 걸음' })), { kind: 'titled', title: '바다를 향한 걸음' });
  assert.equal(routeSessionFrame(ev('assistant.titled', { title: 3 })), null);
  assert.equal(routeSessionFrame(ev('assistant.titled', {})), null);
});

test('routeSessionFrame: invocation.retried는 재attach 대상이 아닌 별도 경로로 분류된다', () => {
  assert.deepEqual(routeSessionFrame(ev('invocation.retried', { target: { kind: 'workflow', id: 'workflow_1' } })), {
    kind: 'invocation-retried',
  });
  assert.deepEqual(routeSessionFrame(ev('invocation.retried', {})), { kind: 'invocation-retried' });
});

test('routeSessionFrame: run 종결이라도 run 좌표가 없으면 부수효과를 만들지 않는다', () => {
  assert.deepEqual(routeSessionFrame(ev('run.failed', {})), { kind: 'run-terminal', runSeq: 2 });
  assert.equal(routeSessionFrame(ev('run.completed', {}, null)), null);
  assert.equal(routeSessionFrame(ev('run.completed', {}, { agent: { id: 'a', name: 'x' } })), null);
});

test('routeSessionFrame: 워크플로 target에 id가 없으면 attach하지 않고, 종결 kind는 전부 분류된다', () => {
  assert.equal(routeSessionFrame(ev('invocation.started', { target: { kind: 'workflow' } })), null);
  assert.equal(routeSessionFrame(ev('invocation.started', {})), null);
  // 링크 행을 이벤트만으로 만들므로 app·name이 없으면 만들 수 없다 — 되묻지 않고 넘긴다
  assert.equal(routeSessionFrame(ev('invocation.started', { target: { kind: 'workflow', id: 'workflow_1', name: 'high' } })), null);
  assert.equal(routeSessionFrame(ev('invocation.started', { target: { kind: 'workflow', id: 'workflow_1', app: 'feedback' } })), null);
  // ref는 없을 수 있다(ref 없이 구동된 워크플로) — null로 실린다
  assert.deepEqual(
    routeSessionFrame(ev('invocation.started', { target: { kind: 'workflow', id: 'workflow_1', app: 'feedback', name: 'high' } })),
    { kind: 'workflow-started', workflowId: 'workflow_1', app: 'feedback', name: 'high', ref: null, startedAt: 1 },
  );
  assert.deepEqual(routeSessionFrame(ev('workflow.completed', {})), { kind: 'workflow-terminal' });
  assert.deepEqual(routeSessionFrame(ev('workflow.canceled', {})), { kind: 'workflow-terminal' });
  assert.equal(routeSessionFrame({ type: 'sync', seq: 3 }), null);
  assert.equal(
    routeSessionFrame({
      type: 'delta',
      delta: { context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 }, channel: 'text', offset: 0, data: 'x' },
    }),
    null,
  );
});
