import { getTableColumns } from 'drizzle-orm';
import { SQLiteSyncDialect } from 'drizzle-orm/sqlite-core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Reviews, ThreadComments, Threads } from './db/index.ts';
import { getAgentAskCalls, getWorkflow, openEvents } from './prism.ts';
import { applyDispositions, carriedIssues, collectEvents, projectIfTerminal, threadId, threadsFromResult } from './project.ts';
import type { Column, SQL, Table } from 'drizzle-orm';
import type { FeedbackResult, PrismWorkflow, ThreadDisposition } from '../feedback/types.ts';
import type { Db } from './db/index.ts';

vi.mock('./prism.ts', () => ({
  fetchEventLog: vi.fn(),
  getAgentAskCalls: vi.fn(),
  getWorkflow: vi.fn(),
  openEvents: vi.fn(),
}));

const result: FeedbackResult = {
  version: 1,
  issues: [
    { trait: '인물의 동기', pass: 'judgment', body: '지적 문면', anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }] },
    { trait: '문장 결', pass: 'stylistic', body: null, anchors: [] },
  ],
  conclusion: { understanding: null, strengths: [], patterns: [], priorities: [] },
};

describe('threadsFromResult', () => {
  it('issue 인덱스를 보존하며 thread 행으로 정규화한다', () => {
    const rows = threadsFromResult('s1', 1, result);
    expect(rows).toEqual([
      {
        id: threadId('s1', 1, 0),
        sessionId: 's1',
        reviewRound: 1,
        issueIndex: 0,
        trait: '인물의 동기',
        pass: 'judgment',
        body: '지적 문면',
        anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }],
        state: 'open',
        stateChangedAt: null,
        issueId: null,
      },
      expect.objectContaining({ id: threadId('s1', 1, 1), issueIndex: 1, body: null }),
    ]);
  });

  it('이슈의 id를 행에 싣는다 — 총평 참조가 이 신원으로 스레드를 가리킨다', () => {
    const identified: FeedbackResult = {
      ...result,
      issues: result.issues.map((issue, index) => ({ ...issue, id: `judgment-${index + 1}` })),
    };

    expect(threadsFromResult('s1', 1, identified).map((row) => row.issueId)).toEqual(['judgment-1', 'judgment-2']);
  });

  it('id 없는 결과(지난 회차·id를 다루지 않는 티어)는 issueId가 null이다', () => {
    expect(threadsFromResult('s1', 1, result).map((row) => row.issueId)).toEqual([null, null]);
  });

  it('threadId는 결정적이다', () => {
    expect(threadId('s1', 1, 0)).toBe(threadId('s1', 1, 0));
    expect(threadId('s1', 1, 0)).not.toBe(threadId('s1', 2, 0));
  });
});

describe('승계 이슈(thread 표지)', () => {
  const carried: FeedbackResult = {
    ...result,
    issues: [
      { trait: '지난 축', pass: 'judgment', body: '지난 문면', anchors: [{ start: 0, end: 5, head: '앞', tail: '앞' }], thread: 's1.1.0' },
      ...result.issues,
    ],
  };

  it('thread 표지 이슈는 새 스레드 행이 되지 않는다', () => {
    const rows = threadsFromResult('s1', 2, carried);
    expect(rows.map((row) => row.issueIndex)).toEqual([1, 2]);
    expect(rows.map((row) => row.id)).toEqual([threadId('s1', 2, 1), threadId('s1', 2, 2)]);
  });

  it('carriedIssues가 기존 스레드를 앉힐 좌표를 추출한다', () => {
    expect(carriedIssues(carried)).toEqual([
      { threadId: 's1.1.0', issueIndex: 0, anchors: [{ start: 0, end: 5, head: '앞', tail: '앞' }] },
    ]);
    expect(carriedIssues(result)).toEqual([]);
  });
});

