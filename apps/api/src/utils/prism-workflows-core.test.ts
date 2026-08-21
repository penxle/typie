import assert from 'node:assert/strict';
import test from 'node:test';
import dayjs from 'dayjs';
import { isRunningChildAgent, isRunRunning, settleUpdate, workflowTargets } from './prism-workflows-core.ts';
import type { InvocationSummary } from '@typie/prism';

const invocation = (over: Partial<InvocationSummary>): InvocationSummary => ({
  invocationId: 'i1',
  targetKind: 'workflow',
  targetId: 'workflow_1',
  originRunSeq: 1,
  status: 'running',
  ...over,
});

const workflow = (over: Partial<Parameters<typeof settleUpdate>[0]>): Parameters<typeof settleUpdate>[0] => ({
  id: 'workflow_1',
  app: 'app_1',
  workflow: 'high',
  ref: 'PRRR1',
  status: 'running',
  result: null,
  error: null,
  usage: null,
  startedAt: 1000,
  finishedAt: null,
  ...over,
});

test('settleUpdate: 아직 running이면 갱신하지 않는다', () => {
  assert.equal(settleUpdate(workflow({ status: 'running' })), null);
  assert.equal(settleUpdate(workflow({ status: 'pending' })), null);
});

test('settleUpdate: 터미널 3종은 각각의 state로, error·finishedAt을 그대로 옮긴다', () => {
  const done = settleUpdate(workflow({ status: 'completed', finishedAt: 2000 }));
  assert.equal(done?.state, 'COMPLETED');
  assert.equal(done?.error, null);
  assert.equal(done?.finishedAt.valueOf(), 2000);

  const failed = settleUpdate(workflow({ status: 'failed', error: 'boom', finishedAt: 3000 }));
  assert.equal(failed?.state, 'FAILED');
  assert.equal(failed?.error, 'boom');

  assert.equal(settleUpdate(workflow({ status: 'canceled', finishedAt: 4000 }))?.state, 'CANCELED');
});

const fold = {
  provider: 'anthropic',
  agent: 'chat',
  model: 'claude-opus-5',
  effort: null,
  turns: 3,
  inputTokens: 100,
  outputTokens: 20,
  cacheReadTokens: 4000,
  cacheWriteTokens: 0,
  thinkingTokens: 5,
};

test('settleUpdate: usage는 판별자를 벗기고 live 회계는 complete:false로 굳는다', () => {
  assert.equal(settleUpdate(workflow({ status: 'completed', usage: null }))?.usage, null);

  const settled = settleUpdate(
    workflow({ status: 'completed', usage: { settled: true, complete: true, folds: [fold] }, finishedAt: 2000 }),
  );
  assert.deepEqual(settled?.usage, { complete: true, folds: [fold] });

  const incomplete = settleUpdate(workflow({ status: 'completed', usage: { settled: true, complete: false, folds: [] } }));
  assert.deepEqual(incomplete?.usage, { complete: false, folds: [] });

  const live = settleUpdate(workflow({ status: 'failed', usage: { settled: false, folds: [] } }));
  assert.deepEqual(live?.usage, { complete: false, folds: [] });
});

test('settleUpdate: finishedAt이 없는 터미널은 관측 시각으로 채운다', () => {
  const before = dayjs();
  const update = settleUpdate(workflow({ status: 'canceled', finishedAt: null }));
  assert.ok(update !== null);
  assert.ok(!update.finishedAt.isBefore(before));
  assert.ok(!update.finishedAt.isAfter(dayjs()));
});

test('workflowTargets: workflow 타깃 id만 순서대로, run·상태와 무관하게 전부', () => {
  const invocations = [
    invocation({ targetId: 'workflow_1', originRunSeq: 3 }),
    invocation({ targetKind: 'agent', targetId: 'agent_1' }),
    invocation({ targetId: 'workflow_2', originRunSeq: null, status: 'completed' }),
  ];
  assert.deepEqual(workflowTargets(invocations), ['workflow_1', 'workflow_2']);
  assert.deepEqual(workflowTargets([]), []);
});

test('isRunningChildAgent: running인 agent 타깃만 허용한다', () => {
  const invocations = [
    invocation({ targetKind: 'agent', targetId: 'agent_1', status: 'running' }),
    invocation({ targetKind: 'agent', targetId: 'agent_2', status: 'completed' }),
    invocation({ targetKind: 'workflow', targetId: 'agent_3', status: 'running' }),
  ];
  assert.equal(isRunningChildAgent(invocations, 'agent_1'), true);
  assert.equal(isRunningChildAgent(invocations, 'agent_2'), false);
  assert.equal(isRunningChildAgent(invocations, 'agent_3'), false);
  assert.equal(isRunningChildAgent([], 'agent_1'), false);
});

test('isRunRunning: 그 runSeq의 run이 running일 때만 참', () => {
  const runs = [
    { runSeq: 1, status: 'completed' },
    { runSeq: 2, status: 'running' },
  ];
  assert.equal(isRunRunning(runs, 2), true);
  assert.equal(isRunRunning(runs, 1), false);
  assert.equal(isRunRunning(runs, 3), false);
});
