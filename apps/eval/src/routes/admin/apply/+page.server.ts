import { error } from '@sveltejs/kit';
import { desc } from 'drizzle-orm';
import { latestApplyIdForStage } from '$lib/server/apply.ts';
import { createDb, PromptApplies, PromptVariants } from '$lib/server/db/index.ts';
import { loadVariantStatusIndex } from '../lib/status.ts';
import type { StageKey, StagePrompt } from '$lib/domain/admin-types.ts';
import type { PageServerLoad } from './$types';

// internal-api.ts의 CurrentPrompts와 동일한 형태(stage → StagePrompt)지만, $lib/server/*는 클라이언트에서
// import할 수 없는 서버 전용 모듈이라 이 admin/apply 트리에서 공유하는 형태로 다시 선언한다.
type CurrentPrompts = Record<StageKey, StagePrompt>;

const STAGES: StageKey[] = ['summarize', 'meta', 'analyze'];

export const load: PageServerLoad = async ({ platform, locals, fetch, url }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  // draft/ran/adopted/applied 파생은 홈·후보 목록과 동일한 단일 소스(routes/admin/lib/status.ts)를 사용한다
  // (ran=succeeded run 기준, applied=status='applied' 최신 행 기준 — 이 페이지에서 별도로 다시 계산하지 않는다).
  const [variants, applies, statusIndex, latestApplyEntries] = await Promise.all([
    db.select().from(PromptVariants).orderBy(desc(PromptVariants.createdAt)),
    db.select().from(PromptApplies).orderBy(desc(PromptApplies.createdAt)).limit(200),
    loadVariantStatusIndex(db),
    Promise.all(STAGES.map(async (stage) => [stage, await latestApplyIdForStage(db, stage)] as const)),
  ]);

  const variantSummaries = variants.map((v) => ({ id: v.id, label: v.label, status: statusIndex.deriveStatus(v.id) }));
  const variantLabelById = new Map(variants.map((v) => [v.id, v.label]));

  // GET current를 self-fetch — U1/U2와 동일한 이유(hooks.server.ts 인증을 다시 통과해야 하므로 이메일 헤더 직접 전달).
  // 로컬에서 internal-api(api 앱)가 기동되어 있지 않으면 실패하는데, 이 경우 빈 diff를 보여주는 대신
  // currentError로 구조화된 안내 배너를 띄운다(브리프 명시 요구).
  let currentPrompts: CurrentPrompts | null = null;
  let currentError: string | null = null;
  try {
    const res = await fetch('/admin/api/prompts/current', { headers: { 'cf-access-authenticated-user-email': locals.email } });
    if (!res.ok) {
      throw new Error(`status ${res.status}`);
    }
    const body = (await res.json()) as { prompts: CurrentPrompts };
    currentPrompts = body.prompts;
  } catch (err) {
    currentError = String(err).slice(0, 200);
  }

  const selectedVariantId = url.searchParams.get('variantId');
  const selectedVariant = selectedVariantId ? (variants.find((v) => v.id === selectedVariantId) ?? null) : null;

  // 롤백은 (a) 적용이 실제로 성공한 행이면서 (b) 그 단계의 가장 최근 이력일 때만 허용한다(rollback 엔드포인트와
  // 동일 기준 — $lib/server/apply.ts의 latestApplyIdForStage/checkRollbackAllowed 공유).
  const latestIdByStage = new Map(latestApplyEntries);

  return {
    variantSummaries,
    selectedVariant: selectedVariant ? { id: selectedVariant.id, label: selectedVariant.label, content: selectedVariant.content } : null,
    currentPrompts,
    currentError,
    applies: applies.map((a) => {
      const isLatestForStage = latestIdByStage.get(a.appliedStage) === a.id;
      const rollbackBlockedReason =
        a.status === 'failed'
          ? '적용에 실패한 이력은 롤백할 수 없습니다.'
          : isLatestForStage
            ? null
            : '이 단계의 더 최근 이력이 있어 최신 이력에서만 롤백할 수 있습니다.';

      return {
        id: a.id,
        stage: a.appliedStage,
        variantLabel: variantLabelById.get(a.appliedVariantId) ?? a.appliedVariantId,
        status: a.status,
        appliedBy: a.appliedBy,
        createdAt: a.createdAt.toISOString(),
        prev: a.prev,
        rollbackBlockedReason,
      };
    }),
  };
};
