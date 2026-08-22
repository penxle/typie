import { describe, expect, it } from 'vitest';
import { applyFrame, emptyTranscript, pendingRootRequests, runningWorkflows } from './conversation.ts';
import type { ProjectedEventData, ProjectedStreamFrame } from '@typie/prism';

const agent = { id: 'typie-1', name: 'assistant' };
const child = { id: 'agent_9', name: 'judgment-high' };

const ev = (seq: number, kind: string, context: Record<string, unknown>, data: Record<string, unknown>): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 1000 + seq,
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
  workflowId = 'workflow_1',
): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 2000 + seq,
    context: context as never,
    source: 'WORKFLOW',
    workflowId,
    ...({ kind, data } as ProjectedEventData),
  },
});

const run = { agent, run: 1 };
const turn = { ...run, turn: 1, attempt: 1 };
const tool = { ...turn, toolCallId: 'c1' };
const childTool = { agent: child, run: 1, turn: 2, attempt: 1, toolCallId: 'q1' };
const target = { kind: 'workflow', id: 'workflow_1', name: 'high', app: 'app_1' };

const reduce = (frames: ProjectedStreamFrame[]) => frames.reduce(applyFrame, emptyTranscript());

const started = (seq: number) => ev(seq, 'invocation.started', { ...tool, invocation: 'i1' }, { target });

