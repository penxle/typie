import { describe, expect, it } from 'vitest';
import { parked } from './parked.ts';
import type { ParkedEvent } from './parked.ts';

const root = { id: 'chat-1', name: 'chat' };
const child = { id: 'agent_9', name: 'judgment' };

const ev = (kind: string, context: ParkedEvent['context'], data: Record<string, unknown> = {}): ParkedEvent => ({ kind, context, data });
const run = (agent: { id: string; name: string }, n = 1) => ({ agent, run: n });
const tool = (agent: { id: string; name: string }, toolCallId: string, n = 1) => ({ agent, run: n, turn: 1, attempt: 1, toolCallId });

describe('parked — agent', () => {
  it('열린 run이 없으면 파킹이 아니다', () => {
    expect(parked([], 'agent')).toBe(false);
    expect(parked([ev('run.started', run(root)), ev('run.completed', run(root))], 'agent')).toBe(false);
  });

  it('user 해소 도구 요청이 미해소면 파킹, 해소되면 해제', () => {
    const events = [ev('run.started', run(root)), ev('tool.requested', tool(root, 'c1'), { tool: 'ask-user' })];
    expect(parked(events, 'agent')).toBe(true);
    expect(parked([...events, ev('tool.resolved', tool(root, 'c1'), { tool: 'ask-user' })], 'agent')).toBe(false);
  });

  it('client 해소 도구 요청은 파킹이 아니고, 미등재 도구는 파킹이다', () => {
    const base = [ev('run.started', run(root))];
    expect(parked([...base, ev('tool.requested', tool(root, 'c1'), { tool: 'list-open-documents' })], 'agent')).toBe(false);
    expect(parked([...base, ev('tool.requested', tool(root, 'c2'), { tool: 'unknown-thing' })], 'agent')).toBe(true);
  });

  it('미종결 workflow 구동은 파킹, 구동 종결로 해제', () => {
    const inv = { ...tool(root, 'c1'), invocation: 'inv_1' };
    const events = [ev('run.started', run(root)), ev('invocation.started', inv, { target: { kind: 'workflow', id: 'wf_1' } })];
    expect(parked(events, 'agent')).toBe(true);
    expect(parked([...events, ev('invocation.completed', inv)], 'agent')).toBe(false);
    expect(parked([...events, ev('invocation.failed', inv)], 'agent')).toBe(false);
  });

  it('agent 대상 구동은 파킹 사유가 아니다', () => {
    const inv = { ...tool(root, 'c1'), invocation: 'inv_2' };
    expect(parked([ev('run.started', run(root)), ev('invocation.started', inv, { target: { kind: 'agent' } })], 'agent')).toBe(false);
  });

  it('run이 종결되면 그 run의 미해소 요청·구동은 무시된다', () => {
    const events = [
      ev('run.started', run(root)),
      ev('tool.requested', tool(root, 'c1'), { tool: 'ask-user' }),
      ev('run.failed', run(root)),
      ev('run.started', run(root, 2)),
    ];
    expect(parked(events, 'agent')).toBe(false);
  });

  it('context가 null인 구세대 행은 건너뛴다', () => {
    expect(parked([ev('run.started', null), ev('tool.requested', null, { tool: 'ask-user' })], 'agent')).toBe(false);
  });
});

describe('parked — workflow', () => {
  const other = { id: 'agent_10', name: 'stylistic' };

  it('열린 자식 run이 없으면 파킹이 아니다', () => {
    expect(parked([ev('workflow.started', {})], 'workflow')).toBe(false);
  });

  it('열린 자식 run 전부가 user 요청 대기일 때만 파킹', () => {
    const both = [ev('run.started', run(child)), ev('run.started', run(other))];
    const one = [...both, ev('tool.requested', tool(child, 'c1'), { tool: 'ask-user' })];
    expect(parked(one, 'workflow')).toBe(false);
    expect(parked([...one, ev('tool.requested', tool(other, 'c2'), { tool: 'ask-user' })], 'workflow')).toBe(true);
    expect(parked([...one, ev('run.completed', run(other))], 'workflow')).toBe(true);
  });

  it('workflow 스코프에서는 invocation을 보지 않는다', () => {
    const events = [
      ev('run.started', run(child)),
      ev('invocation.started', { step: 's', invocation: 'inv_3' }, { target: { kind: 'workflow' } }),
    ];
    expect(parked(events, 'workflow')).toBe(false);
  });
});
