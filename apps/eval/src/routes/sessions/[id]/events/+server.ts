import { error } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
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

  // Phase 1은 세션당 리뷰가 round 1 하나뿐이라 round 조건 없이도 유일하다.
  const [review] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
  if (!review || review.status !== 'running') error(409, 'not running');

  const cursor = resolveCursor(request.headers.get('last-event-id'), url.searchParams.get('lastEventId'));
  const upstream = await openEvents(env, review.prismSessionId, cursor);
  if (!upstream.body) error(502, 'upstream body missing');

  return new Response(watchdogPipe(upstream.body, IDLE_LIMIT_MS), {
    headers: { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' },
  });
};
