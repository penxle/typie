import { afterEach, describe, expect, it, vi } from 'vitest';
import { FeedbackSessions, ManuscriptVersions, Reviews } from './db/index.ts';
import { buildPreviousContext, buildStartRows, requestCancel, resumeReview, startFeedbackSession, startRereview } from './reviews.ts';
import type { AppCatalog } from '../feedback/tiers.ts';
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

// 카탈로그 픽스처 — prism 카탈로그 표면의 형태 재현이다. 값의 정본 동기화 의무는 없다(기능 검증용).
const CATALOG = {
  models: {
    'claude-opus-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
    'claude-sonnet-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
    'gpt-5.6-sol': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
    'gpt-5.6-luna': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
    'gemini-3.6-flash': { provider: 'gemini', efforts: ['minimal', 'low', 'medium', 'high'] },
  },
  agents: {
    'description-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'interpretation-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'rubric-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'calibration-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'judgment-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'stylistic-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'delivery-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'research-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'critique-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'high' },
    'proofread-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'high' },
    'rephrase-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'conclude-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'critique-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
    'proofread-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
    'rephrase-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
  },
  workflows: {
    high: {
      agents: [
        'description-high',
        'interpretation-high',
        'rubric-high',
        'calibration-high',
        'judgment-high',
        'stylistic-high',
        'delivery-high',
      ],
    },
    medium: { agents: ['research-medium', 'critique-medium', 'proofread-medium', 'rephrase-medium', 'conclude-medium'] },
    low: { agents: ['critique-low', 'proofread-low', 'rephrase-low'] },
  },
} satisfies AppCatalog;

const extractOk = () =>
  Promise.resolve(Response.json({ results: [{ documentId: 'D0TEST01', prose: '본문', title: '제목', subtitle: '부제' }] }));

const route = (handlers: {
  extract?: () => Promise<Response>;
  workflows?: () => Promise<Response>;
  retry?: () => Promise<Response>;
  file?: () => Promise<Response>;
  catalog?: () => Promise<Response>;
}) =>
  vi.spyOn(globalThis, 'fetch').mockImplementation((input) => {
    const url = String(input);
    if (url.endsWith('/internal/corpus/extract')) return (handlers.extract ?? extractOk)();
    if (url.endsWith('/apps/feedback/catalog')) return (handlers.catalog ?? (() => Promise.resolve(Response.json(CATALOG))))();
    if (url.endsWith('/workflows')) return (handlers.workflows ?? (() => Promise.resolve(Response.json({}))))();
    if (url.endsWith('/cancel')) return Promise.resolve(Response.json({}));
    if (url.endsWith('/retry')) return (handlers.retry ?? (() => Promise.resolve(Response.json({}))))();
    if (url.includes('/files/')) return (handlers.file ?? (() => Promise.resolve(Response.json({ content: '' }))))();
    throw new Error(`unexpected fetch: ${url}`);
  });

afterEach(() => vi.restoreAllMocks());

describe('buildStartRows', () => {
  it('세션·버전1·리뷰1(running)의 정합 행을 만든다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      subtitle: '부제',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      catalog: CATALOG,
    });
    expect(rows.session.refId).toBe('D0TEST01');
    expect(rows.version).toMatchObject({
      sessionId: rows.session.id,
      version: 1,
      content: '본문',
      title: '제목',
      subtitle: '부제',
      charCount: 2,
    });
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
      subtitle: null,
      content,
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      catalog: CATALOG,
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
      subtitle: null,
      content,
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      catalog: CATALOG,
    });
    expect(rows.version.charCount).toBe(2);
    expect(rows.version.content).toBe(content);
  });

  it('buildStartRows가 modelConfig 스냅샷을 싣는다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      subtitle: '부제',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      catalog: CATALOG,
      overrides: { 'calibration-high': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'xhigh' } },
    });
    expect(rows.review.modelConfig['calibration-high']).toEqual({
      provider: 'anthropic',
      model: 'claude-sonnet-5',
      effort: 'xhigh',
      overridden: true,
    });
    expect(rows.review.modelConfig['description-high']).toEqual({
      provider: 'anthropic',
      model: 'claude-opus-5',
      effort: 'high',
      overridden: false,
    });
  });

  it('티어 무지정은 high 행으로 굳는다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      subtitle: '부제',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      catalog: CATALOG,
    });
    expect(rows.review.tier).toBe('high');
    expect(Object.keys(rows.review.modelConfig)).toHaveLength(7);
  });

  it('티어는 행과 스냅샷에 함께 실린다 — 스냅샷은 그 티어의 에이전트만 담는다', () => {
    const rows = buildStartRows({
      refId: 'D0TEST01',
      email: 't@x.io',
      title: '제목',
      subtitle: '부제',
      content: '본문',
      prismWorkflowId: 'ev-x',
      now: new Date(0),
      catalog: CATALOG,
      tier: 'medium',
      overrides: { 'critique-medium': { provider: 'openai', model: 'gpt-5.6-sol', effort: 'high' } },
    });
    expect(rows.review.tier).toBe('medium');
    expect(Object.keys(rows.review.modelConfig)).toEqual([
      'research-medium',
      'critique-medium',
      'proofread-medium',
      'rephrase-medium',
      'conclude-medium',
    ]);
    expect(rows.review.modelConfig['critique-medium']).toEqual({
      provider: 'openai',
      model: 'gpt-5.6-sol',
      effort: 'high',
      overridden: true,
    });
  });
});

