import { afterEach, describe, expect, it, vi } from 'vitest';
import { FeedbackSessions, ManuscriptVersions, Reviews } from './db/index.ts';
import { buildStartRows, requestCancel, startFeedbackSession } from './reviews.ts';
import type { Db } from './db/index.ts';

type Env = Parameters<typeof startFeedbackSession>[1];

const env = {
  INTERNAL_API_BASE: 'https://api.test',
  INTERNAL_API_KEY: 'ik',
  PRISM_API_ORIGIN: 'https://prism.test',
  PRISM_API_TOKEN: 'tk',
} as unknown as Env;

// db는 호출 기록용 스텁이다. batch는 넘어온 문장 배열을 그대로 붙잡아 3행이 한 묶음인지 보게 한다.
const createDbStub = () => {
  const batches: unknown[][] = [];
  const inserts: { table: unknown; row: Record<string, unknown> }[] = [];
  const updates: { table: unknown; values: Record<string, unknown> }[] = [];
  const db = {
    insert: (table: unknown) => ({
      values: (row: Record<string, unknown>) => {
        inserts.push({ table, row });
        return { table, row };
      },
    }),
    batch: (statements: unknown[]) => {
      batches.push(statements);
      return Promise.resolve([]);
    },
    update: (table: unknown) => ({
      set: (values: Record<string, unknown>) => ({
        where: () => {
          updates.push({ table, values });
          return Promise.resolve([]);
        },
      }),
    }),
  };
  return { db: db as unknown as Db, batches, inserts, updates };
};

const extractOk = () => Promise.resolve(Response.json({ results: [{ documentId: 'D0TEST01', prose: '본문', title: '제목' }] }));

const route = (handlers: { extract?: () => Promise<Response>; workflows?: () => Promise<Response> }) =>
  vi.spyOn(globalThis, 'fetch').mockImplementation((input) => {
    const url = String(input);
    if (url.endsWith('/internal/corpus/extract')) return (handlers.extract ?? extractOk)();
    if (url.endsWith('/workflows')) return (handlers.workflows ?? (() => Promise.resolve(Response.json({}))))();
    if (url.endsWith('/cancel')) return Promise.resolve(Response.json({}));
    throw new Error(`unexpected fetch: ${url}`);
  });

afterEach(() => vi.restoreAllMocks());

describe('buildStartRows', () => {
  it('세션·버전1·리뷰1(running)의 정합 행을 만든다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
    });
    expect(rows.session.refId).toBe('D0TEST01');
    expect(rows.version).toMatchObject({ sessionId: rows.session.id, version: 1, content: '본문', charCount: 2 });
    expect(rows.review).toMatchObject({
      sessionId: rows.session.id,
      round: 1,
      prismWorkflowId: 'ev-x',
      status: 'running',
      manuscriptVersion: 1,
    });
  });

  it('charCount는 grapheme 기준이라 서로게이트 쌍에서 UTF-16 code unit과 갈라진다', () => {
    const content = 'a👍b';
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: null,
      content,
      prismWorkflowId: 'ev-x',
      now: new Date(0),
    });
    expect(rows.version.charCount).toBe(3);
    expect(content).toHaveLength(4);
  });

  it('ZWSP는 정본과 같이 세기 전에만 제거하고 본문은 원본 그대로 보존한다', () => {
    const content = 'a\u{200B}b';
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: null,
      content,
      prismWorkflowId: 'ev-x',
      now: new Date(0),
    });
    expect(rows.version.charCount).toBe(2);
    expect(rows.version.content).toBe(content);
  });

  it('buildStartRows가 modelConfig 스냅샷을 싣는다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      overrides: { 'review-high': { model: 'claude-sonnet-5', effort: 'xhigh' } },
    });
    expect(rows.review.modelConfig['review-high']).toEqual({ model: 'claude-sonnet-5', effort: 'xhigh', overridden: true });
    expect(rows.review.modelConfig['research-high']).toEqual({ model: 'claude-opus-5', effort: 'xhigh', overridden: false });
  });

  it('티어 무지정은 high 행으로 굳는다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
    });
    expect(rows.review.tier).toBe('high');
    expect(Object.keys(rows.review.modelConfig)).toHaveLength(7);
  });

  it('티어는 행과 스냅샷에 함께 실린다 — 스냅샷은 그 티어의 에이전트만 담는다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      tier: 'medium',
      overrides: { 'critique-medium': { model: 'gpt-5.6-sol', effort: 'high' } },
    });
    expect(rows.review.tier).toBe('medium');
    expect(Object.keys(rows.review.modelConfig)).toEqual([
      'research-medium',
      'critique-medium',
      'proofread-medium',
      'rephrase-medium',
      'conclude-medium',
    ]);
    expect(rows.review.modelConfig['critique-medium']).toEqual({ model: 'gpt-5.6-sol', effort: 'high', overridden: true });
  });
});

