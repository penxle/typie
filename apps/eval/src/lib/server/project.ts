import { and, eq } from 'drizzle-orm';
import { initialLive } from '../feedback/live.ts';
import { createSseParser } from '../feedback/sse.ts';
import { Reviews, ThreadComments, Threads } from './db/index.ts';
import { fetchEventLog, getWorkflow, openEvents } from './prism.ts';
import { collectAskAnswers } from './questions.ts';
import type { SseEvent } from '../feedback/sse.ts';
import type { Anchor, FeedbackResult, Pass, ReviewQuestionRecord, ThreadDisposition } from '../feedback/types.ts';
import type { Db } from './db/index.ts';

type PrismEnv = { PRISM_API_ORIGIN: string; PRISM_API_TOKEN: string };
type ReviewRef = { sessionId: string; round: number; prismWorkflowId: string };

export type ThreadRow = {
  id: string;
  sessionId: string;
  reviewRound: number;
  issueIndex: number;
  issueId: string | null;
  trait: string;
  pass: Pass;
  body: string | null;
  anchors: Anchor[];
  state: 'open';
  stateChangedAt: null;
};

export const threadId = (sessionId: string, round: number, issueIndex: number): string => `${sessionId}.${round}.${issueIndex}`;

// thread 표지가 없는 이슈만 새 스레드가 된다 — 표지 이슈는 지난 회차 스레드의 계속이라 행을 만들지 않고
// carriedIssues의 갱신 경로를 탄다.
export const threadsFromResult = (sessionId: string, round: number, result: FeedbackResult): ThreadRow[] =>
  result.issues.flatMap((issue, index) =>
    issue.thread === undefined
      ? [
          {
            id: threadId(sessionId, round, index),
            sessionId,
            reviewRound: round,
            issueIndex: index,
            issueId: issue.id ?? null,
            trait: issue.trait,
            pass: issue.pass,
            body: issue.body,
            anchors: issue.anchors,
            state: 'open' as const,
            stateChangedAt: null,
          },
        ]
      : [],
  );

// 계속되는(kept) 지적 — 기존 스레드를 이번 회차 번호 공간에 다시 앉힐 좌표다. 앵커는 prism이 새 원고에서
// 확정한 값이라 재탐색하지 않는다.
export const carriedIssues = (result: FeedbackResult): { threadId: string; issueIndex: number; anchors: Anchor[] }[] =>
  result.issues.flatMap((issue, index) =>
    issue.thread === undefined ? [] : [{ threadId: issue.thread, issueIndex: index, anchors: issue.anchors }],
  );

// 재리뷰 처분을 승계 스레드에 사영한다. 전이·코멘트 모두 재적용에 안전해야 한다 — 사영은 종결 후 화면 로드가
// 부르고, 중간에 죽으면 다음 로드가 처음부터 다시 사영한다. 전이는 open 조건부라 두 번째 적용이 닿지 않고,
// 코멘트는 결정적 id + onConflictDoNothing으로 한 행에 머문다.
export const applyDispositions = async (db: Db, sessionId: string, round: number, dispositions: ThreadDisposition[]): Promise<void> => {
  for (const disposition of dispositions) {
    // 닫힌 스레드는 처분도 코멘트도 받지 않는다 — 잠금이 리뷰 중 닫기를 막으니 정상 경로에선 만날 일이 없고,
    // 이상 상태로 닫혀 있다면 테스터가 끝낸 대화 위에 뒤늦은 처분을 얹지 않는 쪽이 옳다.
    const [current] = await db
      .select({ state: Threads.state })
      .from(Threads)
      .where(and(eq(Threads.id, disposition.threadId), eq(Threads.sessionId, sessionId)))
      .limit(1);
    if (!current || current.state === 'closed') continue;

    const openOnly = and(eq(Threads.id, disposition.threadId), eq(Threads.sessionId, sessionId), eq(Threads.state, 'open'));
    // kept는 열린 채로 남고 전이가 없다 — 회차·번호·앵커 갱신은 승계 이슈(carriedIssues) 경로가 담당한다.
    if (disposition.verdict !== 'kept') {
      await db.update(Threads).set({ state: disposition.verdict, stateChangedAt: new Date() }).where(openOnly);
    }

    // 코멘트 없는 처분(새 답글 없이 이어지는 kept)은 스레드가 잇는 것으로 충분하다 — 빈 댓글 행을 만들지 않는다.
    if (!disposition.comment) continue;

    await db
      .insert(ThreadComments)
      .values({
        id: `${disposition.threadId}.ai.${round}`,
        threadId: disposition.threadId,
        author: 'ai',
        body: disposition.comment,
        reviewRound: round,
        createdAt: new Date(),
      })
      .onConflictDoNothing();
  }
};

// 종결 세션의 events는 재생 후 EOF다. 무한 스트림 방어로 프레임 간 45초 무수신이면 포기하고 던진다 —
// 사영 실패는 다음 화면 로드가 재시도한다.
const IDLE_LIMIT_MS = 45_000;

export const collectEvents = async (
  env: PrismEnv,
  prismWorkflowId: string,
  open: (env: PrismEnv, id: string, cursor: number) => Promise<Response> = openEvents,
): Promise<SseEvent[]> => {
  const res = await open(env, prismWorkflowId, 0);
  if (!res.body) throw new Error('event replay has no body');
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  const parser = createSseParser();
  const events: SseEvent[] = [];
  for (;;) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const idle = new Promise<'idle'>((resolve) => {
      timer = setTimeout(() => resolve('idle'), IDLE_LIMIT_MS);
    });
    const outcome = await Promise.race([reader.read(), idle]).finally(() => clearTimeout(timer));
    if (outcome === 'idle') {
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- swallow cancel rejection; we throw right below
      await reader.cancel().catch(() => {});
      throw new Error('event replay stalled');
    }
    if (outcome.done) break;
    // eslint-disable-next-line unicorn/no-return-array-push -- the parser's push() returns parsed events
    for (const event of parser.push(decoder.decode(outcome.value, { stream: true }))) {
      if (event.id !== null) events.push(event);
    }
  }
  return events;
};