describe('buildPreviousContext', () => {
  const thread = (id: string, state: 'open' | 'closed' | 'resolved' | 'withdrawn', over: Record<string, unknown> = {}) => ({
    id,
    pass: 'judgment' as const,
    body: '전환이 갑작스럽다' as string | null,
    anchors: [{ start: 0, end: 5, head: '첫 문장', tail: '첫 문장' }],
    state,
    trait: '장면 전환',
    issueId: null as string | null,
    comments: [] as { author: 'tester' | 'ai'; body: string; createdAt: Date }[],
    ...over,
  });

  it('스레드는 상태 불문 전건이 실리고 상태가 그대로 따라간다 — 역할 분류는 prism의 몫이다', () => {
    const context = buildPreviousContext({
      threads: [
        thread('t.1.0', 'open'),
        thread('t.1.1', 'closed', { body: '문장이 길다' }),
        thread('t.1.2', 'resolved', { body: '시점이 흔들린다' }),
        thread('t.1.3', 'withdrawn', { body: '표현이 반복된다' }),
      ],
      manuscriptPath: 'manuscript/v1.txt',
      baseStartedAt: new Date(500),
      meta: { title: '제목', subtitle: null },
    });
    expect(context.manuscriptPath).toBe('manuscript/v1.txt');
    expect(context.meta).toEqual({ title: '제목', subtitle: null });
    expect(context.threads.map((t) => [t.id, t.state])).toEqual([
      ['t.1.0', 'open'],
      ['t.1.1', 'closed'],
      ['t.1.2', 'resolved'],
      ['t.1.3', 'withdrawn'],
    ]);
  });

  it('사영은 스키마가 연 필드만 통과시킨다 — 앵커의 좌표도 행의 다른 컬럼도 싣지 않는다', () => {
    const context = buildPreviousContext({
      threads: [
        thread('t.1.0', 'open', {
          comments: [{ author: 'tester', body: '의도한 전환입니다', createdAt: new Date(1000) }],
        }),
      ],
      manuscriptPath: 'manuscript/v1.txt',
      baseStartedAt: new Date(500),
      meta: { title: '제목', subtitle: null },
    });
    // toEqual은 여분 키를 잡아낸다 — prism의 PREVIOUS는 전 object가 additionalProperties: false다.
    expect(context.threads).toEqual([
      {
        id: 't.1.0',
        pass: 'judgment',
        trait: '장면 전환',
        body: '전환이 갑작스럽다',
        anchors: [{ head: '첫 문장', tail: '첫 문장' }],
        replies: [{ body: '의도한 전환입니다', fresh: true }],
        state: 'open',
      },
    ]);
  });

  it('본문 없는 스레드는 빈 문자열로 선다 — 스키마의 body는 required string이다', () => {
    const context = buildPreviousContext({
      threads: [thread('t.1.0', 'open', { body: null })],
      manuscriptPath: 'manuscript/v1.txt',
      baseStartedAt: new Date(500),
      meta: { title: '제목', subtitle: null },
    });
    expect(context.threads[0].body).toBe('');
  });

  it('fresh는 base 회차 시작 시각으로 가른다 — 그 이전·동시각 답글은 지난 리뷰가 이미 본 것이다', () => {
    const context = buildPreviousContext({
      threads: [
        thread('t.1.0', 'open', {
          comments: [
            { author: 'tester', body: '지난 회차 전', createdAt: new Date(400) },
            { author: 'tester', body: '시작과 동시각', createdAt: new Date(500) },
            { author: 'tester', body: '지난 회차 후', createdAt: new Date(600) },
          ],
        }),
      ],
      manuscriptPath: 'manuscript/v1.txt',
      baseStartedAt: new Date(500),
      meta: { title: '제목', subtitle: null },
    });
    expect(context.threads[0].replies).toEqual([
      { body: '지난 회차 전', fresh: false },
      { body: '시작과 동시각', fresh: false },
      { body: '지난 회차 후', fresh: true },
    ]);
  });

  it('AI 코멘트는 답글에 싣지 않는다 — 지난 리뷰 자신의 산출물이다', () => {
    const context = buildPreviousContext({
      threads: [
        thread('t.1.0', 'open', {
          comments: [
            { author: 'ai', body: '처분 코멘트', createdAt: new Date(1) },
            { author: 'tester', body: '작가의 반론', createdAt: new Date(600) },
          ],
        }),
      ],
      manuscriptPath: 'manuscript/v1.txt',
      baseStartedAt: new Date(500),
      meta: { title: '제목', subtitle: null },
    });
    expect(context.threads[0].replies).toEqual([{ body: '작가의 반론', fresh: true }]);
  });

  it('이슈 id는 있으면 회송하고 없으면 키 자체가 서지 않는다', () => {
    const context = buildPreviousContext({
      threads: [thread('t.1.0', 'open', { issueId: 'i.abc' }), thread('t.1.1', 'open'), thread('t.1.2', 'open', { issueId: '' })],
      manuscriptPath: 'manuscript/v1.txt',
      baseStartedAt: new Date(500),
      meta: { title: '제목', subtitle: null },
    });
    expect(context.threads[0].issue).toBe('i.abc');
    // 스키마의 issue는 min(1) optional이다 — 빈 값은 키를 세우는 대신 생략한다(undefined도 직렬화 금지).
    expect('issue' in context.threads[1]).toBe(false);
    expect('issue' in context.threads[2]).toBe(false);
    expect(JSON.stringify(context.threads[1])).not.toContain('issue');
  });
});