describe('collectEvents', () => {
  it('재생 스트림을 EOF까지 소비해 id 프레임만 수집한다', async () => {
    const body = new ReadableStream<Uint8Array>({
      start(c) {
        c.enqueue(new TextEncoder().encode('id: 1\nevent: workflow.started\ndata: {}\n\n: hb\n\nevent: turn.delta\ndata: {}\n\n'));
        c.enqueue(new TextEncoder().encode('id: 2\nevent: workflow.completed\ndata: {"result":"{}"}\n\n'));
        c.close();
      },
    });
    const env = { PRISM_API_ORIGIN: 'x', PRISM_API_TOKEN: 'x' };
    const events = await collectEvents(env, 'ev-x', () => Promise.resolve(new Response(body)));
    expect(events.map((e) => e.event)).toEqual(['workflow.started', 'workflow.completed']);
  });
});

// db는 호출 기록용 스텁이다(reviews.test.ts와 같은 형태). 사영이 부르는 두 문장만 받는다.
const createDbStub = () => {
  const inserts: { table: unknown; row: Record<string, unknown> }[] = [];
  const updates: { table: unknown; values: Record<string, unknown> }[] = [];
  const db = {
    insert: (table: unknown) => ({
      values: (row: Record<string, unknown>) => ({
        onConflictDoNothing: () => {
          inserts.push({ table, row });
          return Promise.resolve([]);
        },
      }),
    }),
    update: (table: unknown) => ({
      set: (values: Record<string, unknown>) => ({
        where: () => {
          updates.push({ table, values });
          return Promise.resolve([]);
        },
      }),
    }),
  };
  return { db: db as unknown as Db, inserts, updates };
};

type Row = Record<string, unknown>;

// 처분 사영은 현재 상태를 읽고 조건부로 고치는 문장이라 호출 기록만으로는 멱등·closed 보호가 보이지 않는다 —
// 이 블록만 행을 들고 있는 스텁을 쓴다. where는 드리즐 방언으로 렌더해 eq/and 조합만 해석하고, 그 밖의 형태는
// 조용히 통과시키지 않고 던진다.
const dialect = new SQLiteSyncDialect();

const propertyOf = (table: Table, name: string) => {
  const property = Object.entries(getTableColumns(table)).find(([, column]) => column.name === name)?.[0];
  if (property === undefined) throw new Error(`unknown column: ${name}`);
  return property;
};

const matches = (table: Table, where: SQL) => {
  const { sql, params } = dialect.sqlToQuery(where);
  const terms = sql.replaceAll(/[()]/g, '').split(' and ');
  if (terms.length !== params.length) throw new Error(`unsupported where: ${sql}`);
  const conditions = terms.map((term, index) => {
    const name = /^"[^"]+"\."([^"]+)" = \?$/.exec(term.trim())?.[1];
    if (name === undefined) throw new Error(`unsupported where: ${sql}`);
    return [propertyOf(table, name), params[index]] as const;
  });
  return (row: Row) => conditions.every(([property, value]) => row[property] === value);
};

const createStoreStub = (seed: Row[]) => {
  const tables = new Map<unknown, Row[]>();
  const rowsOf = (table: Table) => {
    const rows = tables.get(table);
    if (rows) return rows;
    const created: Row[] = [];
    tables.set(table, created);
    return created;
  };
  const threads = rowsOf(Threads);
  threads.push(...seed.map((row) => ({ ...row })));
  const comments = rowsOf(ThreadComments);

  const db = {
    select: (projection: Record<string, Column>) => ({
      from: (table: Table) => ({
        where: (where: SQL) => ({
          limit: (count: number) =>
            Promise.resolve(
              rowsOf(table)
                .filter(matches(table, where))
                .slice(0, count)
                .map((row) =>
                  Object.fromEntries(Object.entries(projection).map(([alias, column]) => [alias, row[propertyOf(table, column.name)]])),
                ),
            ),
        }),
      }),
    }),
    insert: (table: Table) => ({
      values: (row: Row) => ({
        onConflictDoNothing: () => {
          const rows = rowsOf(table);
          if (rows.every((existing) => existing.id !== row.id)) rows.push({ ...row });
          return Promise.resolve([]);
        },
      }),
    }),
    update: (table: Table) => ({
      set: (values: Row) => ({
        where: (where: SQL) => {
          const matched = matches(table, where);
          for (const row of rowsOf(table)) if (matched(row)) Object.assign(row, values);
          return Promise.resolve([]);
        },
      }),
    }),
  };
  return { db: db as unknown as Db, threads, comments };
};

const threadRow = (id: string, over: Row = {}): Row => ({
  id,
  sessionId: 's',
  reviewRound: 1,
  issueIndex: 0,
  trait: '인물의 동기',
  pass: 'judgment',
  body: '지적 문면',
  anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }],
  state: 'open',
  stateChangedAt: null,
  ...over,
});

