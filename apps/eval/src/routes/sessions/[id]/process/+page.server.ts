import { error, redirect } from '@sveltejs/kit';
import { asc, eq } from 'drizzle-orm';
import { displayRoundNumbers, isRejectedResult, pickRounds } from '$lib/feedback/rounds.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import type { SseEvent } from '$lib/feedback/sse.ts';
import type { ReviewQuestionRecord } from '$lib/feedback/types.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const env = platform.env;
  const db = createDb(env.DB);

  const [session] = await db.select().from(FeedbackSessions).where(eq(FeedbackSessions.id, params.id)).limit(1);
  if (!session) error(404, 'not found');
  if (session.testerEmail !== locals.email && !isAdmin(env, locals.email)) error(403, 'forbidden');

  // 과정 보기가 여는 회차는 결과 화면이 여는 회차다 — 표시 회차 규칙은 pickRounds 한 곳이 정한다.
  const rows = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).orderBy(asc(Reviews.round));
  if (rows.length === 0) error(404, 'not found');
  const rounds = rows.map((row) => ({ ...row, rejected: isRejectedResult(row.result) }));
  const review = pickRounds(rounds).display;

  // 과정의 원천은 사영된 이벤트 스냅샷 하나뿐이다 — 아직 없으면 세션 화면이 사영을 돌린다.
  if (review.status === 'running') redirect(303, `/sessions/${params.id}`);
  const events = review.events as SseEvent[] | null;
  if (!events) redirect(303, `/sessions/${params.id}`);

  return {
    session: { id: session.id, title: session.title },
    review: {
      round: review.round,
      // 표시 회차 서수 — 실패·중단을 건너뛴 번호. 표시 회차가 실패인 경우는 1회차부터 실패한 세션뿐이라
      // 내부 번호가 곧 서수다(세션 화면과 같은 폴백).
      roundNumber:
        review.status === 'failed' || review.status === 'canceled' || review.rejected
          ? review.round
          : displayRoundNumbers(rounds)[review.round],
      status: review.status,
      tier: review.tier,
      startedAt: review.startedAt.getTime(),
      finishedAt: review.finishedAt?.getTime() ?? null,
      error: review.error,
      events,
      // 카드는 events 재생이 세운다 — 기록은 재생으로 복원할 수 없는 답변 문면만 공급한다(project.ts).
      questions: review.questions as ReviewQuestionRecord[] | null,
    },
  };
};
