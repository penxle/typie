import assert from 'node:assert/strict';
import test from 'node:test';
import { applyDelta } from '@typie/prism';
import {
  absentDelay,
  createFrameGate,
  liveFieldKey,
  liveSnapshotFrames,
  logKeyOf,
  parseLogKey,
  planEvent,
  shouldStop,
} from './prism-ingest-core.ts';
import type { EventFrame, ProjectedDeltaFrame, TurnLive } from '@typie/prism';
import type { DomainOp } from './prism-ingest-core.ts';

const agent = { id: 'chat-1', name: 'chat' };
const ev = (seq: number, kind: string, context: EventFrame['context'], data: Record<string, unknown> = {}): EventFrame => ({
  seq,
  kind,
  occurredAt: 1000 + seq,
  loggedAt: 1000 + seq,
  context,
  data,
});
const run = { agent, run: 1 };
const turn = { ...run, turn: 1, attempt: 1 };
const call = { ...turn, toolCallId: 'c1' };

const askOps = (ops: DomainOp[]) => ops.filter((op): op is Extract<DomainOp, { op: 'ask-push' }> => op.op === 'ask-push');

test('logKey는 왕복한다(BullMQ jobId 제약으로 구분자는 -)', () => {
  assert.equal(logKeyOf({ kind: 'agent', sessionId: 'PRSS1' }), 'agent-PRSS1');
  assert.deepEqual(parseLogKey('workflow-PRWF1'), { kind: 'workflow', workflowId: 'PRWF1' });
  assert.equal(parseLogKey('nope'), null);
  assert.equal(parseLogKey('agent-'), null);
});

test('advance는 seq가 커서를 넘을 때만', () => {
  assert.equal(planEvent('agent', ev(5, 'turn.started', turn), 4).advance, true);
  assert.equal(planEvent('agent', ev(5, 'turn.started', turn), 5).advance, false);
  assert.equal(planEvent('agent', ev(5, 'turn.started', turn), 9).advance, false);
});

test('agent 펌프의 도메인 op — run 시작·종결·링크·제목·질문 푸시', () => {
  assert.deepEqual(planEvent('agent', ev(1, 'run.started', run, { message: 'a' }), 0).ops, [{ op: 'run-started', runSeq: 1, at: 1001 }]);
  assert.deepEqual(planEvent('agent', ev(2, 'run.failed', run, { reason: 'x' }), 1).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'FAILED', at: 1002, charge: undefined },
  ]);
  assert.deepEqual(planEvent('agent', ev(3, 'run.canceled', run, { charge: null }), 2).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'CANCELED', at: 1003, charge: null },
  ]);
  assert.deepEqual(planEvent('agent', ev(10, 'run.completed', run, { result: 'r', charge: { milli: 1234 } }), 9).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'COMPLETED', at: 1010, charge: 1234 },
  ]);
  assert.deepEqual(planEvent('agent', ev(11, 'run.completed', run, { result: 'r', charge: { milli: '12' } }), 10).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'COMPLETED', at: 1011, charge: null },
  ]);
  assert.deepEqual(planEvent('agent', ev(12, 'run.completed', run, { result: 'r', charge: { milli: -1 } }), 11).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'COMPLETED', at: 1012, charge: null },
  ]);
  assert.deepEqual(planEvent('agent', ev(13, 'run.completed', run, { result: 'r', charge: 5 }), 12).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'COMPLETED', at: 1013, charge: null },
  ]);
  assert.deepEqual(planEvent('agent', ev(13, 'run.completed', run, { result: 'r', charge: { milli: 1.5 } }), 12).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'COMPLETED', at: 1013, charge: null },
  ]);
  assert.deepEqual(planEvent('agent', ev(14, 'run.completed', run, { result: 'r', charge: { milli: 0 } }), 13).ops, [
    { op: 'run-terminal', runSeq: 1, state: 'COMPLETED', at: 1014, charge: 0 },
  ]);
  assert.deepEqual(
    planEvent(
      'agent',
      ev(
        4,
        'invocation.started',
        { ...call, invocation: 'inv_1' },
        { target: { kind: 'workflow', id: 'wf_1', name: 'high', app: 'feedback', ref: 'PRRR1' } },
      ),
      3,
    ).ops,
    [{ op: 'workflow-link', descriptor: { prismWorkflowId: 'wf_1', app: 'feedback', name: 'high', ref: 'PRRR1', startedAt: 1004 } }],
  );
  assert.deepEqual(
    planEvent('agent', ev(5, 'invocation.started', { ...call, invocation: 'inv_2' }, { target: { kind: 'agent' } }), 4).ops,
    [],
  );
  assert.deepEqual(planEvent('agent', ev(6, 'assistant.titled', call, { title: '제목' }), 5).ops, [{ op: 'titled', title: '제목' }]);
  const payload = { questions: [{ question: 'q', hint: '', multi: false, options: [] }] };
  assert.deepEqual(planEvent('agent', ev(7, 'tool.requested', call, { tool: 'ask-user', data: payload }), 6).ops, [
    { op: 'ask-push', toolCallId: 'c1', tool: 'ask-user', data: payload, at: 1007 },
  ]);
  assert.deepEqual(planEvent('agent', ev(8, 'tool.requested', call, { tool: 'ask-user', data: { broken: true } }), 7).ops, [
    { op: 'ask-push', toolCallId: 'c1', tool: 'ask-user', data: { broken: true }, at: 1008 },
  ]);
  assert.deepEqual(planEvent('agent', ev(9, 'tool.requested', call, { tool: 'confirm-review', data: {} }), 8).ops, [
    { op: 'ask-push', toolCallId: 'c1', tool: 'confirm-review', data: {}, at: 1009 },
  ]);
});