describe('applyFrame', () => {
  it('run.started는 사용자 메시지 + running, 종결은 상태·live 봉인', () => {
    let t = reduce([ev(1, 'run.started', run, { message: '안녕' })]);
    expect(t.messages[0]).toMatchObject({ role: 'user', text: '안녕' });
    expect(t).toMatchObject({ run: 'running', cursor: 1 });
    t = applyFrame(t, ev(2, 'run.failed', run, {}));
    expect(t).toMatchObject({ run: 'failed', live: null, cursor: 2 });
    expect(t.messages.at(-1)).toMatchObject({ role: 'run-failed', key: 'e2' });
  });

  it('커맨드 run은 말풍선을 /이름 인자로 복원하고, 평문 run은 message 그대로', () => {
    const expanded = '<command name="리뷰">\n본문\n</command>';
    expect(reduce([ev(1, 'run.started', run, { message: expanded, command: { name: '리뷰', args: '' } })]).messages[0]).toMatchObject({
      role: 'user',
      text: '/리뷰',
    });
    expect(reduce([ev(1, 'run.started', run, { message: expanded, command: { name: '리뷰', args: '2장만' } })]).messages[0]).toMatchObject({
      role: 'user',
      text: '/리뷰 2장만',
    });
    expect(reduce([ev(1, 'run.started', run, { message: '안녕', command: null })]).messages[0]).toMatchObject({
      role: 'user',
      text: '안녕',
    });
  });

  it('turn.completed는 assistant 메시지(텍스트·toolCalls 이름)를 덧붙이고 live를 봉인한다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' })]);
    t = applyFrame(t, { type: 'delta', delta: { context: turn, channel: 'text', offset: 0, data: '안' } });
    expect(t.live?.text).toBe('안');
    t = applyFrame(
      t,
      ev(2, 'turn.completed', turn, { text: '안녕하세요', toolCalls: [{ kind: 'parsed', id: 'c1', name: 'read', input: {} }] }),
    );
    expect(t.live).toBeNull();
    expect(t.messages.at(-1)).toMatchObject({ role: 'assistant', text: '안녕하세요', toolCalls: [{ id: 'c1', name: 'read' }], at: 1002 });
  });

  it('델타가 흘러온 턴의 assistant 메시지는 streamed로 표시된다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' })]);
    t = applyFrame(t, { type: 'delta', delta: { context: turn, channel: 'text', offset: 0, data: '안녕하' } });
    t = applyFrame(t, { type: 'delta', delta: { context: turn, channel: 'text', offset: 3, data: '세요' } });
    t = applyFrame(t, ev(2, 'turn.completed', turn, { text: '안녕하세요', toolCalls: [] }));
    expect(t.messages.at(-1)).toMatchObject({ role: 'assistant', streamed: true });
  });

  it('델타 없이 닫힌 턴과 seed로 받은 턴은 streamed가 아니다', () => {
    const noDelta = reduce([ev(1, 'run.started', run, { message: 'a' }), ev(2, 'turn.completed', turn, { text: '안녕', toolCalls: [] })]);
    expect(noDelta.messages.at(-1)).toMatchObject({ role: 'assistant', streamed: false });

    let seeded = reduce([ev(1, 'run.started', run, { message: 'a' })]);
    seeded = applyFrame(seeded, { type: 'delta', delta: { context: turn, channel: 'text', offset: 0, data: '안녕', seed: true } });
    seeded = applyFrame(seeded, ev(2, 'turn.completed', turn, { text: '안녕', toolCalls: [] }));
    expect(seeded.messages.at(-1)).toMatchObject({ role: 'assistant', streamed: false });
  });

  it('텍스트도 toolCalls도 없는 turn.completed는 메시지를 만들지 않는다', () => {
    const t = reduce([ev(1, 'run.started', run, { message: 'a' }), ev(2, 'turn.completed', turn, { text: null, toolCalls: [] })]);
    expect(t.messages).toHaveLength(1);
  });

  it('tool.executed/rejected는 phase·ok를 가진 tool 메시지', () => {
    const t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      ev(2, 'tool.executed', tool, { tool: 'read', ok: true }),
      ev(3, 'tool.rejected', { ...tool, toolCallId: 'c2' }, { tool: 'zzz' }),
    ]);
    expect(t.messages.slice(1)).toMatchObject([
      { role: 'tool', name: 'read', phase: 'executed', ok: true },
      { role: 'tool', name: 'zzz', phase: 'rejected', ok: false },
    ]);
  });

  it('turn은 turn.started~turn.completed 사이에만 active다 — 도구 대기·run 종결은 idle', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' })]);
    expect(t.turn).toBe('idle');
    t = applyFrame(t, ev(2, 'turn.started', turn, {}));
    expect(t.turn).toBe('active');
    t = applyFrame(t, ev(3, 'turn.completed', turn, { text: null, toolCalls: [{ kind: 'parsed', id: 'c1', name: 'x', input: {} }] }));
    expect(t.turn).toBe('idle');
    t = applyFrame(t, ev(4, 'tool.requested', { ...turn, toolCallId: 'c1' }, { tool: 'x', data: {} }));
    expect(t.turn).toBe('idle');
    t = applyFrame(t, ev(5, 'turn.retried', turn, {}));
    expect(t.turn).toBe('active');
    t = applyFrame(t, ev(6, 'run.canceled', run, {}));
    expect(t.turn).toBe('idle');
  });

  it('turn.retried는 retrying 표지, turn.started가 지운다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' }), ev(2, 'turn.retried', turn, {})]);
    expect(t.retrying).toBe(true);
    t = applyFrame(t, ev(3, 'turn.started', { ...turn, attempt: 2 }, {}));
    expect(t.retrying).toBe(false);
  });

  it('run.completed는 idle, run.canceled는 canceled', () => {
    const base = [ev(1, 'run.started', run, { message: 'a' })];
    expect(reduce([...base, ev(2, 'run.completed', run, {})])).toMatchObject({ run: 'idle' });
    expect(reduce([...base, ev(2, 'run.canceled', run, {})])).toMatchObject({ run: 'canceled' });
    expect(reduce([...base, ev(2, 'run.canceled', run, {})]).messages.some((m) => m.role === 'run-failed')).toBe(false);
  });

  it('sync는 커서 high-water를 올리고, 지난 seq는 무시한다', () => {
    let t = reduce([ev(5, 'run.started', run, { message: 'a' })]);
    t = applyFrame(t, ev(4, 'run.completed', run, {}));
    expect(t.run).toBe('running');
    t = applyFrame(t, { type: 'sync', seq: 6 });
    expect(t).toMatchObject({ cursor: 6, messages: [{ role: 'user' }] });
  });
});

describe('agent.created', () => {
  it('첫 SESSION 프레임의 context가 루트 agentId를 준다', () => {
    const t = reduce([ev(1, 'agent.created', { agent }, {}), ev(2, 'run.started', run, { message: 'a' })]);
    expect(t.agentId).toBe('typie-1');
    expect(t.messages).toHaveLength(1);
  });

  it('agentId는 첫 값으로 굳는다 — 자식 agent가 실린 WORKFLOW 프레임이 덮어쓰지 않는다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' }), started(2)]);
    t = applyFrame(t, wf(1, 'tool.requested', childTool, { tool: 'ask-user', data: { questions: [] } }));
    expect(t.agentId).toBe('typie-1');
  });

  it('context.agent가 없는 도구 요청은 루트 agentId로 되돌아간다', () => {
    const t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      started(2),
      wf(3, 'tool.requested', { run: 1, turn: 2, attempt: 1, toolCallId: 'q1' }, { tool: 'ask-user', data: { questions: [] } }),
    ]);

    expect(t.messages.at(-1)).toMatchObject({ role: 'tool-request', agentId: 'typie-1' });
  });

  it('빈 트랜스크립트의 agentId는 null', () => {
    expect(emptyTranscript().agentId).toBeNull();
  });
});

