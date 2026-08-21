import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PushLog, Reviews } from './db/index.ts';
import { createInternalApi } from './internal-api.ts';
import { getAgentPendingTool, getWorkflow, getWorkflowInvocations } from './prism.ts';
import { askBody, pollAndPush } from './push-poll.ts';
import type { Db } from './db/index.ts';

vi.mock('./prism.ts', () => ({
  getWorkflow: vi.fn(),
  getWorkflowInvocations: vi.fn(),
  getAgentPendingTool: vi.fn(),
}));
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

// get 뷰의 pending.data는 tool.requested.data와 같은 값이다(prism docs/events.md §7) — 문항은 hint·multi·options까지
// 전부 있어야 파싱된다(live.ts parseAskQuestions).
const pendingAsk = (toolCallId: string, texts: string[]) => ({
  toolCallId,
  tool: 'ask-user',
  data: { questions: texts.map((question) => ({ question, hint: '', multi: false, options: [] })) },
});

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

  it('pending ask-user는 get 뷰의 문면으로 발송한다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('running') as never);
    vi.mocked(getWorkflowInvocations).mockResolvedValue([{ agentId: 'agent-a', agentName: 'plan', status: 'running' }]);
    vi.mocked(getAgentPendingTool).mockResolvedValue(pendingAsk('call_1', ['결말은 의도인가요?']));
    const { db, inserted } = createDbStub([review({ title: null })]);

    expect(await pollAndPush(db, env)).toEqual(['ask:call_1']);
    expect(push).toHaveBeenCalledWith('DOC1', '질문이 있어요 — 제목 없음', '결말은 의도인가요?\nAI 피드백 베타 사이트에서 확인해주세요');
    expect(inserted).toEqual(['ask:call_1']);
  });

  it('이미 기록된 ask 키는 재발송하지 않는다', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('running') as never);
    vi.mocked(getWorkflowInvocations).mockResolvedValue([{ agentId: 'agent-a', agentName: 'plan', status: 'running' }]);
    vi.mocked(getAgentPendingTool).mockResolvedValue(pendingAsk('call_1', ['Q']));
    const { db, inserted } = createDbStub([review()], ['ask:call_1']);

    expect(await pollAndPush(db, env)).toEqual([]);
    expect(push).not.toHaveBeenCalled();
    expect(inserted).toEqual([]);
  });

  it('문면을 읽을 수 없는 pending은 건너뛴다 — 다음 분 재시도', async () => {
    const push = pushMock();
    vi.mocked(getWorkflow).mockResolvedValue(workflow('running') as never);
    vi.mocked(getWorkflowInvocations).mockResolvedValue([{ agentId: 'agent-a', agentName: 'plan', status: 'running' }]);
    vi.mocked(getAgentPendingTool).mockResolvedValue({ toolCallId: 'call_9', tool: 'ask-user', data: null });
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