test('askPush: ask-user 외의 도구도 op를 만든다', () => {
  const ops = planEvent('agent', ev(9, 'tool.requested', { ...call, toolCallId: 'c9' }, { tool: 'confirm-review', data: {} }), 0).ops;
  assert.ok(ops.some((op) => op.op === 'ask-push' && op.tool === 'confirm-review' && op.toolCallId === 'c9'));
});

test('askPush: toolCallId가 없으면 op가 없다', () => {
  const ops = planEvent('agent', ev(9, 'tool.requested', turn, { tool: 'ask-user', data: {} }), 0).ops;
  assert.ok(ops.every((op) => op.op !== 'ask-push'));
});

test('askPush: data는 원시 payload가 그대로 실린다', () => {
  const payload = { questions: [{ question: '무엇을 볼까요?' }] };
  const ops = planEvent('agent', ev(9, 'tool.requested', call, { tool: 'ask-user', data: payload }), 0).ops;
  assert.deepEqual(askOps(ops)[0]?.data, payload);
});

test('agent 펌프는 workflow.* 를 무시하고, workflow 펌프는 run.*·invocation.*·titled를 무시한다', () => {
  assert.deepEqual(planEvent('agent', ev(1, 'workflow.completed', {}, { result: null, usage: null }), 0).ops, []);
  assert.deepEqual(planEvent('workflow', ev(1, 'run.started', run), 0).ops, []);
  assert.deepEqual(planEvent('workflow', ev(2, 'assistant.titled', call, { title: 't' }), 1).ops, []);
});

test('workflow 펌프의 정산 op — usage 사영과 reason', () => {
  const usage = { settled: true, complete: true, folds: [] };
  assert.deepEqual(planEvent('workflow', ev(1, 'workflow.completed', {}, { result: { kind: 'feedback' }, usage }), 0).ops, [
    {
      op: 'workflow-settle',
      state: 'COMPLETED',
      result: { kind: 'feedback' },
      usage: { complete: true, folds: [] },
      error: null,
      at: 1001,
    },
  ]);
  assert.deepEqual(planEvent('workflow', ev(2, 'workflow.failed', {}, { reason: '터짐', usage: { settled: false, folds: [] } }), 1).ops, [
    { op: 'workflow-settle', state: 'FAILED', result: null, usage: { complete: false, folds: [] }, error: '터짐', at: 1002 },
  ]);
  assert.deepEqual(planEvent('workflow', ev(3, 'workflow.canceled', {}, { usage: null }), 2).ops, [
    { op: 'workflow-settle', state: 'CANCELED', result: null, usage: null, error: null, at: 1003 },
  ]);
});