describe('tool-request 변형', () => {
  it('SESSION tool.requested는 루트 tool-request(pending)를 세우고 agentId·data를 담는다', () => {
    const t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      ev(2, 'tool.requested', tool, { tool: 'confirm-review', data: { tier: 'medium' } }),
    ]);
    expect(t.messages.at(-1)).toMatchObject({
      role: 'tool-request',
      tool: 'confirm-review',
      toolCallId: 'c1',
      agentId: 'typie-1',
      workflowId: undefined,
      data: { tier: 'medium' },
      status: 'pending',
      at: 1002,
    });
    expect(pendingRootRequests(t)).toHaveLength(1);
  });

  it('같은 toolCallId의 tool.resolved는 새 메시지 없이 resolved + result를 남긴다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' }), ev(2, 'tool.requested', tool, { tool: 'confirm-review', data: {} })]);
    t = applyFrame(t, ev(3, 'tool.resolved', tool, { tool: 'confirm-review', ok: true, data: { decision: 'declined' } }));
    expect(t.messages).toHaveLength(2);
    expect(t.messages.at(-1)).toMatchObject({ role: 'tool-request', status: 'resolved', result: { decision: 'declined' } });
    expect(pendingRootRequests(t)).toHaveLength(0);
  });

  it('result data가 없는 해소도 resolved로 닫는다', () => {
    let t = reduce([ev(1, 'tool.requested', tool, { tool: 'list-open-documents', data: {} })]);
    t = applyFrame(t, ev(2, 'tool.resolved', tool, { tool: 'list-open-documents', ok: true }));
    expect(t.messages).toHaveLength(1);
    expect(t.messages[0]).toMatchObject({ role: 'tool-request', status: 'resolved' });
    expect((t.messages[0] as { result?: unknown }).result).toBeUndefined();
  });

  it('다른 toolCallId의 해소는 아무 요청도 건드리지 않는다', () => {
    let t = reduce([ev(1, 'tool.requested', tool, { tool: 'ask-user', data: { questions: [] } })]);
    t = applyFrame(t, ev(2, 'tool.resolved', { ...tool, toolCallId: 'zz' }, { tool: 'ask-user', ok: true, data: { answers: [] } }));
    expect(t.messages).toHaveLength(1);
    expect(t.messages[0]).toMatchObject({ status: 'pending' });
  });

  it('WORKFLOW tool.requested는 workflowId를 단 tool-request를 세우고 같은 seq 재생은 무시된다', () => {
    const questions = [{ question: 'Q?', hint: 'h', multi: false, options: [{ label: 'a' }] }];
    let t = reduce([started(1), wf(3, 'tool.requested', childTool, { tool: 'ask-user', data: { questions } })]);
    expect(t.messages.at(-1)).toMatchObject({
      role: 'tool-request',
      tool: 'ask-user',
      workflowId: 'workflow_1',
      agentId: 'agent_9',
      toolCallId: 'q1',
      data: { questions },
      status: 'pending',
    });
    expect(pendingRootRequests(t)).toHaveLength(0);

    t = applyFrame(t, wf(3, 'tool.requested', childTool, { tool: 'ask-user', data: { questions } }));
    expect(t.messages).toHaveLength(2);

    t = applyFrame(
      t,
      wf(4, 'tool.resolved', childTool, { tool: 'ask-user', ok: true, data: { answers: [{ question: 'Q?', choice: ['a'] }] } }),
    );
    expect(t.messages.at(-1)).toMatchObject({ status: 'resolved', result: { answers: [{ question: 'Q?', choice: ['a'] }] } });
  });

  it('루트 run 종결은 루트 pending만 닫고 워크플로 pending은 남긴다', () => {
    const t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      started(2),
      wf(3, 'tool.requested', childTool, { tool: 'ask-user', data: { questions: [] } }),
      ev(4, 'tool.requested', { ...tool, toolCallId: 'c2' }, { tool: 'confirm-review', data: {} }),
      ev(5, 'run.canceled', run, {}),
    ]);
    expect(t.run).toBe('canceled');
    expect(t.messages.filter((m) => m.role === 'tool-request')).toMatchObject([
      { toolCallId: 'q1', status: 'pending' },
      { toolCallId: 'c2', status: 'closed' },
    ]);
    expect(pendingRootRequests(t)).toHaveLength(0);
  });

  it('워크플로 종결은 그 워크플로의 pending을 닫는다', () => {
    let t = reduce([started(1), wf(2, 'tool.requested', childTool, { tool: 'ask-user', data: { questions: [] } })]);
    t = applyFrame(t, wf(3, 'workflow.failed', { agent: child }, {}));
    expect(t.messages.at(-1)).toMatchObject({ role: 'tool-request', status: 'closed', settledAt: 2003 });
  });

  it('resolved된 요청은 이후 종결이 되돌리지 않는다', () => {
    let t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      ev(2, 'tool.requested', tool, { tool: 'confirm-review', data: {} }),
      ev(3, 'tool.resolved', tool, {
        tool: 'confirm-review',
        ok: true,
        data: {
          decision: 'confirmed',
          key: 'r1',
          tier: 'medium',
          document: { id: 'D0DOC1', title: 't', subtitle: null, path: 'manuscript/r1.md' },
        },
      }),
    ]);
    t = applyFrame(t, ev(4, 'run.completed', run, {}));
    expect(t.messages.at(-1)).toMatchObject({ status: 'resolved' });
  });
});

