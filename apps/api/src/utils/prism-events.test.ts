import assert from 'node:assert/strict';
import test from 'node:test';
import { projectFrame } from './prism-events.ts';

const ev = (kind: string, data: Record<string, unknown>, seq = 1) => ({
  type: 'event' as const,
  event: { seq, kind, occurredAt: 10, loggedAt: 11, context: { agent: { id: 'a', name: 'x' }, run: 1 }, data },
});

test('heartbeat는 차단, sync는 프레임 그대로, 델타는 text만 원문, thinking은 글자 수, tool.input은 도구 정보만 나간다', () => {
  assert.equal(projectFrame({ type: 'heartbeat' }), null);
  assert.deepEqual(projectFrame({ type: 'sync', seq: 9 }), { type: 'sync', seq: 9 });

  const context = { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 };
  const text = projectFrame({ type: 'delta', delta: { context, channel: 'text', offset: 2, data: '녕' } });
  assert.deepEqual(text, { type: 'delta', delta: { context, channel: 'text', offset: 2, data: '녕' } });

  const thinking = projectFrame({ type: 'delta', delta: { context, channel: 'thinking', offset: 10, data: '내부 사고 원문' } });
  assert.deepEqual(thinking, { type: 'delta', delta: { context, channel: 'thinking', chars: 18 } });

  const toolInput = projectFrame({
    type: 'delta',
    delta: { context, channel: 'tool.input', offset: 3, data: '{"a', tool: { id: null, name: 'read' } },
  });
  assert.deepEqual(toolInput, { type: 'delta', delta: { context, channel: 'tool.input', tool: { id: null, name: 'read' } } });
});

test('허용 kind는 스키마 strip으로 소비 필드만 남고, 미허용 kind는 차단된다', () => {
  const started = projectFrame(ev('run.started', { message: 'm', key: 'k', tag: null }));
  assert.equal(started?.type, 'event');
  assert.deepEqual(started.event.data, { message: 'm' });

  const turn = projectFrame(
    ev('turn.completed', { text: 't', thinking: '생각', toolCalls: [], stopReason: 'end', usage: null, raw: { big: true } }),
  );
  assert.deepEqual(turn?.type === 'event' ? turn.event.data : null, { text: 't', toolCalls: [] });

  const failed = projectFrame(ev('run.failed', { reason: 'vendor detail', usage: null }));
  assert.deepEqual(failed?.type === 'event' ? failed.event.data : null, {});

  const executed = projectFrame(
    ev('tool.executed', { tool: 'read', input: { path: 'x'.repeat(100_000) }, output: 'o', ok: true, data: null, duration: 1 }),
  );
  assert.deepEqual(executed?.type === 'event' ? executed.event.data : null, { tool: 'read', ok: true });

  assert.equal(projectFrame(ev('agent.created', { app: 'typie', prompt: 'p' })), null);
  assert.equal(projectFrame(ev('run.reentered', { cause: 'watchdog', count: 1, interval: null })), null);
});

test('허용 kind의 형태가 선언과 어긋나면 삼키지 않고 던진다', () => {
  assert.throws(() => projectFrame(ev('turn.completed', { thinking: 1 })));
  assert.throws(() => projectFrame(ev('tool.executed', { tool: 'read' })));
});

test('사영 프레임은 JSON 왕복(GraphQL 스칼라)에서 구조가 보존되고, 이벤트 봉투는 {seq, occurredAt, context}뿐이다', () => {
  const started = projectFrame(ev('run.started', { message: '안녕', key: 'k', tag: null }, 3));
  assert.ok(started !== null && started.type === 'event');
  assert.deepEqual(
    Object.keys(started.event).toSorted((a, b) => a.localeCompare(b)),
    ['context', 'data', 'kind', 'occurredAt', 'seq'],
  );
  assert.deepEqual(structuredClone(started), started);

  const context = { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 };
  const thinking = projectFrame({ type: 'delta', delta: { context, channel: 'thinking', offset: 0, data: '사고' } });
  assert.deepEqual(structuredClone(thinking), thinking);

  const base = ev('run.started', { message: 'm' }, 4);
  assert.throws(() => projectFrame({ ...base, event: { ...base.event, context: null } }), /has no context/);
});
