import { error, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
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

  // Phase 1은 세션당 리뷰가 round 1 하나뿐이다.
  const [review] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
  if (!review) error(404, 'not found');

  // 과정의 원천은 사영된 이벤트 스냅샷 하나뿐이다 — 아직 없으면 세션 화면이 사영을 돌린다.
  if (review.status === 'running') redirect(303, `/sessions/${params.id}`);
  const events = review.events as SseEvent[] | null;
  if (!events) redirect(303, `/sessions/${params.id}`);

  return {
    session: { id: session.id, title: session.title },
    review: {
      round: review.round,
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
