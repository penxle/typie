import { describe, expect, it } from 'vitest';
import { applyFrame, emptyTranscript } from './conversation.ts';
import { fromGraphQL, toGraphQL } from './transcript-wire.ts';
import type { ProjectedEventData, ProjectedStreamFrame } from './projected.ts';
import type { RunStateWire } from './transcript-wire.ts';

const agent = { id: 'chat-1', name: 'chat' };
const child = { id: 'agent_9', name: 'judgment' };

const ev = (seq: number, kind: string, context: Record<string, unknown>, data: Record<string, unknown>): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 1_700_000_000_000 + seq,
    context: context as never,
    source: 'SESSION',
    ...({ kind, data } as ProjectedEventData),
  },
});
const wf = (
  seq: number,
  kind: string,
  context: Record<string, unknown>,
  data: Record<string, unknown>,
  workflowId = 'wf_1',
): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 1_700_000_100_000 + seq,
    context: context as never,
    source: 'WORKFLOW',
    workflowId,
    ...({ kind, data } as ProjectedEventData),
  },
});
const reduce = (frames: ProjectedStreamFrame[]) => frames.reduce(applyFrame, emptyTranscript());

const run1 = { agent, run: 1 };
const turn1 = { agent, run: 1, turn: 1, attempt: 1 };
const call = { ...turn1, toolCallId: 'c1' };
const inv = { ...call, invocation: 'inv_1' };

const session = [
  ev(1, 'agent.created', { agent }, {}),
  ev(2, 'run.started', run1, { message: '안녕', command: null }),
  ev(3, 'turn.started', turn1, {}),
  ev(4, 'turn.completed', turn1, { text: null, toolCalls: [{ kind: 'parsed', id: 'c1', name: 'list-open-documents', input: {} }] }),
  ev(5, 'tool.requested', call, { tool: 'list-open-documents', data: {} }),
  ev(6, 'tool.resolved', call, { tool: 'list-open-documents', ok: true, data: { documents: [] }, resolvedBy: 'user' }),
  ev(7, 'invocation.started', inv, { target: { kind: 'workflow', id: 'wf_1', name: 'high', app: 'review' } }),
  wf(1, 'step.started', { step: 'classify-0' }, {}),
  wf(2, 'tool.requested', { agent: child, run: 1, turn: 1, attempt: 1, toolCallId: 'k1' }, { tool: 'ask-user', data: { questions: [] } }),
  wf(
    3,
    'tool.resolved',
    { agent: child, run: 1, turn: 1, attempt: 1, toolCallId: 'k1' },
    { tool: 'ask-user', ok: true, data: { answers: [] } },
  ),
  wf(4, 'turn.completed', { agent: child, run: 1, turn: 1, attempt: 1, step: 'classify-0' }, { text: '자식 턴' }),
  wf(
    5,
    'tool.executed',
    { agent: child, run: 1, turn: 2, attempt: 1, toolCallId: 'k2', step: 'classify-0' },
    { tool: 'read', ok: true, input: { path: 'a.md' } },
  ),
  wf(6, 'step.completed', { step: 'classify-0' }, {}),
  wf(7, 'workflow.completed', {}, {}),
  ev(8, 'invocation.completed', inv, {}),
  ev(9, 'turn.completed', { ...turn1, turn: 2 }, { text: '끝', toolCalls: [] }),
  ev(10, 'assistant.titled', call, { title: '제목' }),
  ev(11, 'run.completed', run1, {}),
  ev(12, 'run.started', { agent, run: 2 }, { message: '둘째', command: null }),
  ev(13, 'tool.executed', { ...turn1, run: 2, toolCallId: 'c3' }, { tool: 'web-search', ok: false }),
  ev(14, 'run.failed', { agent, run: 2 }, {}),
];

const states: RunStateWire[] = ['COMPLETED', 'FAILED'];
const roundTrip = (frames: ProjectedStreamFrame[], runStates: RunStateWire[]) => {
  const t = reduce(frames);
  const wire = toGraphQL(t);
  return { t, back: fromGraphQL({ ...wire, runs: wire.runs.map((run, index) => ({ ...run, state: runStates[index] ?? 'COMPLETED' })) }) };
};

