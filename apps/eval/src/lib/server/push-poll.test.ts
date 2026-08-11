import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PushLog, Reviews } from './db/index.ts';
import { createInternalApi } from './internal-api.ts';
import { getAgentPendingTool, getWorkflow, getWorkflowInvocations } from './prism.ts';
import { seedEvents } from './project.ts';
import { askBody, pollAndPush } from './push-poll.ts';
import type { SseEvent } from '../feedback/sse.ts';
import type { Db } from './db/index.ts';

vi.mock('./prism.ts', () => ({
  getWorkflow: vi.fn(),
  getWorkflowInvocations: vi.fn(),
  getAgentPendingTool: vi.fn(),
}));
vi.mock('./project.ts', () => ({ seedEvents: vi.fn() }));
vi.mock('./internal-api.ts', () => ({ createInternalApi: vi.fn() }));

beforeEach(() => vi.clearAllMocks());

const env = {
  PRISM_API_ORIGIN: 'https://prism.test',
  PRISM_API_TOKEN: 'pk',
  INTERNAL_API_BASE: 'https://api.test',
  INTERNAL_API_KEY: 'ik',
};

type ReviewRow = { sessionId: string; round: number; prismWorkflowId: string; refId: string; title: string | null };

// 폴이 부르는 세 문장만 받는 스텁(project.test.ts의 createDbStub과 같은 형태) — from() 인자로 분기한다.
const createDbStub = (reviews: ReviewRow[], seenKeys: string[] = []) => {
  const inserted: string[] = [];
  const db = {
    select: () => ({
      from: (table: unknown) => {
        if (table === PushLog) return Promise.resolve(seenKeys.map((key) => ({ key })));
        if (table === Reviews) return { innerJoin: () => ({ where: () => Promise.resolve(reviews) }) };
        throw new Error('unexpected table');
      },
    }),
    insert: () => ({
      values: (row: { key: string }) => ({
        onConflictDoNothing: () => {
          inserted.push(row.key);
          return Promise.resolve();
        },
      }),
    }),
  };
  return { db: db as unknown as Db, inserted };
};

const pushMock = () => {
  const push = vi.fn().mockResolvedValue(true);
  vi.mocked(createInternalApi).mockReturnValue({ extract: vi.fn(), sendPush: push });
  return push;
};

const review = (over: Partial<ReviewRow> = {}): ReviewRow => ({
  sessionId: 'S1',
  round: 1,
  prismWorkflowId: 'ev-wf-1',
  refId: 'DOC1',
  title: '제목',
  ...over,
});

const workflow = (status: string) => ({ workflow: { status, error: null, startedAt: 0, finishedAt: null, result: null, usage: null } });

// live.test.ts의 asked() 픽스처와 같은 봉투 — data는 {seq,kind,data,createdAt} JSON 문자열이고
// ask-user input은 이중 JSON 문자열이다. 문항은 hint·multi·options까지 전부 있어야 파싱된다(live.ts parseAskUser).
const askEvents = (toolCallId: string, texts: string[]): SseEvent[] => {
  const questions = texts.map((question) => ({ question, hint: '', multi: false, options: [] }));
  return [
    { id: 1, event: 'workflow.started', data: JSON.stringify({ seq: 1, kind: 'workflow.started', data: {}, createdAt: 0 }) },
    { id: 2, event: 'step.started', data: JSON.stringify({ seq: 2, kind: 'step.started', data: { step: 'plan-0' }, createdAt: 1 }) },
    {
      id: 3,
      event: 'tool.requested',
      data: JSON.stringify({
        seq: 3,
        kind: 'tool.requested',
        data: {
          agent: { id: 'agent-a', name: 'plan' },
          turn: 1,
          attempt: 1,
          tool: 'ask-user',
          toolCallId,
          input: JSON.stringify({ questions }),
        },
        createdAt: 2,
      }),
    },
  ];
};

describe('askBody', () => {
  it('한 문항은 문항 그대로, 다문항은 외 N개', () => {
    expect(askBody([{ question: '결말은 의도인가요?' }] as never)).toBe('결말은 의도인가요?');
    expect(askBody([{ question: 'A' }, { question: 'B' }, { question: 'C' }] as never)).toBe('A 외 2개');
  });
});

