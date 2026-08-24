import assert from 'node:assert/strict';
import test from 'node:test';
import { materialize } from './prism-transcript.ts';
import type { StoredEvent } from './prism-transcript.ts';

const agent = { id: 'chat-1', name: 'chat' };
const child = { id: 'agent_9', name: 'judgment' };
const ev = (seq: number, kind: string, context: StoredEvent['context'], data: Record<string, unknown> = {}): StoredEvent => ({
  seq,
  kind,
  occurredAt: 1000 + seq,
  loggedAt: 1000 + seq,
  context,
  data,
});
const run = { agent, run: 1 };
const turn = { ...run, turn: 1, attempt: 1 };
const inv = { ...turn, toolCallId: 'c1', invocation: 'inv_1' };

test('워크플로 이벤트는 invocation.started 직후에 끼워 넣어 같은 run 안에 묶인다', () => {
  const session = [
    ev(1, 'run.started', run, { message: 'a', command: null }),
    ev(2, 'invocation.started', inv, { target: { kind: 'workflow', id: 'wf_1', name: 'high', app: 'feedback', ref: null } }),
    ev(3, 'invocation.completed', inv, {}),
    ev(4, 'run.completed', run, { result: null, usage: null }),
    ev(5, 'run.started', { agent, run: 2 }, { message: 'b', command: null }),
  ];
  const workflow = [
    ev(1, 'workflow.started', {}, {}),
    ev(
      2,
      'tool.requested',
      { agent: child, run: 1, turn: 1, attempt: 1, toolCallId: 'k1' },
      { tool: 'ask-user', data: { questions: [] }, input: {}, resultSchema: {} },
    ),
    ev(3, 'workflow.completed', {}, { result: null, usage: null }),
  ];
  const t = materialize(session, new Map([['wf_1', workflow]]));
  assert.deepEqual(
    t.messages.map((m) => m.role),
    ['user', 'workflow', 'tool-request', 'user'],
  );
  assert.equal(t.messages[1].role === 'workflow' ? t.messages[1].status : null, 'completed');
  assert.equal(t.messages[2].role === 'tool-request' ? t.messages[2].status : null, 'closed');
  assert.equal(t.cursor, 5);
});

test('미등재 kind와 구세대 행은 건너뛴다', () => {
  const t = materialize(
    [
      ev(1, 'run.started', run, { message: 'a', command: null }),
      ev(2, 'agent.reconfigured', { agent }, { prompt: 'p' }),
      ev(3, 'tool.called', null, {}),
    ],
    new Map(),
  );
  assert.equal(t.messages.length, 1);
  assert.equal(t.cursor, 1);
});

test('링크되지 않은 워크플로는 이벤트 없이 running으로 남는다', () => {
  const t = materialize(
    [
      ev(1, 'run.started', run, { message: 'a', command: null }),
      ev(2, 'invocation.started', inv, { target: { kind: 'workflow', id: 'wf_2', name: 'high', app: 'feedback' } }),
    ],
    new Map(),
  );
  assert.equal(t.messages[1].role === 'workflow' ? t.messages[1].status : null, 'running');
});
