import assert from 'node:assert/strict';
import test from 'node:test';
import { projectFrame } from './prism-events.ts';

const ev = (kind: string, data: Record<string, unknown>, seq = 1) => ({
  type: 'event' as const,
  event: { seq, kind, occurredAt: 10, loggedAt: 11, context: { agent: { id: 'a', name: 'x' }, run: 1 }, data },
});

test('heartbeat는 차단, sync는 프레임 그대로, 델타는 text만 원문, thinking은 글자 수, tool.input은 도구 정보만 나간다', () => {
  assert.equal(projectFrame({ type: 'heartbeat' }, { source: 'SESSION' }), null);
  assert.deepEqual(projectFrame({ type: 'sync', seq: 9 }, { source: 'SESSION' }), { type: 'sync', seq: 9 });

  const context = { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 };
  const text = projectFrame({ type: 'delta', delta: { context, channel: 'text', offset: 2, data: '녕' } }, { source: 'SESSION' });
  assert.deepEqual(text, { type: 'delta', delta: { context, channel: 'text', offset: 2, data: '녕' } });

  const thinking = projectFrame(
    { type: 'delta', delta: { context, channel: 'thinking', offset: 10, data: '내부 사고 원문' } },
    { source: 'SESSION' },
  );
  assert.deepEqual(thinking, { type: 'delta', delta: { context, channel: 'thinking', chars: 18 } });

  const toolInput = projectFrame(
    {
      type: 'delta',
      delta: { context, channel: 'tool.input', offset: 3, data: '{"a', tool: { id: null, name: 'read' } },
    },
    { source: 'SESSION' },
  );
  assert.deepEqual(toolInput, { type: 'delta', delta: { context, channel: 'tool.input', tool: { id: null, name: 'read' } } });
});

test('허용 kind는 스키마 strip으로 소비 필드만 남고, 미허용 kind는 차단된다', () => {
  const started = projectFrame(ev('run.started', { message: 'm', key: 'k', tag: null }), { source: 'SESSION' });
  assert.equal(started?.type, 'event');
  assert.deepEqual(started.event.data, { message: 'm', command: null });

  const turn = projectFrame(
    ev('turn.completed', { text: 't', thinking: '생각', toolCalls: [], stopReason: 'end', usage: null, raw: { big: true } }),
    { source: 'SESSION' },
  );
  assert.deepEqual(turn?.type === 'event' ? turn.event.data : null, { text: 't', toolCalls: [] });

  const failed = projectFrame(ev('run.failed', { reason: 'vendor detail', usage: null }), { source: 'SESSION' });
  assert.deepEqual(failed?.type === 'event' ? failed.event.data : null, {});

  const executed = projectFrame(
    ev('tool.executed', { tool: 'read', input: { path: 'x'.repeat(100_000) }, output: 'o', ok: true, data: null, duration: 1 }),
    { source: 'SESSION' },
  );
  assert.deepEqual(executed?.type === 'event' ? executed.event.data : null, { tool: 'read', ok: true });

  assert.equal(projectFrame(ev('run.reentered', { cause: 'watchdog', count: 1, interval: null }), { source: 'SESSION' }), null);
});

test('앱 이벤트 assistant.titled는 SESSION에서 title만 남기고 통과하며 WORKFLOW에서는 차단된다', () => {
  const context = { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1, toolCallId: 'c1' };
  const base = ev('assistant.titled', { title: '바다를 향한 걸음', extra: 1 });
  const frame = { ...base, event: { ...base.event, context } };
  const titled = projectFrame(frame, { source: 'SESSION' });
  assert.deepEqual(titled?.type === 'event' ? { kind: titled.event.kind, data: titled.event.data, context: titled.event.context } : null, {
    kind: 'assistant.titled',
    data: { title: '바다를 향한 걸음' },
    context,
  });
  assert.equal(projectFrame(frame, { source: 'WORKFLOW', workflowId: 'workflow_1' }), null);
  assert.throws(() => projectFrame(ev('assistant.titled', { name: 'x' }), { source: 'SESSION' }));
});

test('허용 kind의 형태가 선언과 어긋나면 삼키지 않고 던진다', () => {
  assert.throws(() => projectFrame(ev('turn.completed', { thinking: 1 }), { source: 'SESSION' }));
  assert.throws(() => projectFrame(ev('tool.executed', { tool: 'read' }), { source: 'SESSION' }));
});

test('사영 프레임은 JSON 왕복(GraphQL 스칼라)에서 구조가 보존되고, 이벤트 봉투는 {seq, occurredAt, context, source}뿐이다', () => {
  const started = projectFrame(ev('run.started', { message: '안녕', key: 'k', tag: null }, 3), { source: 'SESSION' });
  assert.ok(started !== null && started.type === 'event');
  assert.deepEqual(
    Object.keys(started.event).toSorted((a, b) => a.localeCompare(b)),
    ['context', 'data', 'kind', 'occurredAt', 'seq', 'source'],
  );
  assert.deepEqual(structuredClone(started), started);

  const context = { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 };
  const thinking = projectFrame({ type: 'delta', delta: { context, channel: 'thinking', offset: 0, data: '사고' } }, { source: 'SESSION' });
  assert.deepEqual(structuredClone(thinking), thinking);

  const base = ev('run.started', { message: 'm' }, 4);
  assert.throws(() => projectFrame({ ...base, event: { ...base.event, context: null } }, { source: 'SESSION' }), /has no context/);
});