const env = { PRISM_API_ORIGIN: 'https://prism.test', PRISM_API_TOKEN: 'tk' };
const review = { sessionId: 's1', round: 1, prismWorkflowId: 'ev-x' };

const workflow = (over: Partial<PrismWorkflow>): PrismWorkflow => ({
  status: 'completed',
  result: null,
  error: null,
  usage: null,
  startedAt: 1000,
  finishedAt: 2000,
  ...over,
});

const sse = (text: string) =>
  new Response(
    new ReadableStream<Uint8Array>({
      start(c) {
        c.enqueue(new TextEncoder().encode(text));
        c.close();
      },
    }),
  );

const replay = () => sse('id: 1\nevent: workflow.started\ndata: {}\n\nid: 2\nevent: workflow.completed\ndata: {}\n\n');

// 질문 재생분 — 프레임 본문은 {seq,kind,data,createdAt} 봉투다(questions.test.ts의 wrap과 같은 형태).
const frame = (id: number, kind: string, data: Record<string, unknown>) =>
  `id: ${id}\nevent: ${kind}\ndata: ${JSON.stringify({ seq: id, kind, data, createdAt: 1000 + id })}\n\n`;

const ASK_INPUT = JSON.stringify({ questions: [{ question: 'Q?', hint: '고르세요', multi: false, options: [{ label: '가' }] }] });

const requested = (id: number) =>
  frame(id, 'tool.requested', {
    agent: { id: 'agent_a', name: 'rubric' },
    tool: 'ask-user',
    toolCallId: 'call_1',
    input: ASK_INPUT,
  });

const called = (id: number) =>
  frame(id, 'tool.called', { agent: { id: 'agent_a', name: 'rubric' }, tool: 'ask-user', input: {}, ok: true });

const askReplay = (resolved: boolean) =>
  sse(
    frame(1, 'workflow.started', {}) +
      frame(2, 'step.started', { step: 'rubric-0' }) +
      requested(3) +
      (resolved ? called(4) : '') +
      frame(5, 'workflow.completed', {}),
  );

