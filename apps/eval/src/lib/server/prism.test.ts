import { afterEach, describe, expect, it, vi } from 'vitest';
import { cancelRun, getSession, newPrismSessionId, openEvents, PrismApiError, startWorkflow } from './prism.ts';

const env = { PRISM_API_ORIGIN: 'https://prism.test', PRISM_API_TOKEN: 'tk' };
const stub = (status: number, body: unknown) => vi.spyOn(globalThis, 'fetch').mockResolvedValue(Response.json(body, { status }));

afterEach(() => vi.restoreAllMocks());

describe('prism client', () => {
  it('startWorkflow는 계약 형태 그대로 보낸다', async () => {
    const spy = stub(200, {});
    await startWorkflow(env, {
      sessionId: 'ev-x',
      input: { manuscriptPath: 'manuscript/v1.txt' },
      files: [{ path: 'manuscript/v1.txt', content: '본문' }],
    });
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows');
    expect(init?.method).toBe('POST');
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk', 'content-type': 'application/json' });
    const body = JSON.parse(init?.body as string);
    expect(body).toEqual({
      sessionId: 'ev-x',
      app: 'feedback',
      workflow: 'main',
      input: { manuscriptPath: 'manuscript/v1.txt' },
      files: [{ path: 'manuscript/v1.txt', content: '본문' }],
    });
  });

  it('cancelRun은 계약 형태 그대로 보낸다', async () => {
    const spy = stub(200, {});
    await cancelRun(env, 'ev-x');
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/sessions/ev-x/cancel');
    expect(init?.method).toBe('POST');
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk', 'content-type': 'application/json' });
    expect(JSON.parse(init?.body as string)).toEqual({ runSeq: 1 });
  });

  it('openEvents는 스트림 응답을 그대로 통과시킨다', async () => {
    const upstream = new Response('data: {}\n\n', { status: 200, headers: { 'content-type': 'text/event-stream' } });
    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(upstream);
    const res = await openEvents(env, 'ev-x', 7);
    expect(res).toBe(upstream);
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/sessions/ev-x/events?lastEventId=7');
    expect(init?.method).toBeUndefined();
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk' });
  });

  it('getSession은 와이어의 result·usage 문자열을 객체로 복원한다', async () => {
    const result = {
      version: 1,
      issues: [],
      conclusion: { understanding: null, strengths: [], clearances: [], patterns: [], priorities: [] },
    };
    const usage = { complete: true, folds: [] };
    stub(200, {
      session: { id: 'ev-x' },
      runs: [
        {
          runSeq: 1,
          status: 'completed',
          result: JSON.stringify(result),
          usage: JSON.stringify(usage),
          error: null,
          startedAt: 1,
          finishedAt: 2,
        },
      ],
    });
    const view = await getSession(env, 'ev-x');
    expect(view.runs[0]).toMatchObject({ result, usage });
  });

  it('getSession은 running run의 result·usage null을 그대로 통과시킨다', async () => {
    stub(200, {
      session: { id: 'ev-x' },
      runs: [{ runSeq: 1, status: 'running', result: null, usage: null, error: null, startedAt: 1, finishedAt: null }],
    });
    const view = await getSession(env, 'ev-x');
    expect(view.runs[0]).toMatchObject({ result: null, usage: null });
  });

  it('오류 응답은 {error:code}를 PrismApiError로 판별한다', async () => {
    const spy = stub(403, { error: 'forbidden' });
    const caught = await getSession(env, 'ev-x').catch((err: unknown) => err);
    expect(caught).toBeInstanceOf(PrismApiError);
    expect((caught as PrismApiError).code).toBe('forbidden');
    expect((caught as PrismApiError).status).toBe(403);
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/sessions/ev-x');
    expect(init?.method).toBeUndefined();
    expect(init?.headers).toMatchObject({ authorization: 'Bearer tk' });
  });

  it('비JSON 오류 본문은 코드 internal로 강등한다', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('<html>', { status: 500 }));
    const caught = await cancelRun(env, 'ev-x').catch((err: unknown) => err);
    expect((caught as PrismApiError).code).toBe('internal');
  });

  it('newPrismSessionId는 표면 형식을 지킨다', () => {
    expect(newPrismSessionId()).toMatch(/^ev-[a-z0-9-]{36}$/);
    expect(newPrismSessionId()).not.toBe(newPrismSessionId());
  });
});
