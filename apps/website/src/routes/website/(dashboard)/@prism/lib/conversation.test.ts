import { describe, expect, it } from 'vitest';
import { applyFrame, emptyTranscript } from './conversation.ts';
import type { ProjectedEventData, ProjectedStreamFrame } from '@typie/prism';

const agent = { id: 'typie-1', name: 'assistant' };
const ev = (seq: number, kind: string, context: Record<string, unknown>, data: Record<string, unknown>): ProjectedStreamFrame => ({
  type: 'event',
  event: {
    seq,
    occurredAt: 1000 + seq,
    context: context as never,
    ...({ kind, data } as ProjectedEventData),
  },
});
const run = { agent, run: 1 };
const turn = { ...run, turn: 1, attempt: 1 };
const tool = { ...turn, toolCallId: 'c1' };

const reduce = (frames: ProjectedStreamFrame[]) => frames.reduce(applyFrame, emptyTranscript());

describe('applyFrame', () => {
  it('run.started는 사용자 메시지 + running, 종결은 상태·live 봉인', () => {
    let t = reduce([ev(1, 'run.started', run, { message: '안녕' })]);
    expect(t.messages[0]).toMatchObject({ role: 'user', text: '안녕' });
    expect(t).toMatchObject({ run: 'running', cursor: 1 });
    t = applyFrame(t, ev(2, 'run.failed', run, {}));
    expect(t).toMatchObject({ run: 'failed', live: null, cursor: 2 });
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

  it('텍스트도 toolCalls도 없는 turn.completed는 메시지를 만들지 않는다', () => {
    const t = reduce([ev(1, 'run.started', run, { message: 'a' }), ev(2, 'turn.completed', turn, { text: null, toolCalls: [] })]);
    expect(t.messages).toHaveLength(1);
  });

  it('tool.executed/rejected/requested/resolved는 phase·ok를 가진 tool 메시지', () => {
    const t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      ev(2, 'tool.executed', tool, { tool: 'read', ok: true }),
      ev(3, 'tool.rejected', { ...tool, toolCallId: 'c2' }, { tool: 'zzz' }),
      ev(4, 'tool.requested', { ...tool, toolCallId: 'c3' }, { tool: 'ask-user' }),
      ev(5, 'tool.resolved', { ...tool, toolCallId: 'c3' }, { tool: 'ask-user', ok: true }),
    ]);
    expect(t.messages.slice(1)).toMatchObject([
      { role: 'tool', name: 'read', phase: 'executed', ok: true },
      { role: 'tool', name: 'zzz', phase: 'rejected', ok: false },
      { role: 'tool', name: 'ask-user', phase: 'requested', ok: null },
      { role: 'tool', name: 'ask-user', phase: 'resolved', ok: true },
    ]);
  });

  it('turn.retried는 retrying 표지, turn.started가 지운다', () => {
    let t = reduce([ev(1, 'run.started', run, { message: 'a' }), ev(2, 'turn.retried', turn, {})]);
    expect(t.retrying).toBe(true);
    t = applyFrame(t, ev(3, 'turn.started', { ...turn, attempt: 2 }, {}));
    expect(t.retrying).toBe(false);
  });

  it('사영 방언(소비 필드만)의 data가 assistant 메시지와 도구 이름을 만든다', () => {
    const t = reduce([
      ev(1, 'run.started', run, { message: 'a' }),
      ev(2, 'turn.completed', turn, { text: '확정', toolCalls: [] }),
      ev(3, 'tool.requested', tool, { tool: 'ask-user' }),
    ]);
    expect(t.messages.slice(1)).toMatchObject([
      { role: 'assistant', text: '확정' },
      { role: 'tool', name: 'ask-user', phase: 'requested' },
    ]);
  });

  it('run.completed는 idle, run.canceled는 canceled', () => {
    const base = [ev(1, 'run.started', run, { message: 'a' })];
    expect(reduce([...base, ev(2, 'run.completed', run, {})])).toMatchObject({ run: 'idle' });
    expect(reduce([...base, ev(2, 'run.canceled', run, {})])).toMatchObject({ run: 'canceled' });
  });

  it('sync는 커서 high-water를 올리고, 지난 seq는 무시한다', () => {
    let t = reduce([ev(5, 'run.started', run, { message: 'a' })]);
    t = applyFrame(t, ev(4, 'run.completed', run, {}));
    expect(t.run).toBe('running');
    t = applyFrame(t, { type: 'sync', seq: 6 });
    expect(t).toMatchObject({ cursor: 6, messages: [{ role: 'user' }] });
  });
});