describe('startRereview 구세션 가드', () => {
  // 가드가 반려하는 경로는 회차 목록 하나만 읽고 끝난다 — 행을 세우는 문장이 조립되면 이 스텁이 붙잡는다.
  const createGuardDbStub = () => {
    const inserts: unknown[] = [];
    const batches: unknown[][] = [];
    const db = {
      select: () => ({ from: () => ({ where: () => ({ orderBy: () => Promise.resolve([completedRound]) }) }) }),
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
    };
    return { db: db as unknown as Db, inserts, batches };
  };

  const completedRound = {
    sessionId: 's1',
    round: 1,
    status: 'completed',
    tier: 'high',
    prismWorkflowId: 'ev-x',
    manuscriptVersion: 1,
    startedAt: new Date(500),
    modelConfig: null,
    result: null,
  };

  it('지난 산출물이 없는 세션은 행을 세우기 전에 반려한다', async () => {
    const spy = route({ file: () => Promise.resolve(Response.json({ content: null })) });
    const { db, inserts, batches } = createGuardDbStub();

    const outcome = await startRereview(db, env, 's1');

    expect(outcome).toEqual({ error: '이 세션은 이전 버전 리뷰라 다시 요청할 수 없어요. 새 피드백으로 시작해 주세요' });
    expect(inserts).toHaveLength(0);
    expect(batches).toHaveLength(0);
    // 가드가 카탈로그·원고 반입보다 앞이라 반려에 딸린 왕복이 없다
    expect(spy.mock.calls.map(([url]) => String(url))).toEqual(['https://prism.test/workflows/ev-x/files/artifacts/continuity.yaml']);
  });

  it('산출물 조회 자체가 실패하면 구세션으로 단정하지 않고 재시도를 안내한다', async () => {
    route({ file: () => Promise.reject(new TypeError('network down')) });
    const { db, inserts, batches } = createGuardDbStub();

    const outcome = await startRereview(db, env, 's1');

    expect(outcome).toEqual({ error: '리뷰를 다시 시작하지 못했어요. 잠시 후 다시 시도해 주세요' });
    expect(inserts).toHaveLength(0);
    expect(batches).toHaveLength(0);
  });
});

