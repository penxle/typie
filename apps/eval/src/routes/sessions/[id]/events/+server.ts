import { error } from '@sveltejs/kit';
import { and, desc, eq } from 'drizzle-orm';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import { openEvents } from '$lib/server/prism.ts';
import { resolveCursor, watchdogPipe } from '$lib/server/relay.ts';
import type { RequestHandler } from './$types';

const IDLE_LIMIT_MS = 45_000;

export const GET: RequestHandler = async ({ params, locals, platform, request, url }) => {
  if (!platform) error(500, 'platform unavailable');

  const env = platform.env;
  const db = createDb(env.DB);
  const [session] = await db.select().from(FeedbackSessions).where(eq(FeedbackSessions.id, params.id)).limit(1);
  if (!session) error(404, 'not found');
  if (session.testerEmail !== locals.email && !isAdmin(env, locals.email)) error(403, 'forbidden');

  // 스트림이 겨누는 것은 지금 도는 회차다 — 세션에 running은 최대 하나뿐이라 회차를 따로 받지 않는다
  // (중단·답변 액션의 runningReview와 같은 잣대). 회차 조건 없이 집으면 재리뷰 회차에서 옛 회차를 물어 409가 된다.
  const [review] = await db
    .select()
    .from(Reviews)
    .where(and(eq(Reviews.sessionId, params.id), eq(Reviews.status, 'running')))
    .orderBy(desc(Reviews.round))
    .limit(1);
  if (!review) error(409, 'not running');

  const cursor = resolveCursor(request.headers.get('last-event-id'), url.searchParams.get('lastEventId'));
  const upstream = await openEvents(env, review.prismWorkflowId, cursor);
  if (!upstream.body) error(502, 'upstream body missing');

  return new Response(watchdogPipe(upstream.body, IDLE_LIMIT_MS), {
    headers: { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' },
  });
};