describe('toGraphQL', () => {
  it('run.started마다 run을 열고 항목을 순서대로 묶는다 — 워크플로 요청은 워크플로 항목 뒤에 온다', () => {
    const wire = toGraphQL(reduce(session));
    expect(wire.runs.map((run) => run.runSeq)).toEqual([1, 2]);
    expect(wire.runs[0].items.map((item) => item.kind)).toEqual([
      'user',
      'assistant',
      'toolRequest',
      'workflow',
      'toolRequest',
      'assistant',
    ]);
    expect(wire.runs[1].items.map((item) => item.kind)).toEqual(['user', 'tool', 'runFailure']);
    expect(wire.runs[0].items[2]).toMatchObject({ kind: 'toolRequest', toolCallId: 'c1', status: 'RESOLVED', resolvedBy: 'USER' });
    expect(wire).toMatchObject({ cursor: 14, title: '제목', agentId: agent.id, turn: 'IDLE', retrying: false });
  });

  it('시각은 ISO 문자열, 워크플로 transcript는 단계·턴·도구를 싣는다', () => {
    const wire = toGraphQL(reduce(session));
    const workflow = wire.runs[0].items.find((item) => item.kind === 'workflow');
    expect(workflow).toMatchObject({
      prismWorkflowId: 'wf_1',
      app: 'review',
      name: 'high',
      status: 'COMPLETED',
      invocation: 'inv_1',
      cursor: 7,
    });
    expect(workflow?.kind === 'workflow' ? workflow.transcript.steps[0] : null).toMatchObject({ name: 'classify-0', seq: 1 });
    expect(workflow?.kind === 'workflow' ? workflow.transcript.tools[0] : null).toMatchObject({ tool: 'read', path: 'a.md', query: null });
    expect(wire.runs[0].items[0]).toMatchObject({ kind: 'user', at: new Date(1_700_000_000_002).toISOString() });
  });

  it('진행 중 턴은 ACTIVE, 재시도 중은 retrying', () => {
    const t = reduce([ev(1, 'run.started', run1, { message: 'a', command: null }), ev(2, 'turn.started', turn1, {})]);
    expect(toGraphQL(t)).toMatchObject({ turn: 'ACTIVE', retrying: false });
    expect(toGraphQL(applyFrame(t, ev(3, 'turn.retried', turn1, {})))).toMatchObject({ turn: 'ACTIVE', retrying: true });
  });
});

describe('fromGraphQL', () => {
  it('왕복은 항등이다(live는 null)', () => {
    const { t, back } = roundTrip(session, states);
    expect(back).toEqual({ ...t, live: null });
  });

  it('run 상태는 마지막 run의 state에서 파생된다', () => {
    expect(roundTrip(session, ['COMPLETED', 'RUNNING']).back.run).toBe('running');
    expect(roundTrip(session, ['COMPLETED', 'FAILED']).back.run).toBe('failed');
    expect(roundTrip(session, ['COMPLETED', 'CANCELED']).back.run).toBe('canceled');
    expect(roundTrip(session, ['COMPLETED', 'COMPLETED']).back.run).toBe('idle');
    expect(fromGraphQL({ cursor: 0, title: null, agentId: null, turn: 'IDLE', retrying: false, runs: [] })).toEqual(emptyTranscript());
  });

  it('미해소 요청·진행 중 워크플로·진행 중 턴도 왕복된다', () => {
    const frames = [
      ev(1, 'run.started', run1, { message: 'a', command: null }),
      ev(2, 'invocation.started', inv, { target: { kind: 'workflow', id: 'wf_1', name: 'high', app: 'review' } }),
      wf(
        1,
        'tool.requested',
        { agent: child, run: 1, turn: 1, attempt: 1, toolCallId: 'k1' },
        { tool: 'ask-user', data: { questions: [{ question: 'q', hint: '', multi: false, options: [] }] } },
      ),
      ev(3, 'turn.started', turn1, {}),
    ];
    const { t, back } = roundTrip(frames, ['RUNNING']);
    expect(back).toEqual({ ...t, live: null });
    expect(back.run).toBe('running');
  });

  it('모든 live는 왕복에서 비워진다 — 워크플로 안쪽 live도 구독이 다시 채운다', () => {
    const frames: ProjectedStreamFrame[] = [
      ev(1, 'run.started', run1, { message: 'a', command: null }),
      ev(2, 'invocation.started', inv, { target: { kind: 'workflow', id: 'wf_1', name: 'high', app: 'review' } }),
      {
        type: 'delta',
        delta: { context: { agent: child, run: 1, turn: 1, attempt: 1 }, channel: 'text', offset: 0, data: '스트리밍', workflowId: 'wf_1' },
      },
    ];
    const t = reduce(frames);
    const workflow = t.messages.find((message) => message.role === 'workflow');
    expect(workflow?.role === 'workflow' ? workflow.transcript.live?.text : null).toBe('스트리밍');

    const { back } = roundTrip(frames, ['RUNNING']);
    const backWorkflow = back.messages.find((message) => message.role === 'workflow');
    expect(backWorkflow?.role === 'workflow' ? backWorkflow.transcript.live : undefined).toBeNull();
    expect(back).toEqual({
      ...t,
      live: null,
      messages: t.messages.map((message) =>
        message.role === 'workflow' ? { ...message, transcript: { ...message.transcript, live: null } } : message,
      ),
    });
  });
});