describe('startFeedbackSession', () => {
  it('세 행을 batch 한 묶음으로 넣고 prism 워크플로를 띄운다', async () => {
    const spy = route({});
    const { db, batches, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG });

    expect(outcome).toEqual({ sessionId: inserts[0].row.id });
    expect(batches).toHaveLength(1);
    expect(batches[0]).toHaveLength(3);
    expect(inserts.map((i) => i.table)).toEqual([FeedbackSessions, ManuscriptVersions, Reviews]);
    expect(inserts[0].row).toMatchObject({ refId: 'D0TEST01', title: '제목', testerEmail: 't@x.io' });
    expect(updates).toHaveLength(0);

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect(body.workflowId).toBe(inserts[2].row.prismWorkflowId);
    expect(body.input).toEqual({ manuscriptPath: 'manuscript/v1.txt', meta: { title: '제목', subtitle: '부제' } });
    expect(body.files).toEqual([{ path: 'manuscript/v1.txt', content: '본문' }]);
    // 티어 무지정은 high — 행·워크플로 이름이 같은 값을 가리킨다
    expect(inserts[2].row.tier).toBe('high');
    expect(body.workflow).toBe('high');
  });

  it('티어는 리뷰 행에 저장되고 그 이름 그대로 워크플로가 된다', async () => {
    const spy = route({});
    const { db, inserts } = createDbStub();

    await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG, tier: 'low' });

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
      catalog: CATALOG,
      tier: 'medium',
      overrides: { 'rephrase-medium': { provider: 'openai', model: 'gpt-5.6-luna', effort: 'low' } },
    });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect(body.workflow).toBe('medium');
    expect(body.input.overrides).toEqual({ 'rephrase-medium': { provider: 'openai', model: 'gpt-5.6-luna', effort: 'low' } });
  });

  it('제목·부제가 input.meta로 실린다', async () => {
    const spy = route({});
    const { db } = createDbStub();

    await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string) as {
      input: { meta: unknown };
    };
    expect(body.input.meta).toEqual({ title: '제목', subtitle: '부제' });
  });

  it('무오버라이드면 input에 overrides 키가 없다', async () => {
    const spy = route({});
    const { db } = createDbStub();

    await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string);
    expect('overrides' in body.input).toBe(false);
  });

  it('반입 반려는 사용자 문면 그대로 돌리고 아무 행도 쓰지 않는다', async () => {
    route({ extract: () => Promise.resolve(Response.json({ results: [{ documentId: 'D0TEST01', prose: null, title: null }] })) });
    const { db, batches, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG });

    expect(outcome).toEqual({ error: '문서를 찾을 수 없어요. 문서 ID를 확인해 주세요' });
    expect(batches).toHaveLength(0);
    expect(inserts).toHaveLength(0);
    expect(updates).toHaveLength(0);
  });

  it('시작 실패(PrismApiError)는 리뷰 행에 귀속하고 sessionId는 그대로 돌린다', async () => {
    route({ workflows: () => Promise.resolve(Response.json({ error: 'forbidden' }, { status: 403 })) });
    const { db, inserts, updates } = createDbStub();

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG });

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

    const outcome = await startFeedbackSession(db, env, { refId: 'D0TEST01', email: 't@x.io', catalog: CATALOG });

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

