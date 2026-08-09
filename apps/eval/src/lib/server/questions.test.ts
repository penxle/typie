import { afterEach, describe, expect, it, vi } from 'vitest';
import { collectAskAnswers } from './questions.ts';
import type { SseEvent } from '../feedback/sse.ts';

const env = { PRISM_API_ORIGIN: 'https://prism.test', PRISM_API_TOKEN: 'tk' };

const wrap = (id: number, kind: string, data: Record<string, unknown>): SseEvent => ({
  id,
  event: kind,
  data: JSON.stringify({ seq: id, kind, data, createdAt: 1000 + id }),
});

const askInput = (question: string) => JSON.stringify({ questions: [{ question, hint: 'h', multi: false, options: [{ label: '가' }] }] });

const requested = (id: number, agentId: string, toolCallId: string, question: string) =>
  wrap(id, 'tool.requested', {
    agent: { id: agentId, name: 'plan' },
    turn: 1,
    attempt: 1,
    tool: 'ask-user',
    toolCallId,
    input: askInput(question),
  });

const called = (id: number, agentId: string) =>
  wrap(id, 'tool.called', { agent: { id: agentId, name: 'plan' }, turn: 1, attempt: 1, tool: 'ask-user', input: {}, ok: true });

// 원장 응답은 호출마다 새 Response로 준다 — 같은 객체를 재사용하면 두 번째 agent의 body가 이미 소비돼 있다.
const ledgers = (byAgent: Record<string, unknown[][]>) =>
  vi.spyOn(globalThis, 'fetch').mockImplementation((input) => {
    const agentId = /\/agents\/([^/]+)\/calls$/.exec(String(input))?.[1] ?? '';
    const answers = byAgent[agentId] ?? [];
    return Promise.resolve(Response.json({ calls: answers.map((a) => ({ tool: 'ask-user', input: {}, data: { answers: a } })) }));
  });

afterEach(() => vi.restoreAllMocks());

describe('collectAskAnswers', () => {
  it('answered 엔트리를 agent별 원장과 순서로 짝지어 toolCallId로 색인한다', async () => {
    const spy = ledgers({ agent_a: [[{ question: 'Q?', choice: ['가'] }]] });
    const events = [wrap(1, 'step.started', { step: 'plan-0' }), requested(2, 'agent_a', 'call_1', 'Q?'), called(3, 'agent_a')];

    expect(await collectAskAnswers(env, events)).toEqual({ call_1: [{ question: 'Q?', choice: ['가'] }] });
    expect(spy.mock.calls[0][0]).toBe('https://prism.test/agents/agent_a/calls');
  });

  it('answered가 없으면 prism을 부르지 않는다', async () => {
    const spy = vi.spyOn(globalThis, 'fetch');
    const events = [wrap(1, 'step.started', { step: 'plan-0' }), requested(2, 'agent_a', 'call_1', 'Q?')];

    expect(await collectAskAnswers(env, events)).toEqual({});
    expect(spy).not.toHaveBeenCalled();
  });

  it('agent가 여럿이면 원장을 각각 조회해 자기 agent의 순번으로 짝짓는다', async () => {
    const spy = ledgers({
      agent_a: [[{ question: 'A1?', choice: ['가'] }], [{ question: 'A2?', choice: ['나'] }]],
      agent_b: [[{ question: 'B1?', choice: ['다'] }]],
    });
    const events = [
      wrap(1, 'step.started', { step: 'research' }),
      requested(2, 'agent_a', 'call_1', 'A1?'),
      called(3, 'agent_a'),
      wrap(4, 'step.started', { step: 'plan' }),
      requested(5, 'agent_b', 'call_2', 'B1?'),
      called(6, 'agent_b'),
      requested(7, 'agent_a', 'call_3', 'A2?'),
      called(8, 'agent_a'),
    ];

    expect(await collectAskAnswers(env, events)).toEqual({
      call_1: [{ question: 'A1?', choice: ['가'] }],
      call_2: [{ question: 'B1?', choice: ['다'] }],
      call_3: [{ question: 'A2?', choice: ['나'] }],
    });
    expect(spy.mock.calls.map((call) => String(call[0])).toSorted((a, b) => a.localeCompare(b))).toEqual([
      'https://prism.test/agents/agent_a/calls',
      'https://prism.test/agents/agent_b/calls',
    ]);
  });

  it('아직 답하지 않은 엔트리는 색인에 넣지 않는다', async () => {
    ledgers({ agent_a: [[{ question: 'A1?', choice: ['가'] }]] });
    const events = [
      wrap(1, 'step.started', { step: 'plan-0' }),
      requested(2, 'agent_a', 'call_1', 'A1?'),
      called(3, 'agent_a'),
      requested(4, 'agent_a', 'call_2', 'A2?'),
    ];

    expect(await collectAskAnswers(env, events)).toEqual({ call_1: [{ question: 'A1?', choice: ['가'] }] });
  });

  it('원장이 엔트리보다 짧으면 짝 없는 엔트리를 건너뛴다', async () => {
    ledgers({ agent_a: [[{ question: 'A1?', choice: ['가'] }]] });
    const events = [
      wrap(1, 'step.started', { step: 'plan-0' }),
      requested(2, 'agent_a', 'call_1', 'A1?'),
      called(3, 'agent_a'),
      requested(4, 'agent_a', 'call_2', 'A2?'),
      called(5, 'agent_a'),
    ];

    expect(await collectAskAnswers(env, events)).toEqual({ call_1: [{ question: 'A1?', choice: ['가'] }] });
  });

  it('원장 조회가 실패하면 던진다 — 호출부가 관용 처분한다', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(Response.json({ error: 'not-found' }, { status: 404 }));
    const events = [wrap(1, 'step.started', { step: 'plan-0' }), requested(2, 'agent_a', 'call_1', 'Q?'), called(3, 'agent_a')];

    await expect(collectAskAnswers(env, events)).rejects.toThrow('prism-api 404');
  });
});
