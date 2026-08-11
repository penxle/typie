import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  cancelWorkflow,
  fetchEventLog,
  getAgentAskCalls,
  getAgentPendingTool,
  getWorkflow,
  getWorkflowInvocations,
  hasPendingQuestion,
  newPrismWorkflowId,
  openEvents,
  PrismApiError,
  resolveAskUser,
  startWorkflow,
} from './prism.ts';

const env = { PRISM_API_ORIGIN: 'https://prism.test', PRISM_API_TOKEN: 'tk' };
const stub = (status: number, body: unknown) => vi.spyOn(globalThis, 'fetch').mockResolvedValue(Response.json(body, { status }));

afterEach(() => vi.restoreAllMocks());

describe('prism client', () => {
  it('startWorkflow는 계약 형태 그대로 보낸다', async () => {
    const spy = stub(200, {});
    await startWorkflow(env, {
      workflowId: 'ev-x',
      workflow: 'medium',
      input: { manuscriptPath: 'manuscript/v1.txt', meta: { title: '제목', subtitle: null } },
      files: [{ path: 'manuscript/v1.txt', content: '본문' }],
    });
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows');
    expect(init?.method).toBe('POST');
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk', 'content-type': 'application/json' });
    const body = JSON.parse(init?.body as string);
    // workflow는 호출자가 정한다 — 티어 이름이 곧 워크플로 이름이다
    expect(body).toEqual({
      workflowId: 'ev-x',
      app: 'feedback',
      workflow: 'medium',
      input: { manuscriptPath: 'manuscript/v1.txt', meta: { title: '제목', subtitle: null } },
      files: [{ path: 'manuscript/v1.txt', content: '본문' }],
    });
  });

  it('cancelWorkflow는 바디 없이 보낸다', async () => {
    const spy = stub(200, {});
    await cancelWorkflow(env, 'ev-x');
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows/ev-x/cancel');
    expect(init?.method).toBe('POST');
    expect(init?.body).toBeUndefined();
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk' });
  });

  it('openEvents는 스트림 응답을 그대로 통과시킨다', async () => {
    const upstream = new Response('data: {}\n\n', { status: 200, headers: { 'content-type': 'text/event-stream' } });
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(upstream);
    const res = await openEvents(env, 'ev-x', 7);
    expect(res).toBe(upstream);
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows/ev-x/events?lastEventId=7');
    expect(init?.method).toBeUndefined();
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk' });
  });

  it('getWorkflow는 result 문자열을 파싱하고 usage 객체를 그대로 통과시킨다', async () => {
    const result = {
      version: 1,
      issues: [],
      conclusion: { understanding: null, strengths: [], clearances: [], patterns: [], priorities: [] },
    };
    const fold = {
      provider: 'anthropic',
      agent: 'critic',
      model: 'm',
      effort: null,
      turns: 1,
      inputTokens: 1,
      outputTokens: 2,
      cacheReadTokens: 0,
      cacheWriteTokens: 0,
      thinkingTokens: null,
    };
    const usage = { settled: true, complete: true, folds: [fold] };
    stub(200, {
      workflow: {
        status: 'completed',
        result: JSON.stringify(result),
        usage,
        error: null,
        startedAt: 1,
        finishedAt: 2,
        // 와이어 잉여 열 — 명시 사영이 떨궈야 한다(caller·input이 화면 계층으로 새는 것의 방벽)
        caller: 'caller-x',
        input: '{"manuscriptPath":"m"}',
      },
    });
    const view = await getWorkflow(env, 'ev-x');
    expect(view.workflow).toEqual({ status: 'completed', result, usage, error: null, startedAt: 1, finishedAt: 2 });
  });

  it('getWorkflow는 실행 중 워크플로의 미확정 usage도 그대로 통과시킨다', async () => {
    stub(200, {
      workflow: { status: 'running', result: null, usage: { settled: false, folds: [] }, error: null, startedAt: 1, finishedAt: null },
    });
    const view = await getWorkflow(env, 'ev-x');
    expect(view.workflow.usage).toEqual({ settled: false, folds: [] });
  });

  it('getWorkflow는 running 워크플로의 result·usage null을 그대로 통과시킨다', async () => {
    stub(200, {
      workflow: { status: 'running', result: null, usage: null, error: null, startedAt: 1, finishedAt: null },
    });
    const view = await getWorkflow(env, 'ev-x');
    expect(view.workflow).toEqual({ status: 'running', result: null, usage: null, error: null, startedAt: 1, finishedAt: null });
  });

  it('fetchEventLog는 로그 경로를 친다', async () => {
    const spy = stub(200, { events: [] });
    await fetchEventLog(env, 'ev-x');
    expect(spy.mock.calls[0][0]).toBe('https://prism.test/workflows/ev-x/log');
  });

  it('오류 응답은 {error:code}를 PrismApiError로 판별한다', async () => {
    const spy = stub(403, { error: 'forbidden' });
    const caught = await getWorkflow(env, 'ev-x').catch((err: unknown) => err);
    expect(caught).toBeInstanceOf(PrismApiError);
    expect((caught as PrismApiError).code).toBe('forbidden');
    expect((caught as PrismApiError).status).toBe(403);
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows/ev-x');
    expect(init?.method).toBeUndefined();
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk' });
  });

  it('비JSON 오류 본문은 코드 internal로 강등한다', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('<html>', { status: 500 }));
    const caught = await cancelWorkflow(env, 'ev-x').catch((err: unknown) => err);
    expect((caught as PrismApiError).code).toBe('internal');
  });

  it('newPrismWorkflowId는 표면 형식을 지킨다', () => {
    expect(newPrismWorkflowId()).toMatch(/^ev-[a-z0-9-]{36}$/);
    expect(newPrismWorkflowId()).not.toBe(newPrismWorkflowId());
  });
});