test('SESSION: agent.created는 data를 통째로 벗기고, tool.requested/resolved는 도구별 data, invocation.started는 workflow target만 남긴다', () => {
  const created = projectFrame(
    ev('agent.created', {
      app: 'assistant',
      prompt: 'p'.repeat(10_000),
      tools: [
        { kind: 'external', name: 'confirm-review', inputSchema: {} },
        { kind: 'execute', name: 'run-review' },
      ],
    }),
    { source: 'SESSION' },
  );
  assert.deepEqual(created?.type === 'event' ? created.event.data : null, {});
  assert.equal(created?.type === 'event' ? created.event.source : null, 'SESSION');

  const confirm = projectFrame(
    ev('tool.requested', {
      tool: 'confirm-review',
      input: { documentId: 'D1', tier: 'high' },
      data: { documentId: 'D1', tier: 'high' },
      resultSchema: {},
    }),
    { source: 'SESSION' },
  );
  assert.deepEqual(confirm?.type === 'event' ? confirm.event.data : null, {
    tool: 'confirm-review',
    data: { documentId: 'D1', tier: 'high' },
  });

  const listing = projectFrame(ev('tool.requested', { tool: 'list-open-documents', input: {}, data: {}, resultSchema: {} }), {
    source: 'SESSION',
  });
  assert.deepEqual(listing?.type === 'event' ? listing.event.data : null, { tool: 'list-open-documents', data: {} });

  const asked = projectFrame(ev('tool.requested', { tool: 'ask-user', input: {}, data: { questions: [] }, resultSchema: {} }), {
    source: 'SESSION',
  });
  assert.deepEqual(asked?.type === 'event' ? asked.event.data : null, { tool: 'ask-user', data: { questions: [] } });

  const other = projectFrame(ev('tool.requested', { tool: 'write', input: {}, data: { path: 'x' }, resultSchema: {} }), {
    source: 'SESSION',
  });
  assert.deepEqual(other?.type === 'event' ? other.event.data : null, { tool: 'write' });

  const resolved = projectFrame(
    ev('tool.resolved', {
      tool: 'confirm-review',
      input: {},
      output: 'o',
      ok: true,
      data: {
        decision: 'confirmed',
        key: 'PRRR1',
        tier: 'high',
        document: { title: 't', subtitle: null, path: 'manuscript/PRRR1.md' },
        extra: 1,
      },
      duration: 1,
    }),
    { source: 'SESSION' },
  );
  assert.deepEqual(resolved?.type === 'event' ? resolved.event.data : null, {
    tool: 'confirm-review',
    ok: true,
    data: { decision: 'confirmed', key: 'PRRR1', tier: 'high', document: { title: 't', subtitle: null, path: 'manuscript/PRRR1.md' } },
  });

  const started = projectFrame(
    ev('invocation.started', {
      ordinal: 0,
      target: { kind: 'workflow', id: 'workflow_1', name: 'high', app: 'app_1', input: { path: 'manuscript/v1.txt' }, files: [] },
    }),
    { source: 'SESSION' },
  );
  assert.deepEqual(started?.type === 'event' ? started.event.data : null, {
    target: { kind: 'workflow', id: 'workflow_1', name: 'high', app: 'app_1' },
  });

  const agent = projectFrame(
    ev('invocation.started', {
      ordinal: 1,
      target: { kind: 'agent', id: 'agent_1', name: 'x', message: '사용자 원문', metadata: {} },
    }),
    { source: 'SESSION' },
  );
  assert.deepEqual(agent?.type === 'event' ? agent.event.data : null, { target: { kind: 'agent' } });

  const completed = projectFrame(ev('invocation.completed', { result: { big: true }, usage: null }), { source: 'SESSION' });
  assert.deepEqual(completed?.type === 'event' ? completed.event.data : null, {});
});

