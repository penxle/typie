import { error, fail, redirect } from '@sveltejs/kit';
import { and, desc, eq } from 'drizzle-orm';
import { resolveTierSubmission } from '$lib/feedback/tiers.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import { fetchManuscript } from '$lib/server/ingest.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import { hasPendingQuestion } from '$lib/server/prism.ts';
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
      tier: Reviews.tier,
      prismWorkflowId: Reviews.prismWorkflowId,
    })
    .from(FeedbackSessions)
    .innerJoin(Reviews, and(eq(Reviews.sessionId, FeedbackSessions.id), eq(Reviews.round, 1)))
    .where(eq(FeedbackSessions.testerEmail, locals.email))
    .orderBy(desc(FeedbackSessions.createdAt));

  // 진행 중 리뷰만 물어본다 — 종결한 리뷰에는 답할 수 있는 질문이 없다. prismWorkflowId는 조회에만 쓰고
  // 응답에는 싣지 않는다(명시 사영).
  // tier는 반대로 전원에게 싣는다 — 상세·과정 화면이 티어별 단계를 그리는 데 필수라 admin 게이트를 걸지 않는다.
  const withPending = await Promise.all(
    sessions.map(async (session) => {
      let pendingQuestion = false;
      if (session.status === 'running') {
        try {
          pendingQuestion = await hasPendingQuestion(platform.env, session.prismWorkflowId);
        } catch {
          // 배지는 안내일 뿐이다 — 조회 실패로 목록을 막지 않는다.
        }
      }
      return {
        id: session.id,
        refId: session.refId,
        title: session.title,
        createdAt: session.createdAt.getTime(),
        status: session.status,
        tier: session.tier,
        pendingQuestion,
      };
    }),
  );

  return { sessions: withPending, isAdmin: isAdmin(platform.env, locals.email) };
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

    // 에이전트 이름의 문자 집합은 여기서 판정하지 않는다 — 걷어서 넘기고, 미지 키는 resolveTierSubmission이
    // 명시 400으로 반려한다. 이름 형태로 거르면 새 이름이 무음으로 사라진다.
    const raw: Record<string, { model?: string; effort?: string }> = Object.create(null);
    for (const [key, value] of form.entries()) {
      const match = key.match(/^tier\.([^.]+)\.(model|effort)$/);
      if (!match) continue;
      (raw[match[1]] ??= {})[match[2] as 'model' | 'effort'] = String(value);
    }
    // 티어 미제출·빈 값은 high — 운영자만 쓰는 선택기라 테스터 폼에는 필드가 없다.
    const tier = String(form.get('tier') ?? '') || 'high';
    const submission = resolveTierSubmission(tier, raw, isAdmin(platform.env, locals.email));
    if ('error' in submission) return fail(400, { error: submission.error });

    const db = createDb(platform.env.DB);
    // 확인 단계의 반입을 재사용하지 않는다 — 확인과 시작 사이에 문서가 바뀌었으면 최신본이 리뷰 대상이다.
    let result;
    try {
      result = await startFeedbackSession(db, platform.env, {
        refId: documentId,
        email: locals.email,
        tier: submission.tier,
        overrides: submission.overrides,
      });
    } catch {
      return fail(502, { error: IMPORT_FAILED });
    }
    if ('error' in result) return fail(400, { error: result.error });

    // redirect는 throw다 — try 밖에 두어야 catch가 삼키지 않는다.
    redirect(303, `/sessions/${result.sessionId}`);
  },
};
