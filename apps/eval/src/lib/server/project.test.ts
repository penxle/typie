import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Reviews, Threads } from './db/index.ts';
import { getAgentAskCalls, getWorkflow, openEvents } from './prism.ts';
import { collectEvents, projectIfTerminal, threadId, threadsFromResult } from './project.ts';
import type { FeedbackResult, PrismWorkflow } from '../feedback/types.ts';
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
    { axis: '인물의 동기', pass: 'critique', body: '지적 문면', anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }] },
    { axis: '문장 결', pass: 'proofread', body: null, anchors: [] },
  ],
  conclusion: { understanding: null, strengths: [], clearances: [], patterns: [], priorities: [] },
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
        axis: '인물의 동기',
        pass: 'critique',
        body: '지적 문면',
        anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }],
        state: 'open',
        stateChangedAt: null,
      },
      expect.objectContaining({ id: threadId('s1', 1, 1), issueIndex: 1, body: null }),
    ]);
  });

  it('threadId는 결정적이다', () => {
    expect(threadId('s1', 1, 0)).toBe(threadId('s1', 1, 0));
    expect(threadId('s1', 1, 0)).not.toBe(threadId('s1', 2, 0));
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
    agent: { id: 'agent_a', name: 'plan' },
    tool: 'ask-user',
    toolCallId: 'call_1',
    input: ASK_INPUT,
  });

const called = (id: number) => frame(id, 'tool.called', { agent: { id: 'agent_a', name: 'plan' }, tool: 'ask-user', input: {}, ok: true });

const askReplay = (resolved: boolean) =>
  sse(
    frame(1, 'workflow.started', {}) +
      frame(2, 'step.started', { step: 'plan-0' }) +
      requested(3) +
      (resolved ? called(4) : '') +
      frame(5, 'workflow.completed', {}),
  );

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
        agentName: 'plan',
        toolCallId: 'call_1',
        stage: 'plan',
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
});