test('workflow 펌프의 질문 푸시 op 도 at 을 싣는다', () => {
  const payload = { questions: [{ question: 'q', hint: '', multi: false, options: [] }] };
  assert.deepEqual(planEvent('workflow', ev(4, 'tool.requested', call, { tool: 'ask-user', data: payload }), 3).ops, [
    { op: 'ask-push', toolCallId: 'c1', tool: 'ask-user', data: payload, at: 1004 },
  ]);
});

test('tool.requested: tier 보유 도구는 tool-serve op를 낸다 (agent·workflow 공통)', () => {
  const event = ev(5, 'tool.requested', { ...call, toolCallId: 'tc-1' }, { tool: 'search-entities', data: { query: '해변' } });
  assert.deepEqual(
    planEvent('agent', event, 0).ops.filter((op) => op.op === 'tool-serve'),
    [{ op: 'tool-serve', toolCallId: 'tc-1', tool: 'search-entities', input: { query: '해변' }, agentId: 'chat-1', runSeq: 1 }],
  );
  assert.deepEqual(
    planEvent('workflow', event, 0).ops.filter((op) => op.op === 'tool-serve'),
    [{ op: 'tool-serve', toolCallId: 'tc-1', tool: 'search-entities', input: { query: '해변' }, agentId: 'chat-1', runSeq: null }],
  );
});

test('tool.requested: 인터랙티브·client·미등재 도구는 tool-serve를 내지 않는다', () => {
  for (const tool of ['ask-user', 'confirm-review', 'list-open-documents', 'unknown-tool']) {
    const event = ev(5, 'tool.requested', { ...call, toolCallId: 'tc-1' }, { tool, data: {} });
    assert.equal(
      planEvent('agent', event, 0).ops.some((op) => op.op === 'tool-serve'),
      false,
    );
  }
});

test('destructive 도구도 tool-serve op는 난다 (실행 여부는 서브 시점 판정)', () => {
  const event = ev(5, 'tool.requested', { ...call, toolCallId: 'tc-1' }, { tool: 'delete-entities', data: { ids: ['E1'] } });
  assert.equal(
    planEvent('agent', event, 0).ops.some((op) => op.op === 'tool-serve'),
    true,
  );
});

test('turn.completed는 라이브 봉인, 종결은 라이브 비움', () => {
  assert.deepEqual(planEvent('agent', ev(1, 'turn.completed', turn, { text: 't' }), 0), {
    advance: true,
    ops: [],
    sealTurn: true,
    clearLive: false,
  });
  assert.equal(planEvent('agent', ev(2, 'run.completed', run, { result: null, usage: null }), 1).clearLive, true);
  assert.equal(planEvent('workflow', ev(3, 'workflow.failed', {}, { reason: 'r', usage: null }), 2).clearLive, true);
});

test('구세대 행(context null)은 적재·전진만 하고 op은 없다', () => {
  assert.deepEqual(planEvent('agent', ev(1, 'run.started', null, { run: 1 }), 0), {
    advance: true,
    ops: [],
    sealTurn: false,
    clearLive: false,
  });
});

test('종료 판정 — sync 전엔 절대, sync 후 닫힘 또는 파킹', () => {
  assert.equal(shouldStop({ synced: false, open: false, parked: false }), false);
  assert.equal(shouldStop({ synced: true, open: true, parked: false }), false);
  assert.equal(shouldStop({ synced: true, open: false, parked: false }), true);
  assert.equal(shouldStop({ synced: true, open: true, parked: true }), true);
});

test('404 백오프는 1s→30s 상한, 누적 5분을 넘으면 포기', () => {
  assert.equal(absentDelay(1, 0), 1000);
  assert.equal(absentDelay(2, 1000), 2000);
  assert.equal(absentDelay(6, 60_000), 30_000);
  assert.equal(absentDelay(7, 5 * 60_000 + 1), null);
});

test('라이브 스냅샷 필드는 마지막 채널을 맨 뒤에 내고 seed를 단다', () => {
  const context = { agent, run: 1, turn: 1, attempt: 1 };
  const fields = {
    [liveFieldKey({ source: 'SESSION' }, agent.id)]: {
      context,
      text: '안녕',
      textBroken: false,
      thinkingChars: 12,
      toolInput: null,
      last: 'thinking' as const,
      seeded: false,
    },
    [liveFieldKey({ source: 'WORKFLOW', workflowId: 'wf_1' }, 'agent_9')]: {
      context: { ...context, agent: { id: 'agent_9', name: 'j' } },
      text: '',
      textBroken: false,
      thinkingChars: 0,
      toolInput: { name: 'read' },
      last: 'tool.input' as const,
      seeded: false,
    },
  };
  assert.deepEqual(liveSnapshotFrames(fields), [
    { context, channel: 'text', offset: 0, data: '안녕', seed: true },
    { context, channel: 'thinking', chars: 12, seed: true },
    {
      context: { ...context, agent: { id: 'agent_9', name: 'j' } },
      channel: 'tool.input',
      tool: { id: null, name: 'read' },
      seed: true,
      workflowId: 'wf_1',
    },
  ]);
});