describe('applyDispositions', () => {
  const dispositions: ThreadDisposition[] = [
    { threadId: 's.1.0', verdict: 'resolved', comment: '반영을 확인했어요' },
    { threadId: 's.1.1', verdict: 'kept', comment: '여전히 걸려요' },
    { threadId: 's.1.2', verdict: 'withdrawn', comment: '제가 잘못 봤어요' },
  ];

  const seed = () => [threadRow('s.1.0'), threadRow('s.1.1', { issueIndex: 1, pass: 'stylistic' }), threadRow('s.1.2', { issueIndex: 2 })];

  it('열린 스레드를 전이하고 AI 코멘트를 결정적 id로 단다', async () => {
    const { db, threads, comments } = createStoreStub(seed());

    await applyDispositions(db, 's', 2, dispositions);

    expect(threads[0]).toMatchObject({ state: 'resolved' });
    expect(threads[0].stateChangedAt).toBeInstanceOf(Date);
    expect(threads[2]).toMatchObject({ state: 'withdrawn' });
    // kept는 열린 채로 남고 전이가 없다 — 앵커·회차 갱신은 승계 이슈(carriedIssues) 경로가 담당한다
    expect(threads[1]).toMatchObject({ state: 'open', stateChangedAt: null });
    expect(threads[1].anchors).toEqual(threadRow('s.1.1').anchors);
    expect(comments).toEqual([
      { id: 's.1.0.ai.2', threadId: 's.1.0', author: 'ai', body: '반영을 확인했어요', reviewRound: 2, createdAt: expect.any(Date) },
      { id: 's.1.1.ai.2', threadId: 's.1.1', author: 'ai', body: '여전히 걸려요', reviewRound: 2, createdAt: expect.any(Date) },
      { id: 's.1.2.ai.2', threadId: 's.1.2', author: 'ai', body: '제가 잘못 봤어요', reviewRound: 2, createdAt: expect.any(Date) },
    ]);
  });

  it('두 번 적용해도 상태·코멘트가 한 번 적용과 같다', async () => {
    const { db, threads, comments } = createStoreStub(seed());

    await applyDispositions(db, 's', 2, dispositions);
    const changedAt = threads[0].stateChangedAt;
    await applyDispositions(db, 's', 2, dispositions);

    expect(threads[0]).toMatchObject({ state: 'resolved' });
    // 전이가 open 조건부라 재적용은 닿지 않는다 — 닿았다면 전이 시각이 새 Date로 갈린다
    expect(threads[0].stateChangedAt).toBe(changedAt);
    expect(threads[1]).toMatchObject({ state: 'open' });
    expect(threads[2]).toMatchObject({ state: 'withdrawn' });
    expect(comments.map((comment) => comment.id)).toEqual(['s.1.0.ai.2', 's.1.1.ai.2', 's.1.2.ai.2']);
  });

  it('코멘트 없는 처분(null)은 댓글을 달지 않는다 — 새 답글 없이 이어지는 kept의 생략', async () => {
    const { db, threads, comments } = createStoreStub(seed());

    await applyDispositions(db, 's', 2, [{ threadId: 's.1.1', verdict: 'kept', comment: null }]);

    expect(threads[1]).toMatchObject({ state: 'open', stateChangedAt: null });
    expect(comments).toHaveLength(0);
  });

  it('이미 닫힌 스레드는 전이도 코멘트도 건너뛴다', async () => {
    const closedAt = new Date(1000);
    const { db, threads, comments } = createStoreStub([threadRow('s.1.0', { state: 'closed', stateChangedAt: closedAt })]);

    await applyDispositions(db, 's', 2, [dispositions[0]]);

    expect(threads[0]).toMatchObject({ state: 'closed', stateChangedAt: closedAt });
    expect(comments).toHaveLength(0);
  });

  it('다른 세션의 스레드는 건드리지 않는다', async () => {
    const { db, threads, comments } = createStoreStub([threadRow('s.1.0', { sessionId: 'other' })]);

    await applyDispositions(db, 's', 2, [dispositions[0]]);

    expect(threads[0]).toMatchObject({ state: 'open', stateChangedAt: null });
    expect(comments).toHaveLength(0);
  });
});