describe('startFeedbackSession', () => {
  it('세 행을 batch 한 묶음으로 넣고 prism 워크플로를 띄운다', async () => {
    const spy = route({});
    const { db, batches, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io' });

    expect(outcome).toEqual({ sessionId: inserts[0].row.id });
    expect(batches).toHaveLength(1);
    expect(batches[0]).toHaveLength(3);
    expect(inserts.map((i) => i.table)).toEqual([FeedbackSessions, ManuscriptVersions, Reviews]);
    expect(inserts[0].row).toMatchObject({ refId: 'D0TEST01', title: '제목', testerEmail: 't@x.io' });
    expect(updates).toHaveLength(0);

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect(body.workflowId).toBe(inserts[2].row.prismWorkflowId);
    expect(body.input).toEqual({ manuscriptPath: 'manuscript/v1.txt' });
    expect(body.files).toEqual([{ path: 'manuscript/v1.txt', content: '본문' }]);
    // 티어 무지정은 high — 행·워크플로 이름이 같은 값을 가리킨다
    expect(inserts[2].row.tier).toBe('high');
    expect(body.workflow).toBe('high');
  });

  it('티어는 리뷰 행에 저장되고 그 이름 그대로 워크플로가 된다', async () => {
    const spy = route({});
    const { db, inserts } = createDbStub();

    await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', tier: 'low' });

    expect(inserts[2].row.tier).toBe('low');
    expect(Object.keys(inserts[2].row.modelConfig as object)).toEqual(['critique-low', 'proofread-low', 'rephrase-low']);
    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect(body.workflow).toBe('low');
  });

  it('오버라이드가 있으면 /workflows input에 sparse로 실린다', async () => {
    const spy = route({});
    const { db } = createDbStub();

    await startFeedbackSession(db, env, {
      refId: 'D0TEST01',
      email: 't@x.io',
      tier: 'medium',
      overrides: { 'rephrase-medium': { model: 'gpt-5.6-luna', effort: 'low' } },
    });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect(body.workflow).toBe('medium');
    expect(body.input.overrides).toEqual({ 'rephrase-medium': { model: 'gpt-5.6-luna', effort: 'low' } });
  });

  it('무오버라이드면 input에 overrides 키가 없다', async () => {
    const spy = route({});
    const { db } = createDbStub();

    await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io' });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect('overrides' in body.input).toBe(false);
  });

  it('반입 반려는 사용자 문면 그대로 돌리고 아무 행도 쓰지 않는다', async () => {
    route({ extract: () => Promise.resolve(Response.json({ results: [{ documentId: 'D0TEST01', prose: null, title: null }] })) });
    const { db, batches, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io' });

    expect(outcome).toEqual({ error: '문서를 찾을 수 없어요. 문서 ID를 확인해 주세요' });
    expect(batches).toHaveLength(0);
    expect(inserts).toHaveLength(0);
    expect(updates).toHaveLength(0);
  });

  it('시작 실패(PrismApiError)는 리뷰 행에 귀속하고 sessionId는 그대로 돌린다', async () => {
    route({ workflows: () => Promise.resolve(Response.json({ error: 'forbidden' }, { status: 403 })) });
    const { db, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io' });

    expect(outcome).toEqual({ sessionId: inserts[0].row.id });
    expect(updates).toHaveLength(1);
    expect(updates[0].table).toBe(Reviews);
    expect(updates[0].values.status).toBe('failed');
    expect(updates[0].values.error).toContain('prism-api 403: forbidden');
    expect(updates[0].values.finishedAt).toBeInstanceOf(Date);
  });

  it('네트워크 실패(raw TypeError)도 같은 경로로 귀속하고 1000자로 자른다', async () => {
    route({ workflows: () => Promise.reject(new TypeError('가'.repeat(2000))) });
    const { db, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io' });

    expect(outcome).toEqual({ sessionId: inserts[0].row.id });
    expect(updates).toHaveLength(1);
    expect(updates[0].values.status).toBe('failed');
    expect(updates[0].values.error).toHaveLength(1000);
    expect(updates[0].values.error).toContain('TypeError');
  });
});

describe('requestCancel', () => {
  it('prism 취소만 부르고 로컬 상태는 건드리지 않는다', async () => {
    const spy = route({});
    await requestCancel(env, 'ev-x');
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows/ev-x/cancel');
    expect(init?.body).toBeUndefined();
  });
});