test('WORKFLOW: workflow.*·ask-user 요청/해소가 통과하고 sync·기타 도구·루트 전용 kind는 차단된다', () => {
  const done = projectFrame(ev('workflow.completed', { result: '{"kind":"issues"}', usage: null }), {
    source: 'WORKFLOW',
    workflowId: 'workflow_1',
  });
  assert.deepEqual(
    done?.type === 'event' ? { data: done.event.data, source: done.event.source, workflowId: done.event.workflowId } : null,
    {
      data: {},
      source: 'WORKFLOW',
      workflowId: 'workflow_1',
    },
  );

  const asked = projectFrame(
    ev('tool.requested', {
      tool: 'ask-user',
      input: { path: 'q.yaml' },
      data: { questions: [{ question: 'Q?', hint: 'h', multi: false, options: [{ label: 'a' }] }] },
      resultSchema: {},
    }),
    { source: 'WORKFLOW', workflowId: 'workflow_1' },
  );
  assert.deepEqual(asked?.type === 'event' ? asked.event.data : null, {
    tool: 'ask-user',
    data: { questions: [{ question: 'Q?', hint: 'h', multi: false, options: [{ label: 'a' }] }] },
  });

  const answered = projectFrame(
    ev('tool.resolved', {
      tool: 'ask-user',
      input: {},
      output: 'o',
      ok: true,
      data: { answers: [{ question: 'Q?', choice: ['a'] }] },
      duration: 1,
    }),
    { source: 'WORKFLOW', workflowId: 'workflow_1' },
  );
  assert.deepEqual(answered?.type === 'event' ? answered.event.data : null, {
    tool: 'ask-user',
    ok: true,
    data: { answers: [{ question: 'Q?', choice: ['a'] }] },
  });

  const declined = projectFrame(ev('tool.resolved', { tool: 'ask-user', input: {}, output: '', ok: false, data: null, duration: 1 }), {
    source: 'WORKFLOW',
    workflowId: 'workflow_1',
  });
  assert.deepEqual(declined?.type === 'event' ? declined.event.data : null, { tool: 'ask-user', ok: false });

  assert.equal(projectFrame({ type: 'sync', seq: 9 }, { source: 'WORKFLOW', workflowId: 'workflow_1' }), null);
  assert.equal(
    projectFrame(ev('tool.requested', { tool: 'write', input: {}, data: {}, resultSchema: {} }), {
      source: 'WORKFLOW',
      workflowId: 'workflow_1',
    }),
    null,
  );

  assert.equal(
    projectFrame(ev('run.started', { message: 'm', key: 'k', tag: null }), { source: 'WORKFLOW', workflowId: 'workflow_1' }),
    null,
  );
  assert.equal(projectFrame(ev('turn.started', {}), { source: 'WORKFLOW', workflowId: 'workflow_1' }), null);
});

test('WORKFLOW: step·turn.completed{text}·tool.executed{input}이 통과하고 델타는 workflowId를 싣는다', () => {
  const step = projectFrame(
    { type: 'event', event: { seq: 5, kind: 'step.started', occurredAt: 10, context: { step: 'description-0' }, data: {} } },
    { source: 'WORKFLOW', workflowId: 'w1' },
  );
  assert.deepEqual(step, {
    type: 'event',
    event: {
      seq: 5,
      occurredAt: 10,
      context: { step: 'description-0' },
      source: 'WORKFLOW',
      workflowId: 'w1',
      kind: 'step.started',
      data: {},
    },
  });

  const turn = projectFrame(
    ev('turn.completed', { text: '첫 구획을 읽었어요', thinking: 'x', toolCalls: [], stopReason: 'end', usage: null, raw: null }),
    { source: 'WORKFLOW', workflowId: 'w1' },
  );
  assert.deepEqual(turn?.type === 'event' ? turn.event.data : null, { text: '첫 구획을 읽었어요' });

  const executed = projectFrame(
    ev('tool.executed', {
      tool: 'read',
      input: { path: 'manuscript/a.md', offset: 1 },
      output: 'o'.repeat(9000),
      ok: true,
      data: null,
      duration: 1,
    }),
    { source: 'WORKFLOW', workflowId: 'w1' },
  );
  assert.deepEqual(executed?.type === 'event' ? executed.event.data : null, { tool: 'read', ok: true, input: { path: 'manuscript/a.md' } });

  const context = { agent: { id: 'c', name: 'description-medium' }, run: 1, turn: 2, attempt: 1 };
  const delta = projectFrame(
    { type: 'delta', delta: { context, channel: 'text', offset: 0, data: '구' } },
    { source: 'WORKFLOW', workflowId: 'w1' },
  );
  assert.deepEqual(delta, { type: 'delta', delta: { context, channel: 'text', offset: 0, data: '구', workflowId: 'w1' } });

  assert.equal(projectFrame({ type: 'sync', seq: 3 }, { source: 'WORKFLOW', workflowId: 'w1' }), null);
});

test('WORKFLOW: tool.requested/resolved는 ask-user만, 나머지 도구는 차단된다', () => {
  const asked = projectFrame(
    ev('tool.requested', {
      tool: 'ask-user',
      input: {},
      data: { questions: [{ question: 'Q', hint: '', multi: false, options: [{ label: 'a' }] }] },
      resultSchema: {},
    }),
    { source: 'WORKFLOW', workflowId: 'w1' },
  );
  assert.equal(asked?.type, 'event');
  assert.equal(
    projectFrame(ev('tool.requested', { tool: 'write', input: {}, data: {}, resultSchema: {} }), { source: 'WORKFLOW', workflowId: 'w1' }),
    null,
  );
});