describe('projectIfTerminal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(openEvents).mockImplementation(() => Promise.resolve(replay()));
  });

  it('실행 중이면 사영하지 않고 running을 돌린다', async () => {
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'running', finishedAt: null }) });
    const { db, inserts, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('running');
    expect(inserts).toHaveLength(0);
    expect(updates).toHaveLength(0);
    expect(vi.mocked(openEvents)).not.toHaveBeenCalled();
  });

  it('완료면 thread 행을 넣고 리뷰를 종결로 갱신한다', async () => {
    const usage = { settled: true as const, complete: true, folds: [] };
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result, usage }) });
    const { db, inserts, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(inserts.map((i) => i.table)).toEqual([Threads, Threads]);
    expect(inserts.map((i) => i.row.id)).toEqual([threadId('s1', 1, 0), threadId('s1', 1, 1)]);
    expect(updates).toHaveLength(1);
    expect(updates[0].table).toBe(Reviews);
    // DB에는 판별자를 벗긴 형태만 굳는다
    expect(updates[0].values).toMatchObject({
      status: 'completed',
      result,
      usage: { complete: true, folds: [] },
      error: null,
      finishedAt: new Date(2000),
    });
    expect((updates[0].values.events as { event: string }[]).map((e) => e.event)).toEqual(['workflow.started', 'workflow.completed']);
  });

  it('종결 응답에 live(settled: false) usage가 와도 폴드를 버리지 않고 complete만 꺾는다', async () => {
    // 배포 겹침 창의 방어 분기 — 정상 경로는 아니지만 도달하면 회계 하한이 보여야 한다(무음 유실 금지)
    const folds = [
      {
        provider: 'anthropic',
        agent: 'plan',
        model: 'm',
        effort: null,
        turns: 1,
        inputTokens: 10,
        outputTokens: 5,
        cacheReadTokens: 0,
        cacheWriteTokens: 0,
        thinkingTokens: null,
      },
    ];
    const usage = { settled: false as const, folds };
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result, usage }) });
    const { db, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(updates[0].values.usage).toEqual({ complete: false, folds });
  });

  it('거부 종결(kind: rejected)은 thread 없이 행만 굳힌다', async () => {
    const rejection = {
      version: 1 as const,
      kind: 'rejected' as const,
      rejected: { category: 'diary' as const, message: '이 글은 개인적인 기록으로 분류되어, 피드백을 진행하지 않았습니다.', basis: '근거' },
    };
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result: rejection }) });
    const { db, inserts, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(inserts).toHaveLength(0);
    expect(updates).toHaveLength(1);
    // 결과는 그대로 굳는다 — 화면(거부 문면·admin basis)의 재료다
    expect(updates[0].values).toMatchObject({ status: 'completed', result: rejection });
  });

  it('실패면 thread를 넣지 않고 사유만 반영한다', async () => {
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'failed', error: '터졌다', finishedAt: null }) });
    const { db, inserts, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(inserts).toHaveLength(0);
    expect(updates).toHaveLength(1);
    expect(updates[0].values).toMatchObject({ status: 'failed', result: null, error: '터졌다' });
    expect(updates[0].values.finishedAt).toBeInstanceOf(Date);
  });

  it('사영이 질문 기록을 questions 컬럼으로 영속한다', async () => {
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result }) });
    vi.mocked(openEvents).mockImplementation(() => Promise.resolve(askReplay(true)));
    vi.mocked(getAgentAskCalls).mockResolvedValue([[{ question: 'Q?', choice: ['가'] }]]);
    const { db, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(updates[0].values.questions).toEqual([
      {
        agentName: 'rubric',
        toolCallId: 'call_1',
        stage: 'rubric',
        at: 1003,
        status: 'answered',
        questions: [{ question: 'Q?', hint: '고르세요', multi: false, options: [{ label: '가' }] }],
        answers: [{ question: 'Q?', choice: ['가'] }],
      },
    ]);
  });

  it('답을 못 받고 끝난 질문은 closed로 답변 없이 남긴다', async () => {
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'canceled', finishedAt: null }) });
    vi.mocked(openEvents).mockImplementation(() => Promise.resolve(askReplay(false)));
    const { db, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(updates[0].values.questions).toEqual([expect.objectContaining({ toolCallId: 'call_1', status: 'closed', answers: null })]);
    expect(vi.mocked(getAgentAskCalls)).not.toHaveBeenCalled();
  });

  it('질문 없는 실행은 questions가 null이다', async () => {
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result }) });
    const { db, updates } = createDbStub();

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(updates[0].values.questions).toBeNull();
    expect(vi.mocked(getAgentAskCalls)).not.toHaveBeenCalled();
  });

  it('재리뷰 결과의 처분을 승계 스레드에 사영한다', async () => {
    const dispositions: ThreadDisposition[] = [{ threadId: 's1.1.0', verdict: 'withdrawn', comment: '제가 잘못 봤어요' }];
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result: { ...result, dispositions } }) });
    const { db, threads, comments } = createStoreStub([threadRow('s1.1.0', { sessionId: 's1' })]);

    expect(await projectIfTerminal(db, env, { ...review, round: 2 })).toBe('projected');
    expect(threads.map((thread) => thread.id)).toEqual(['s1.1.0', threadId('s1', 2, 0), threadId('s1', 2, 1)]);
    expect(threads[0]).toMatchObject({ state: 'withdrawn' });
    expect(comments.map((comment) => comment.id)).toEqual(['s1.1.0.ai.2']);
  });

  it('처분 키가 없는 1회차 결과는 사영할 처분이 없다', async () => {
    vi.mocked(getWorkflow).mockResolvedValue({ workflow: workflow({ status: 'completed', result }) });
    const { db, threads, comments } = createStoreStub([threadRow('s1.1.0', { sessionId: 's1' })]);

    expect(await projectIfTerminal(db, env, review)).toBe('projected');
    expect(threads[0]).toMatchObject({ state: 'open', stateChangedAt: null });
    expect(comments).toHaveLength(0);
  });
});