describe('resumeReview', () => {
  // select 체인은 회차 목록 하나만 답한다 — resumeReview의 유일한 읽기다.
  const createResumeDbStub = (rounds: Record<string, unknown>[]) => {
    const updates: { values: Record<string, unknown> }[] = [];
    const db = {
      select: () => ({ from: () => ({ where: () => ({ orderBy: () => Promise.resolve(rounds) }) }) }),
      update: () => ({
        set: (values: Record<string, unknown>) => ({
          where: () => {
            updates.push({ values });
            return Promise.resolve([]);
          },
        }),
      }),
    };
    return { db: db as unknown as Db, updates };
  };

  const failedRound = { sessionId: 's1', round: 2, status: 'failed', prismWorkflowId: 'ev-x' };

  it('prism retry를 부르고 행을 running으로 되돌린다 — 종결 산출물 컬럼은 눕는다', async () => {
    const spy = route({});
    const { db, updates } = createResumeDbStub([{ ...failedRound, round: 1 }, failedRound]);

    const outcome = await resumeReview(db, env, 's1');

    expect(outcome).toEqual({ round: 2 });
    const [url, init] = spy.mock.calls[0];
    expect(url).toBe('https://prism.test/workflows/ev-x/retry');
    expect(init?.body).toBeUndefined();
    expect(updates).toHaveLength(1);
    expect(updates[0].values).toEqual({
      status: 'running',
      error: null,
      result: null,
      usage: null,
      events: null,
      questions: null,
      finishedAt: null,
    });
  });

  it('최신 회차가 failed가 아니면 반려한다 — 취소·실행 중·완료 전부', async () => {
    const spy = route({});
    for (const status of ['canceled', 'running', 'completed']) {
      const { db, updates } = createResumeDbStub([{ ...failedRound, status }]);
      const outcome = await resumeReview(db, env, 's1');
      expect(outcome).toEqual({ error: '지금은 이어서 다시 시도할 수 없어요' });
      expect(updates).toHaveLength(0);
    }
    expect(spy).not.toHaveBeenCalled();
  });

  it('retry-rejected는 수렴 경로다 — prism은 이미 failed가 아니니 행만 되돌리고 성공으로 답한다', async () => {
    route({ retry: () => Promise.resolve(Response.json({ error: 'retry-rejected' }, { status: 409 })) });
    const { db, updates } = createResumeDbStub([failedRound]);

    const outcome = await resumeReview(db, env, 's1');

    expect(outcome).toEqual({ round: 2 });
    expect(updates).toHaveLength(1);
    expect(updates[0].values.status).toBe('running');
  });

  it('retry-unsettled는 일시 반려다 — 행을 건드리지 않는다', async () => {
    route({ retry: () => Promise.resolve(Response.json({ error: 'retry-unsettled' }, { status: 409 })) });
    const { db, updates } = createResumeDbStub([failedRound]);

    const outcome = await resumeReview(db, env, 's1');

    expect(outcome).toEqual({ error: '아직 이어서 다시 시도할 준비가 안 됐어요. 잠시 후 다시 시도해 주세요' });
    expect(updates).toHaveLength(0);
  });

  it('404(워크플로 부재)는 시작 실패다 — 새 세션 경로로 안내하고 행을 건드리지 않는다', async () => {
    route({ retry: () => Promise.resolve(Response.json({ error: 'not-found' }, { status: 404 })) });
    const { db, updates } = createResumeDbStub([failedRound]);

    const outcome = await resumeReview(db, env, 's1');

    expect(outcome).toEqual({ error: '이어서 다시 시도할 수 없는 실패예요. 새 세션으로 처음부터 시작해 주세요' });
    expect(updates).toHaveLength(0);
  });

  it('네트워크 실패(raw TypeError)는 재시도 안내로 반려한다', async () => {
    route({ retry: () => Promise.reject(new TypeError('network down')) });
    const { db, updates } = createResumeDbStub([failedRound]);

    const outcome = await resumeReview(db, env, 's1');

    expect(outcome).toEqual({ error: '다시 시도 요청을 보내지 못했어요. 잠시 후 다시 시도해 주세요' });
    expect(updates).toHaveLength(0);
  });
});