describe('workflow 변형', () => {
  it('invocation.started(workflow)는 app·name을 담은 workflow 메시지를 세운다', () => {
    const t = reduce([started(1)]);
    expect(t.messages[0]).toMatchObject({
      role: 'workflow',
      workflowId: 'workflow_1',
      app: 'app_1',
      name: 'high',
      status: 'running',
      startedAt: 1001,
      cursor: 0,
    });
    expect(runningWorkflows(t)).toHaveLength(1);
  });

  it('invocation.started(agent)는 아무것도 만들지 않는다', () => {
    expect(reduce([ev(1, 'invocation.started', tool, { target: { kind: 'agent' } })]).messages).toHaveLength(0);
  });

  it('workflow.* 종결이 status와 cursor를 옮긴다', () => {
    let t = reduce([started(1)]);
    t = applyFrame(t, wf(7, 'workflow.completed', { agent: child }, {}));
    expect(t.messages[0]).toMatchObject({ role: 'workflow', status: 'completed', cursor: 7 });
    expect(runningWorkflows(t)).toHaveLength(0);
  });

  it('invocation.* 종결도 status를 닫는다', () => {
    for (const [kind, status] of [
      ['invocation.completed', 'completed'],
      ['invocation.failed', 'failed'],
      ['invocation.canceled', 'canceled'],
    ] as const) {
      const t = applyFrame(reduce([started(1)]), ev(2, kind, { ...tool, invocation: 'i1' }, {}));
      expect(t.messages[0]).toMatchObject({ role: 'workflow', status });
      expect(runningWorkflows(t)).toHaveLength(0);
    }
  });

  it('invocation.* 종결은 invocation id로 짝지어 그 워크플로만 닫는다', () => {
    let t = reduce([
      started(1),
      ev(2, 'invocation.started', { ...tool, invocation: 'i2' }, { target: { ...target, id: 'workflow_2', name: 'low' } }),
      wf(3, 'tool.requested', childTool, { tool: 'ask-user', data: { questions: [] } }, 'workflow_1'),
      wf(3, 'tool.requested', { ...childTool, toolCallId: 'q2' }, { tool: 'ask-user', data: { questions: [] } }, 'workflow_2'),
    ]);

    t = applyFrame(t, ev(4, 'invocation.completed', { ...tool, invocation: 'i1' }, {}));

    expect(t.messages.filter((m) => m.role === 'workflow')).toMatchObject([
      { workflowId: 'workflow_1', status: 'completed' },
      { workflowId: 'workflow_2', status: 'running' },
    ]);
    expect(t.messages.filter((m) => m.role === 'tool-request')).toMatchObject([
      { toolCallId: 'q1', status: 'closed' },
      { toolCallId: 'q2', status: 'pending' },
    ]);
    expect(runningWorkflows(t)).toHaveLength(1);
  });

  it('invocation id가 없는 종결은 가장 최근 running 워크플로를 닫는다', () => {
    let t = reduce([
      started(1),
      ev(2, 'invocation.started', { ...tool, invocation: 'i2' }, { target: { ...target, id: 'workflow_2', name: 'low' } }),
    ]);

    t = applyFrame(t, ev(3, 'invocation.completed', tool, {}));

    expect(t.messages.filter((m) => m.role === 'workflow')).toMatchObject([
      { workflowId: 'workflow_1', status: 'running' },
      { workflowId: 'workflow_2', status: 'completed' },
    ]);
  });

  it('양쪽 다 invocation id를 가졌는데 짝이 없으면 아무 워크플로도 닫지 않는다', () => {
    let t = reduce([
      started(1),
      ev(2, 'invocation.started', { ...tool, invocation: 'i2' }, { target: { ...target, id: 'workflow_2', name: 'low' } }),
      wf(3, 'tool.requested', childTool, { tool: 'ask-user', data: { questions: [] } }, 'workflow_1'),
      wf(3, 'tool.requested', { ...childTool, toolCallId: 'q2' }, { tool: 'ask-user', data: { questions: [] } }, 'workflow_2'),
    ]);

    t = applyFrame(t, ev(4, 'invocation.failed', { ...tool, invocation: 'i9' }, {}));

    expect(runningWorkflows(t)).toHaveLength(2);
    expect(t.messages.filter((m) => m.role === 'tool-request')).toMatchObject([
      { toolCallId: 'q1', status: 'pending' },
      { toolCallId: 'q2', status: 'pending' },
    ]);
  });

  it('워크플로 여럿은 각자의 커서를 쓴다', () => {
    let t = reduce([
      started(1),
      ev(2, 'invocation.started', { ...tool, invocation: 'i2' }, { target: { ...target, id: 'workflow_2', name: 'low' } }),
    ]);
    t = applyFrame(t, wf(5, 'workflow.completed', { agent: child }, {}, 'workflow_1'));
    t = applyFrame(t, wf(3, 'workflow.failed', { agent: child }, {}, 'workflow_2'));
    expect(t.messages).toMatchObject([
      { workflowId: 'workflow_1', status: 'completed', cursor: 5 },
      { workflowId: 'workflow_2', status: 'failed', cursor: 3 },
    ]);
  });

  it('assistant.titled는 제목을 덮어쓰고(마지막 값 우선) 커서를 올리되 메시지는 만들지 않는다', () => {
    expect(emptyTranscript().title).toBeNull();
    let t = reduce([ev(1, 'run.started', run, { message: '안녕' }), ev(2, 'assistant.titled', tool, { title: '첫 제목' })]);
    expect(t).toMatchObject({ title: '첫 제목', cursor: 2 });
    expect(t.messages).toHaveLength(1);
    t = applyFrame(t, ev(3, 'assistant.titled', tool, { title: '둘째 제목' }));
    expect(t).toMatchObject({ title: '둘째 제목', cursor: 3 });
    expect(applyFrame(t, ev(2, 'assistant.titled', tool, { title: '재생' }))).toBe(t);
  });

  it('WORKFLOW 프레임은 루트 커서를 올리지 않고, workflowId 없는 프레임과 미지의 워크플로는 버려진다', () => {
    let t = reduce([started(1)]);
    expect(t.cursor).toBe(1);
    t = applyFrame(t, wf(9, 'workflow.started', { agent: child }, {}));
    expect(t.cursor).toBe(1);
    expect(applyFrame(t, wf(2, 'workflow.completed', { agent: child }, {}, 'workflow_zz'))).toBe(t);

    const orphan: ProjectedStreamFrame = {
      type: 'event',
      event: {
        seq: 9,
        occurredAt: 2009,
        context: { agent } as never,
        source: 'WORKFLOW',
        ...({ kind: 'workflow.completed', data: {} } as ProjectedEventData),
      },
    };
    expect(applyFrame(t, orphan)).toBe(t);

    t = applyFrame(t, ev(2, 'run.started', run, { message: 'a' }));
    expect(t).toMatchObject({ run: 'running', cursor: 2 });
  });
});

