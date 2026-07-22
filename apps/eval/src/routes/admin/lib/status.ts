import { and, desc, eq, inArray } from 'drizzle-orm';
import { createDb, FeedbackSets, PipelineRuns, PromptApplies, Tasks, Variants } from '$lib/server/db/index.ts';
import type { StageKey } from '$lib/domain/admin-types.ts';
import type { DerivedVariantStatus } from '../VariantStatusBadge.svelte';

type Db = ReturnType<typeof createDb>;

const STAGES: StageKey[] = ['summarize', 'meta', 'analyze'];

export type CurrentStageInfo = { stage: StageKey; appliedVariantId: string | null; appliedAt: string | null };

export type VariantStatusIndex = {
  deriveStatus: (promptVariantId: string) => DerivedVariantStatus;
  currentByStage: CurrentStageInfo[];
};

// 홈·목록 페이지가 draft/ran/adopted/applied를 동일하게 계산하도록 단일화한 헬퍼.
// PromptVariants.status 컬럼은 서버 어디서도 쓰이지 않으므로(B4 확인) 실제 사용 신호로부터 매번 다시 계산한다.
export const loadVariantStatusIndex = async (db: Db): Promise<VariantStatusIndex> => {
  const legacyVariants = await db.select({ id: Variants.id, promptVariantId: Variants.promptVariantId }).from(Variants);
  const legacyIds = legacyVariants.map((v) => v.id);

  // ran: legacy Variants(promptVariantId 연결)가 만든 pipeline run 중 최소 하나가 성공했는가.
  // GET /admin/api/runs(.limit(100))에 의존하지 않도록 PipelineRuns를 variantId로 직접, 무제한 조회한다.
  const succeededRuns =
    legacyIds.length > 0
      ? await db
          .select({ variantId: PipelineRuns.variantId })
          .from(PipelineRuns)
          .where(and(eq(PipelineRuns.kind, 'pipeline'), eq(PipelineRuns.status, 'succeeded'), inArray(PipelineRuns.variantId, legacyIds)))
      : [];
  const succeededLegacyIds = new Set(succeededRuns.map((r) => r.variantId as string));
  const ranPromptVariantIds = new Set(
    legacyVariants.filter((v) => v.promptVariantId && succeededLegacyIds.has(v.id)).map((v) => v.promptVariantId as string),
  );

  // adopted: 그 legacy Variants가 만든 feedback set이 실제 평가 라운드(Task)에 편입되었는가.
  const tasks = await db.select({ setIds: Tasks.setIds }).from(Tasks);
  const usedSetIds = new Set(tasks.flatMap((t) => t.setIds));
  const feedbackSets =
    usedSetIds.size > 0 ? await db.select({ id: FeedbackSets.id, variantId: FeedbackSets.variantId }).from(FeedbackSets) : [];
  const adoptedLegacyIds = new Set(feedbackSets.filter((f) => usedSetIds.has(f.id)).map((f) => f.variantId));
  const adoptedPromptVariantIds = new Set(
    legacyVariants.filter((v) => v.promptVariantId && adoptedLegacyIds.has(v.id)).map((v) => v.promptVariantId as string),
  );

  // applied: stage별로 status==='applied'인 "가장 최근" 행만 채택한다 — 그 이후 재시도가 실패했더라도
  // 실제 서버에 적용된 상태(라이브 상태)는 그 성공한 시점 그대로이므로 실패 시도가 이를 가려서는 안 된다.
  const applies = await db.select().from(PromptApplies).orderBy(desc(PromptApplies.createdAt)).limit(200);
  const latestAppliedByStage = new Map<StageKey, (typeof applies)[number]>();
  for (const apply of applies) {
    if (apply.status === 'applied' && !latestAppliedByStage.has(apply.appliedStage)) {
      latestAppliedByStage.set(apply.appliedStage, apply);
    }
  }
  const appliedPromptVariantIds = new Set([...latestAppliedByStage.values()].map((a) => a.appliedVariantId));

  const currentByStage: CurrentStageInfo[] = STAGES.map((stage) => {
    const apply = latestAppliedByStage.get(stage);
    return { stage, appliedVariantId: apply?.appliedVariantId ?? null, appliedAt: apply ? apply.createdAt.toISOString() : null };
  });

  const deriveStatus = (promptVariantId: string): DerivedVariantStatus => {
    if (appliedPromptVariantIds.has(promptVariantId)) return 'applied';
    if (adoptedPromptVariantIds.has(promptVariantId)) return 'adopted';
    if (ranPromptVariantIds.has(promptVariantId)) return 'ran';
    return 'draft';
  };

  return { deriveStatus, currentByStage };
};