describe('startRereview 제목·부제', () => {
  const round1 = {
    sessionId: 's1',
    round: 1,
    status: 'completed',
    tier: 'high',
    prismWorkflowId: 'ev-x',
    manuscriptVersion: 1,
    startedAt: new Date(500),
    modelConfig: null,
    result: null,
  };
  const session1 = { id: 's1', refId: 'D0TEST01', title: '구제목', testerEmail: 't@x.io', createdAt: new Date(100) };
  // 구 행 — 제목 미기록(NULL). 무제목과 구분하지 않는다(오너 결정).
  const version1 = { sessionId: 's1', version: 1, content: '본문', title: null, subtitle: null, charCount: 2, importedAt: new Date(100) };

  const createRereviewDbStub = (queue: unknown[][]) => {
    const inserts: { table: unknown; row: Record<string, unknown> }[] = [];
    const updates: { table: unknown; values: Record<string, unknown> }[] = [];
    const batches: unknown[][] = [];
    const chain = (rows: unknown[]) => {
      const p = Promise.resolve(rows) as Promise<unknown[]> & Record<string, () => unknown>;
      for (const m of ['from', 'where', 'orderBy', 'limit', 'innerJoin']) p[m] = () => p;
      return p;
    };
    const db = {
      select: () => chain(queue.shift() ?? []),
      insert: (table: unknown) => ({
        values: (row: Record<string, unknown>) => {
          inserts.push({ table, row });
          return { table, row };
        },
      }),
      update: (table: unknown) => ({
        set: (values: Record<string, unknown>) => ({
          where: () => {
            updates.push({ table, values });
            return Promise.resolve([]);
          },
        }),
      }),
      batch: (statements: unknown[]) => {
        batches.push(statements);
        return Promise.resolve([]);
      },
    };
    return { db: db as unknown as Db, inserts, updates, batches };
  };

  it('제목만 바뀐 재검토도 새 버전이 서고, previous.meta는 구 행의 NULL 그대로 실린다', async () => {
    const spy = route({ file: () => Promise.resolve(Response.json({ content: 'seed' })) });
    // select 순서: 회차 목록 → 세션 → 최신 버전 → 스레드 → 코멘트 → base 버전
    const { db, inserts, updates, batches } = createRereviewDbStub([[round1], [session1], [version1], [], [], [version1]]);

    const outcome = await startRereview(db, env, 's1');

    expect(outcome).toEqual({ round: 2 });
    expect(batches[0]).toHaveLength(3);
    const versionRow = inserts.find((i) => i.table === ManuscriptVersions)?.row;
    expect(versionRow).toMatchObject({ version: 2, content: '본문', title: '제목', subtitle: '부제' });
    expect(updates).toContainEqual({ table: FeedbackSessions, values: { title: '제목' } });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string) as {
      input: { manuscriptPath: string; meta: unknown; previous: { manuscriptPath: string; meta: unknown } };
    };
    expect(body.input.manuscriptPath).toBe('manuscript/v2.txt');
    expect(body.input.meta).toEqual({ title: '제목', subtitle: '부제' });
    expect(body.input.previous.manuscriptPath).toBe('manuscript/v1.txt');
    expect(body.input.previous.meta).toEqual({ title: null, subtitle: null });
  });

  it('본문·제목·부제가 전건 동일하면 버전을 재사용한다', async () => {
    const spy = route({ file: () => Promise.resolve(Response.json({ content: 'seed' })) });
    const sameVersion = { ...version1, title: '제목', subtitle: '부제' };
    const { db, inserts } = createRereviewDbStub([[round1], [session1], [sameVersion], [], [], [sameVersion]]);

    const outcome = await startRereview(db, env, 's1');

    expect(outcome).toEqual({ round: 2 });
    expect(inserts.some((i) => i.table === ManuscriptVersions)).toBe(false);

    // 버전을 재사용하면 신·구 원고가 같은 경로를 가리킨다 — prism은 이 동일성으로 무변경 재리뷰를 가른다.
    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string) as {
      input: { manuscriptPath: string; previous: { manuscriptPath: string } };
    };
    expect(body.input.manuscriptPath).toBe('manuscript/v1.txt');
    expect(body.input.previous.manuscriptPath).toBe('manuscript/v1.txt');
  });

  it('본문만 바뀌어도 새 버전이 서고, previous.meta는 base 행의 값 그대로 실린다', async () => {
    const spy = route({
      extract: () =>
        Promise.resolve(Response.json({ results: [{ documentId: 'D0TEST01', prose: '새 본문', title: '제목', subtitle: '부제' }] })),
      file: () => Promise.resolve(Response.json({ content: 'seed' })),
    });
    const baseVersion = { ...version1, title: '제목', subtitle: '부제' };
    const { db, inserts } = createRereviewDbStub([[round1], [session1], [baseVersion], [], [], [baseVersion]]);

    const outcome = await startRereview(db, env, 's1');

    expect(outcome).toEqual({ round: 2 });
    const versionRow = inserts.find((i) => i.table === ManuscriptVersions)?.row;
    expect(versionRow).toMatchObject({ version: 2, content: '새 본문', title: '제목', subtitle: '부제' });

    const body = JSON.parse(spy.mock.calls.find(([url]) => String(url).endsWith('/workflows'))?.[1]?.body as string) as {
      input: { manuscriptPath: string; previous: { manuscriptPath: string; meta: unknown } };
    };
    expect(body.input.manuscriptPath).toBe('manuscript/v2.txt');
    expect(body.input.previous.manuscriptPath).toBe('manuscript/v1.txt');
    expect(body.input.previous.meta).toEqual({ title: '제목', subtitle: '부제' });
  });
});