// 실행 중 세션의 첫 화면 시드 — 이벤트 로그 스냅샷(JSON)을 클라이언트 SseEvent 형태로 옮긴다. data는
// 클라이언트 리듀서가 SSE data 라인과 같은 봉투({seq,kind,data,createdAt} 문자열)를 기대하므로 행 전체를
// 다시 직렬화한다(live.ts decode 참조). 시드 없이 재생을 화면에서 재연하면 새로고침마다 과거 기록이 전환과
// 함께 빨리감기로 보인다.
export const seedEvents = async (env: PrismEnv, prismWorkflowId: string): Promise<SseEvent[]> => {
  const rows = await fetchEventLog(env, prismWorkflowId);
  return rows.map((row) => {
    // thinking 전문은 리듀서가 읽지 않는 최대 중량 필드다(xhigh 추론이라 턴당 수 KB) — 시드에서 떨궈
    // 첫 로드 페이로드를 줄인다. 필드 형태는 남긴다(null): 소비자가 부재와 미지원을 구별할 일이 없게.
    const data = 'thinking' in row.data ? { ...row.data, thinking: null } : row.data;
    return { id: row.seq, event: row.kind, data: JSON.stringify({ ...row, data }) };
  });
};

// 멱등 순서: threads 먼저(onConflictDoNothing — 재시도 안전), reviews 조건부 갱신을 마지막에.
// 중간에 죽으면 review가 running으로 남아 다음 로드가 처음부터 다시 사영한다.
export const projectIfTerminal = async (db: Db, env: PrismEnv, review: ReviewRef): Promise<'running' | 'projected'> => {
  const { workflow } = await getWorkflow(env, review.prismWorkflowId);
  if (workflow.status === 'running') return 'running';

  const events = await collectEvents(env, review.prismWorkflowId);

  // 사영은 종결 후 1회라, 답변 문면(이벤트에 없고 prism 원장에만 있다)을 여기서 당겨 두면 원장 리텐션과 무관하게
  // 기록이 완결된다. 원장 조회 실패는 던진다 — 반쪽 기록을 굳히면 다시 채울 자리가 없고, 던지면 사영이 통째로
  // 미뤄져 호출부의 관용 처분(sessions/[id]/+page.server.ts의 load — projectIfTerminal try/catch)이 다음 로드에 재시도한다.
  const asked = initialLive(events).questions;
  let questions: ReviewQuestionRecord[] | null = null;
  if (asked.length > 0) {
    const answers = await collectAskAnswers(env, events);
    questions = asked.map((entry) => ({
      agentName: entry.agentName,
      toolCallId: entry.toolCallId,
      stage: entry.stage,
      at: entry.at,
      status: entry.status === 'answered' ? 'answered' : 'closed',
      questions: entry.questions,
      answers: answers[entry.toolCallId] ?? null,
    }));
  }

  // 거부 종결(kind: 'rejected')은 지적·처분이 없다 — 스레드를 만들지 않고 아래에서 행만 굳힌다.
  // 판별은 kind === 'rejected'로만 한다(구 결과에는 키가 없다 — 부재는 정상 결과).
  if (workflow.status === 'completed' && workflow.result && workflow.result.kind !== 'rejected') {
    const rows = threadsFromResult(review.sessionId, review.round, workflow.result);
    for (const row of rows) {
      await db.insert(Threads).values(row).onConflictDoNothing();
    }
    // 계속되는 스레드를 이번 회차 번호 공간에 앉힌다 — 갱신값이 결정적이라 재적용도 같은 값이다(멱등).
    for (const carried of carriedIssues(workflow.result)) {
      await db
        .update(Threads)
        .set({ reviewRound: review.round, issueIndex: carried.issueIndex, anchors: carried.anchors })
        .where(and(eq(Threads.id, carried.threadId), eq(Threads.sessionId, review.sessionId), eq(Threads.state, 'open')));
    }
    if (workflow.result.dispositions) {
      await applyDispositions(db, review.sessionId, review.round, workflow.result.dispositions);
    }
  }
  await db
    .update(Reviews)
    .set({
      status: workflow.status,
      // 판별자를 벗겨 굳힌다 — DB 행은 판별자 없는 단일 형태(RunUsage)다. 종결 응답에 live(settled: false)가
      // 오는 경우는 없지만, 배포 겹침 창에서 도착하더라도 폴드는 그대로 싣고 complete만 꺾는다 — live 폴드는
      // 회계의 하한이라 버리면 무음 유실이 되고, 남기면 최소한 하한이 보인다.
      usage:
        workflow.usage === null
          ? null
          : { complete: workflow.usage.settled ? workflow.usage.complete : false, folds: workflow.usage.folds },
      result: workflow.result,
      error: workflow.error,
      events,
      questions,
      finishedAt: workflow.finishedAt === null ? new Date() : new Date(workflow.finishedAt),
    })
    .where(and(eq(Reviews.sessionId, review.sessionId), eq(Reviews.round, review.round), eq(Reviews.status, 'running')));
  return 'projected';
};