describe('workflow trace', () => {
  const childTurn = { agent: child, run: 1, turn: 1, attempt: 1 };

  it('워크플로 step·turn·tool.executed 이벤트는 그 워크플로 메시지의 trace에 접히고 루트 메시지를 만들지 않는다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: '/리뷰' }), started(2)]);
    t = applyFrame(t, wf(1, 'step.started', { step: 'description-0' }, {}));
    t = applyFrame(t, wf(2, 'turn.completed', { ...childTurn, step: 'description-0' }, { text: '첫 구획을 읽었어요' }));
    t = applyFrame(
      t,
      wf(3, 'tool.executed', { ...childTurn, toolCallId: 'c1' }, { tool: 'read', ok: true, input: { path: 'manuscript/a.md' } }),
    );
    const workflow = t.messages.find((m) => m.role === 'workflow');
    expect(workflow?.role === 'workflow' ? workflow.trace : null).toMatchObject({
      steps: [{ name: 'description-0', completedAt: null }],
      turns: [{ text: '첫 구획을 읽었어요', step: 'description-0' }],
      tools: [{ tool: 'read', path: 'manuscript/a.md' }],
    });
    expect(t.messages.filter((m) => m.role === 'assistant')).toHaveLength(0);
    expect(workflow?.role === 'workflow' ? workflow.cursor : null).toBe(3);
  });

  it('workflowId가 있는 델타는 그 워크플로의 trace.live로 가고 루트 live는 건드리지 않는다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: '/리뷰' }), started(2)]);
    t = applyFrame(t, { type: 'delta', delta: { context: childTurn, channel: 'text', offset: 0, data: '첫', workflowId: 'workflow_1' } });
    const workflow = t.messages.find((m) => m.role === 'workflow');
    expect(workflow?.role === 'workflow' ? workflow.trace.live?.text : null).toBe('첫');
    expect(t.live).toBeNull();
    const next = applyFrame(t, {
      type: 'delta',
      delta: { context: childTurn, channel: 'text', offset: 1, data: ' 구획', workflowId: 'workflow_9' },
    });
    const after = next.messages.find((m) => m.role === 'workflow');
    expect(after?.role === 'workflow' ? after.trace.live?.text : null).toBe('첫');
    expect(next.live).toBeNull();
  });

  it('워크플로 종결은 trace.live를 비운다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: '/리뷰' }), started(2)]);
    t = applyFrame(t, { type: 'delta', delta: { context: childTurn, channel: 'text', offset: 0, data: '첫', workflowId: 'workflow_1' } });
    t = applyFrame(t, wf(5, 'workflow.canceled', {}, {}));
    const workflow = t.messages.find((m) => m.role === 'workflow');
    expect(workflow?.role === 'workflow' ? workflow.trace.live : 'x').toBeNull();
    expect(workflow).toMatchObject({ status: 'canceled', finishedAt: 2005 });
  });

  it('이미 자취가 선 워크플로를 재생해도 중복 없이 커서 이후만 얹힌다', () => {
    const live = [
      wf(1, 'step.started', { step: 'description-0' }, {}),
      wf(2, 'turn.completed', { ...childTurn, step: 'description-0' }, { text: '첫 구획을 읽었어요' }),
      wf(3, 'tool.executed', { ...childTurn, toolCallId: 'c1' }, { tool: 'read', ok: true, input: { path: 'manuscript/a.md' } }),
    ];
    const replayed = [
      ...live,
      wf(4, 'tool.executed', { ...childTurn, toolCallId: 'c2' }, { tool: 'grep', ok: true, input: { query: '회고' } }),
    ];

    let t = reduce([ev(1, 'run.started', run, { message: '/리뷰' }), started(2), ...live]);
    t = replayed.reduce(applyFrame, t);

    const workflow = t.messages.find((m) => m.role === 'workflow');
    const trace = workflow?.role === 'workflow' ? workflow.trace : null;
    expect(trace?.steps).toHaveLength(1);
    expect(trace?.turns).toHaveLength(1);
    expect(trace?.tools.map((row) => row.tool)).toEqual(['read', 'grep']);
    expect(workflow?.role === 'workflow' ? workflow.cursor : null).toBe(4);
    expect(t.messages).toHaveLength(2);
  });

  it('tool-request는 seq를 갖고 resolved·closed 시각을 settledAt에 남긴다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: '/리뷰' }), started(2)]);
    t = applyFrame(
      t,
      wf(7, 'tool.requested', childTool, {
        tool: 'ask-user',
        data: { questions: [{ question: 'Q', hint: '', multi: false, options: [{ label: 'a' }] }] },
      }),
    );
    t = applyFrame(
      t,
      wf(9, 'tool.resolved', childTool, { tool: 'ask-user', ok: true, data: { answers: [{ question: 'Q', choice: ['a'] }] } }),
    );
    const request = t.messages.find((m) => m.role === 'tool-request');
    expect(request).toMatchObject({ seq: 7, status: 'resolved', settledAt: 2009 });
  });
});