test('라이브 스냅샷은 applyDelta로 왕복한다 — 루트·자식 필드가 각각 복원된다', () => {
  const fold = (deltas: ProjectedDeltaFrame[]): TurnLive => {
    let live: TurnLive | null = null;
    for (const delta of deltas) live = applyDelta(live, delta);
    assert.ok(live !== null);
    return live;
  };

  const rootContext = { agent, run: 2, turn: 3, attempt: 1 };
  const childContext = { agent: { id: 'agent_9', name: 'judge' }, run: 1, turn: 1, attempt: 2 };

  const root = fold([
    { context: rootContext, channel: 'text', offset: 0, data: '안녕' },
    { context: rootContext, channel: 'text', offset: 2, data: '하세요' },
    { context: rootContext, channel: 'thinking', chars: 42 },
    { context: rootContext, channel: 'tool.input', tool: { id: 't1', name: 'read' } },
  ]);
  const child = fold([
    { context: childContext, channel: 'thinking', chars: 7, workflowId: 'wf_1' },
    { context: childContext, channel: 'text', offset: 0, data: '초안', workflowId: 'wf_1' },
    { context: childContext, channel: 'text', offset: 2, data: ' 정리', workflowId: 'wf_1' },
  ]);
  assert.equal(root.text, '안녕하세요');
  assert.equal(child.last, 'text');

  const rootKey = liveFieldKey({ source: 'SESSION' }, agent.id);
  const childKey = liveFieldKey({ source: 'WORKFLOW', workflowId: 'wf_1' }, childContext.agent.id);
  const byField = new Map<string, ProjectedDeltaFrame[]>();
  for (const frame of liveSnapshotFrames({ [rootKey]: root, [childKey]: child })) {
    const key = liveFieldKey(
      frame.workflowId === undefined ? { source: 'SESSION' } : { source: 'WORKFLOW', workflowId: frame.workflowId },
      frame.context.agent.id,
    );
    byField.set(key, [...(byField.get(key) ?? []), frame]);
  }

  for (const [key, origin] of [
    [rootKey, root],
    [childKey, child],
  ] as const) {
    const restored = fold(byField.get(key) ?? []);
    assert.deepEqual(restored.context, origin.context);
    assert.equal(restored.text, origin.text);
    assert.equal(restored.thinkingChars, origin.thinkingChars);
    assert.deepEqual(restored.toolInput, origin.toolInput);
    assert.equal(restored.last, origin.last);
    assert.equal(restored.seeded, true);
  }
});

test('프레임 게이트는 이벤트 seq 중복을 거르고 델타·sync는 통과시킨다', () => {
  const gate = createFrameGate(3, new Map([['wf_1', 2]]));
  const event = (seq: number, workflowId?: string) => ({
    type: 'event' as const,
    event: {
      seq,
      occurredAt: 0,
      context: {},
      source: workflowId === undefined ? ('SESSION' as const) : ('WORKFLOW' as const),
      workflowId,
      kind: 'run.completed' as const,
      data: {},
    },
  });
  assert.equal(gate.accept(event(3)), false);
  assert.equal(gate.accept(event(4)), true);
  assert.equal(gate.accept(event(4)), false);
  assert.equal(gate.accept(event(2, 'wf_1')), false);
  assert.equal(gate.accept(event(3, 'wf_1')), true);
  assert.equal(gate.accept(event(1, 'wf_2')), true);
  assert.equal(gate.accept({ type: 'sync', seq: 4 }), true);
  assert.equal(
    gate.accept({ type: 'delta', delta: { context: { agent, run: 1, turn: 1, attempt: 1 }, channel: 'text', offset: 0, data: 'x' } }),
    true,
  );
});