describe('ask-user 표면', () => {
  it('resolveAskUser가 result 봉투로 해소를 제출한다', async () => {
    const spy = stub(200, {});
    await resolveAskUser(env, 'agent_a', 'call_1', [{ question: 'q', choice: ['a'] }]);
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/agents/agent_a/tools/call_1/result');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(init?.body as string)).toEqual({ result: { answers: [{ question: 'q', choice: ['a'] }] } });
  });

  it('resolveAskUser는 폼에서 온 세그먼트를 인코딩해 경로 순회를 막는다', async () => {
    const spy = stub(200, {});
    await resolveAskUser(env, 'agent_a', '../../victim/tools/x/result?', [{ question: 'q', choice: ['a'] }]);
    // fetch는 URL 파서를 거친다 — 파서가 dot-segment를 정규화한 뒤에도 경로가 이 에이전트 아래 머물러야 한다.
    const url = new URL(String(spy.mock.calls[0][0]));
    expect(url.pathname).toBe('/agents/agent_a/tools/..%2F..%2Fvictim%2Ftools%2Fx%2Fresult%3F/result');
    expect(url.search).toBe('');
  });

  it('no-pending-tool 409가 PrismApiError로 던져진다', async () => {
    stub(409, { error: 'no-pending-tool' });
    const caught = await resolveAskUser(env, 'agent_a', 'call_1', [{ question: 'q', choice: ['a'] }]).catch((err: unknown) => err);
    expect(caught).toBeInstanceOf(PrismApiError);
    expect(caught).toMatchObject({ code: 'no-pending-tool', status: 409 });
  });

  it('getWorkflowInvocations가 invocations만 명시 사영한다', async () => {
    const spy = stub(200, {
      workflow: {},
      steps: [],
      invocations: [{ invocationId: 'i1', agentId: 'agent_a', agentName: 'plan', status: 'running', descriptorHash: 'x' }],
    });
    expect(await getWorkflowInvocations(env, 'ev-x')).toEqual([{ agentId: 'agent_a', agentName: 'plan', status: 'running' }]);
    expect(spy.mock.calls[0][0]).toBe('https://prism.test/workflows/ev-x');
  });

  it('getAgentPendingTool이 pending을 좁혀 돌려준다', async () => {
    const spy = stub(200, { agent: {}, runs: [], pending: { toolCallId: 'call_1', tool: 'ask-user', input: {} } });
    expect(await getAgentPendingTool(env, 'agent_a')).toEqual({ toolCallId: 'call_1', tool: 'ask-user' });
    expect(spy.mock.calls[0][0]).toBe('https://prism.test/agents/agent_a');
  });

  it('getAgentPendingTool은 대기가 없으면 null이다', async () => {
    stub(200, { agent: {}, runs: [], pending: null });
    expect(await getAgentPendingTool(env, 'agent_a')).toBeNull();
  });

  it('getAgentAskCalls가 ask-user 성공 해소의 answers만 시간순으로 모은다', async () => {
    const spy = stub(200, {
      calls: [
        { tool: 'read', input: { path: 'a' }, data: null },
        { tool: 'ask-user', input: { questions: [] }, data: { answers: [{ question: 'q1', choice: ['a'] }] } },
        // 오류 문면 커밋 — 해소가 아니라 대기의 실패라 answers가 없다
        { tool: 'ask-user', input: { questions: [] }, data: null },
        { tool: 'ask-user', input: { questions: [] }, data: { answers: [{ question: 'q2', choice: ['b', '직접 쓴 답'] }] } },
      ],
    });
    expect(await getAgentAskCalls(env, 'agent_a')).toEqual([
      [{ question: 'q1', choice: ['a'] }],
      [{ question: 'q2', choice: ['b', '직접 쓴 답'] }],
    ]);
    expect(spy.mock.calls[0][0]).toBe('https://prism.test/agents/agent_a/calls');
  });
});

describe('hasPendingQuestion', () => {
  // 2홉이 서로 다른 경로를 친다 — 단일 응답 stub 대신 경로로 갈라 답한다.
  const stubHops = (pending: unknown, status = 'running') =>
    vi.spyOn(globalThis, 'fetch').mockImplementation((input) =>
      Promise.resolve(
        String(input).includes('/agents/')
          ? Response.json({ agent: {}, runs: [], pending })
          : Response.json({
              workflow: {},
              steps: [],
              invocations: [{ invocationId: 'i1', agentId: 'agent_a', agentName: 'plan', status }],
            }),
      ),
    );

  it('running invocation의 agent가 ask-user를 기다리면 true다', async () => {
    stubHops({ toolCallId: 'call_1', tool: 'ask-user', input: {} });
    expect(await hasPendingQuestion(env, 'ev-x')).toBe(true);
  });

  it('대기가 없으면 false다', async () => {
    stubHops(null);
    expect(await hasPendingQuestion(env, 'ev-x')).toBe(false);
  });

  it('타 도구 대기는 질문이 아니다', async () => {
    stubHops({ toolCallId: 'call_1', tool: 'read', input: {} });
    expect(await hasPendingQuestion(env, 'ev-x')).toBe(false);
  });

  it('종결한 invocation의 agent는 조회하지 않는다', async () => {
    const spy = stubHops({ toolCallId: 'call_1', tool: 'ask-user', input: {} }, 'completed');
    expect(await hasPendingQuestion(env, 'ev-x')).toBe(false);
    expect(spy.mock.calls.every(([url]) => !String(url).includes('/agents/'))).toBe(true);
  });
});
