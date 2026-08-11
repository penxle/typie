import { eq } from 'drizzle-orm';
import { initialLive } from '../feedback/live.ts';
import { FeedbackSessions, PushLog, Reviews } from './db/index.ts';
import { createInternalApi } from './internal-api.ts';
import { getAgentPendingTool, getWorkflow, getWorkflowInvocations } from './prism.ts';
import { seedEvents } from './project.ts';
import type { AskQuestion } from '../feedback/live.ts';
import type { Db } from './db/index.ts';

export type PollEnv = {
  PRISM_API_ORIGIN: string;
  PRISM_API_TOKEN: string;
  INTERNAL_API_BASE: string;
  INTERNAL_API_KEY: string;
};

// 푸시 수신처는 typie 앱이지만 확인처는 eval 웹이다 — 모든 body 끝에 안내를 붙인다.
const CHECK_NOTICE = 'AI 피드백 베타 사이트에서 확인해주세요';

export const askBody = (questions: AskQuestion[]): string => {
  const first = questions[0]?.question ?? '';
  return questions.length > 1 ? `${first} 외 ${questions.length - 1}개` : first;
};

// 매분 폴 — 발송 성공(HTTP 2xx) 후에만 기록한다. 실패는 기록 없이 다음 분 재시도, 겹침 실행의
// 중복 발송은 수용(유실보다 중복 — 오너 확정). 사영(projectIfTerminal)은 페이지 로드 몫이라 건드리지 않는다.
export const pollAndPush = async (db: Db, env: PollEnv): Promise<string[]> => {
  const rows = await db
    .select({
      sessionId: Reviews.sessionId,
      round: Reviews.round,
      prismWorkflowId: Reviews.prismWorkflowId,
      refId: FeedbackSessions.refId,
      title: FeedbackSessions.title,
    })
    .from(Reviews)
    .innerJoin(FeedbackSessions, eq(FeedbackSessions.id, Reviews.sessionId))
    .where(eq(Reviews.status, 'running'));
  if (rows.length === 0) return [];

  const pushedRows = await db.select({ key: PushLog.key }).from(PushLog);
  const seen = new Set(pushedRows.map((row) => row.key));
  const api = createInternalApi(env.INTERNAL_API_BASE, env.INTERNAL_API_KEY);
  const pushed: string[] = [];

  const send = async (key: string, documentId: string, title: string, body: string) => {
    if (seen.has(key)) return;
    if (!(await api.sendPush(documentId, title, body))) return;
    await db.insert(PushLog).values({ key, pushedAt: new Date() }).onConflictDoNothing();
    seen.add(key);
    pushed.push(key);
  };

  for (const row of rows) {
    const title = row.title || '제목 없음';
    try {
      const { workflow } = await getWorkflow(env, row.prismWorkflowId);
      if (workflow.status === 'completed') {
        await send(`done:${row.sessionId}:${row.round}`, row.refId, `리뷰가 끝났어요 — ${title}`, `결과가 정리돼 있어요.\n${CHECK_NOTICE}`);
        continue;
      }
      if (workflow.status !== 'running') continue;

      for (const invocation of await getWorkflowInvocations(env, row.prismWorkflowId)) {
        if (invocation.status !== 'running') continue;
        const pending = await getAgentPendingTool(env, invocation.agentId);
        if (pending?.tool !== 'ask-user') continue;
        const key = `ask:${pending.toolCallId}`;
        if (seen.has(key)) continue;
        // 문면은 이벤트 로그에만 있다(getAgentPendingTool은 toolCallId뿐) — 미발송 신규 질문일 때만 당긴다.
        const entry = initialLive(await seedEvents(env, row.prismWorkflowId)).questions.find(
          (question) => question.toolCallId === pending.toolCallId && question.status === 'pending',
        );
        if (!entry) continue;
        await send(key, row.refId, `질문이 있어요 — ${title}`, `${askBody(entry.questions)}\n${CHECK_NOTICE}`);
      }
    } catch (err) {
      console.error(`push-poll ${row.sessionId}#${row.round}: ${String(err)}`);
    }
  }
  return pushed;
};
