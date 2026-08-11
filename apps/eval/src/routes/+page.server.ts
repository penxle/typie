import { error, fail, redirect } from '@sveltejs/kit';
import { asc, desc, eq, sql } from 'drizzle-orm';
import { pickRounds } from '$lib/feedback/rounds.ts';
import { resolveTierSubmission } from '$lib/feedback/tiers.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import { fetchManuscript } from '$lib/server/ingest.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import { fetchCatalog, hasPendingQuestion } from '$lib/server/prism.ts';
import { countChars, startFeedbackSession } from '$lib/server/reviews.ts';
import type { AppCatalog } from '$lib/feedback/tiers.ts';
import type { Actions, PageServerLoad } from './$types';

const IMPORT_FAILED = '문서를 불러오지 못했어요. 잠시 후 다시 시도해 주세요';

export const load: PageServerLoad = async ({ locals, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  // 세션과 첫 리뷰는 한 batch(암묵 트랜잭션)로 함께 들어간다 — 리뷰 없는 세션은 없으므로 inner join이 목록을 잃지 않는다.
  // 회차는 전량을 가져와 세션마다 표시 회차를 고른다(pickRounds — 세션 화면과 같은 규칙). 재리뷰가 도는 세션이
  // 지난 회차의 완료로 보이면 안 된다. 정렬은 세션 묶음이 최신순으로 이어지고 묶음 안이 회차 오름차순이 되게 준다
  // (pickRounds가 회차 오름차순 목록을 전제한다).
  const rows = await db
    .select({
      id: FeedbackSessions.id,
      refId: FeedbackSessions.refId,
      title: FeedbackSessions.title,
      createdAt: FeedbackSessions.createdAt,
      round: Reviews.round,
      status: Reviews.status,
      tier: Reviews.tier,
      prismWorkflowId: Reviews.prismWorkflowId,
      // 거부 표지만 뽑는다 — 목록에 회차 전량의 result 전문을 실어 오면 무겁다.
      rejectedKind: sql<string | null>`json_extract(${Reviews.result}, '$.kind')`,
    })
    .from(FeedbackSessions)
    .innerJoin(Reviews, eq(Reviews.sessionId, FeedbackSessions.id))
    .where(eq(FeedbackSessions.testerEmail, locals.email))
    .orderBy(desc(FeedbackSessions.createdAt), asc(Reviews.round));

  // 세션별 묶음 — Map의 삽입 순서가 곧 목록 순서다. 행이 있는 세션만 묶이므로 빈 묶음은 생기지 않는다.
  const grouped = new Map<string, ((typeof rows)[number] & { rejected: boolean })[]>();
  for (const raw of rows) {
    const row = { ...raw, rejected: raw.rejectedKind === 'rejected' };
    const group = grouped.get(row.id);
    if (group) group.push(row);
    else grouped.set(row.id, [row]);
  }

  const sessions = [...grouped.values()].map((group) => {
    const row = group[0];
    const { display } = pickRounds(group);
    return {
      id: row.id,
      refId: row.refId,
      title: row.title,
      createdAt: row.createdAt,
      status: display.status,
      tier: display.tier,
      prismWorkflowId: display.prismWorkflowId,
    };
  });

  // 진행 중 리뷰만 물어본다 — 종결한 리뷰에는 답할 수 있는 질문이 없다. 겨누는 것은 표시 회차의 워크플로다.
  // prismWorkflowId는 조회에만 쓰고 응답에는 싣지 않는다(명시 사영).
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

  const admin = isAdmin(platform.env, locals.email);

  // 티어 설정 폼의 옵션 재료 — admin에게만 걷는다(테스터 폼에는 티어 필드 자체가 없다). 실패는 목록 열람을
  // 막지 않고 null로 눕는다 — 화면은 티어 설정 상자를 접고, 시작 자체는 액션의 재조회가 다시 판정한다.
  let catalog: AppCatalog | null = null;
  if (admin) {
    try {
      catalog = await fetchCatalog(platform.env);
    } catch {
      catalog = null;
    }
  }

  return { sessions: withPending, isAdmin: admin, catalog };
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

    // 제출 검증과 스냅샷 조립의 재료를 한 번에 걷는다 — 폴백 없음(오너 결정): 못 걷으면 시작만 명시로 막힌다.
    let catalog;
    try {
      catalog = await fetchCatalog(platform.env);
    } catch {
      return fail(502, { error: '리뷰 구성을 불러오지 못했어요. 잠시 후 다시 시도해 주세요' });
    }
    const submission = resolveTierSubmission(catalog, tier, raw, isAdmin(platform.env, locals.email));
    if ('error' in submission) return fail(400, { error: submission.error });

    const db = createDb(platform.env.DB);
    // 확인 단계의 반입을 재사용하지 않는다 — 확인과 시작 사이에 문서가 바뀌었으면 최신본이 리뷰 대상이다.
    let result;
    try {
      result = await startFeedbackSession(db, platform.env, {
        refId: documentId,
        email: locals.email,
        catalog,
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