describe('pollAndPush', () => {
  it('running 리뷰가 없으면 prism을 부르지 않는다', async () => {
    const { db } = createDbStub([]);
    expect(await pollAndPush(db, env)).toEqual([]);
    expect(vi.mocked(getWorkflow)).not.toHaveBeenCalled();
  });

  it('completed 리뷰는 done 키로 발송하고 기록한다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('completed') as never);
    const { db, inserted } = createDbStub([review()]);

    expect(await pollAndPush(db, env)).toEqual(['done:S1:1']);
    expect(push).toHaveBeenCalledWith('DOC1', '리뷰가 끝났어요 — 제목', '결과가 정리돼 있어요.\nAI 피드백 베타 사이트에서 확인해주세요');
    expect(inserted).toEqual(['done:S1:1']);
  });

  it('이미 기록된 done 키는 재발송하지 않는다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('completed') as never);
    const { db, inserted } = createDbStub([review()], ['done:S1:1']);

    expect(await pollAndPush(db, env)).toEqual([]);
    expect(push).not.toHaveBeenCalled();
    expect(inserted).toEqual([]);
  });

  it('pending ask-user는 로그에서 문면을 찾아 발송한다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('running') as never);
    vi.mocked(getWorkflowInvocations).mockResolvedValue([{ agentId: 'agent-a', agentName: 'plan', status: 'running' }]);
    vi.mocked(getAgentPendingTool).mockResolvedValue({ toolCallId: 'call_1', tool: 'ask-user' });
    vi.mocked(seedEvents).mockResolvedValue(askEvents('call_1', ['결말은 의도인가요?']));
    const { db, inserted } = createDbStub([review({ title: null })]);

    expect(await pollAndPush(db, env)).toEqual(['ask:call_1']);
    expect(push).toHaveBeenCalledWith('DOC1', '질문이 있어요 — 제목 없음', '결말은 의도인가요?\nAI 피드백 베타 사이트에서 확인해주세요');
    expect(inserted).toEqual(['ask:call_1']);
  });

  it('로그에 아직 없는 pending은 건너뛴다 — 다음 분 재시도', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('running') as never);
    vi.mocked(getWorkflowInvocations).mockResolvedValue([{ agentId: 'agent-a', agentName: 'plan', status: 'running' }]);
    vi.mocked(getAgentPendingTool).mockResolvedValue({ toolCallId: 'call_9', tool: 'ask-user' });
    vi.mocked(seedEvents).mockResolvedValue(askEvents('call_1', ['Q']));
    const { db, inserted } = createDbStub([review()]);

    expect(await pollAndPush(db, env)).toEqual([]);
    expect(push).not.toHaveBeenCalled();
    expect(inserted).toEqual([]);
  });

  it('발송 실패(false)는 기록하지 않는다 — at-least-once', async () => {
    const push = vi.fn().mockResolvedValue(false);
    vi.mocked(createInternalApi).mockReturnValue({ extract: vi.fn(), sendPush: push });
    vi.mocked(getWorkflow).mockResolvedValue(workflow('completed') as never);
    const { db, inserted } = createDbStub([review()]);

    expect(await pollAndPush(db, env)).toEqual([]);
    expect(inserted).toEqual([]);
  });

  it('한 리뷰의 prism 오류는 다른 리뷰를 막지 않는다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow)
      .mockRejectedValueOnce(new Error('prism down'))
      .mockResolvedValueOnce(workflow('completed') as never);
    const { db } = createDbStub([review(), review({ sessionId: 'S2', prismWorkflowId: 'ev-wf-2', refId: 'DOC2' })]);

    expect(await pollAndPush(db, env)).toEqual(['done:S2:1']);
    expect(push).toHaveBeenCalledWith('DOC2', '리뷰가 끝났어요 — 제목', '결과가 정리돼 있어요.\nAI 피드백 베타 사이트에서 확인해주세요');
  });

  it('failed 리뷰는 침묵한다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('failed') as never);
    const { db, inserted } = createDbStub([review()]);

    expect(await pollAndPush(db, env)).toEqual([]);
    expect(push).not.toHaveBeenCalled();
    expect(inserted).toEqual([]);
  });
});
