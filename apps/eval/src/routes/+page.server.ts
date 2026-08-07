import { error, fail, redirect } from '@sveltejs/kit';
import { and, desc, eq } from 'drizzle-orm';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import { fetchManuscript } from '$lib/server/ingest.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import { countChars, startFeedbackSession } from '$lib/server/reviews.ts';
import type { Actions, PageServerLoad } from './$types';

const IMPORT_FAILED = '문서를 불러오지 못했어요. 잠시 후 다시 시도해 주세요';

export const load: PageServerLoad = async ({ locals, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  // 세션과 round 1 리뷰는 한 batch(암묵 트랜잭션)로 함께 들어간다 — 리뷰 없는 세션은 없으므로 inner join이 목록을 잃지 않는다.
  const sessions = await db
    .select({
      id: FeedbackSessions.id,
      refId: FeedbackSessions.refId,
      title: FeedbackSessions.title,
      createdAt: FeedbackSessions.createdAt,
      status: Reviews.status,
    })
    .from(FeedbackSessions)
    .innerJoin(Reviews, and(eq(Reviews.sessionId, FeedbackSessions.id), eq(Reviews.round, 1)))
    .where(eq(FeedbackSessions.testerEmail, locals.email))
    .orderBy(desc(FeedbackSessions.createdAt));

  return { sessions: sessions.map((session) => ({ ...session, createdAt: session.createdAt.getTime() })) };
};

export const actions: Actions = {
  // 확인 단계 — 반입만 하고 아무 행도 쓰지 않는다. 오입력은 여기서 제목으로 걸러진다.
  preview: async ({ platform, request }) => {
    if (!platform) error(500, 'platform unavailable');

    const form = await request.formData();
    const documentId = String(form.get('documentId') ?? '').trim();
    if (!documentId) return fail(400, { error: '문서 ID를 입력해 주세요' });

    const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);
    let manuscript;
    try {
      manuscript = await fetchManuscript(api, documentId);
    } catch {
      return fail(502, { error: IMPORT_FAILED });
    }
    if ('error' in manuscript) return fail(400, { error: manuscript.error });

    // 글자 수는 이 반입본 기준이다 — 시작은 문서를 다시 반입하므로 그 사이 편집분은 반영되지 않는다.
    return { preview: { refId: documentId, title: manuscript.title, charCount: countChars(manuscript.content) } };
  },

  start: async ({ locals, platform, request }) => {
    if (!platform) error(500, 'platform unavailable');

    const form = await request.formData();
    const documentId = String(form.get('documentId') ?? '').trim();
    if (!documentId) return fail(400, { error: '문서 ID를 입력해 주세요' });

    const db = createDb(platform.env.DB);
    // 확인 단계의 반입을 재사용하지 않는다 — 확인과 시작 사이에 문서가 바뀌었으면 최신본이 리뷰 대상이다.
    let result;
    try {
      result = await startFeedbackSession(db, platform.env, { refId: documentId, email: locals.email });
    } catch {
      return fail(502, { error: IMPORT_FAILED });
    }
    if ('error' in result) return fail(400, { error: result.error });

    // redirect는 throw다 — try 밖에 두어야 catch가 삼키지 않는다.
    redirect(303, `/sessions/${result.sessionId}`);
  },
};
