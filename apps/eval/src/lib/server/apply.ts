import { desc, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDb, PromptApplies } from './db/index.ts';
import type { StageKey, StagePrompt } from '../domain/admin-types.ts';
import type { InternalApi } from './internal-api.ts';

type Db = ReturnType<typeof createDb>;

// admin/apply 로더의 목록 배지와 롤백 엔드포인트의 서버측 재검증이 공유하는 기준.
export const latestApplyIdForStage = async (db: Db, stage: StageKey): Promise<string | null> => {
  const [row] = await db
    .select({ id: PromptApplies.id })
    .from(PromptApplies)
    .where(eq(PromptApplies.appliedStage, stage))
    .orderBy(desc(PromptApplies.createdAt))
    .limit(1);
  return row?.id ?? null;
};

export type RollbackGuardResult = { ok: true } | { ok: false; reason: string };

export const checkRollbackAllowed = async (
  db: Db,
  apply: { id: string; appliedStage: StageKey; status: 'applied' | 'failed' },
): Promise<RollbackGuardResult> => {
  if (apply.status === 'failed') {
    return { ok: false, reason: '적용에 실패한 이력은 롤백할 수 없습니다.' };
  }

  const latestId = await latestApplyIdForStage(db, apply.appliedStage);
  if (latestId !== apply.id) {
    return { ok: false, reason: '이 단계의 더 최근 이력이 있어 최신 이력에서만 롤백할 수 있습니다.' };
  }

  return { ok: true };
};

export const performApply = async (
  db: Db,
  api: InternalApi,
  input: { appliedVariantId: string; stage: StageKey; content: StagePrompt; appliedBy: string },
): Promise<{ ok: boolean; applyId: string }> => {
  const current = await api.current();
  const prev = current[input.stage];

  const applyId = nanoid();
  let status: 'applied' | 'failed' = 'applied';
  try {
    await api.apply(input.stage, input.content);
  } catch (err) {
    console.warn(`prompt apply failed for stage ${input.stage}: ${String(err).slice(0, 200)}`);
    status = 'failed';
  }

  await db.insert(PromptApplies).values({
    id: applyId,
    promptId: api.stagePromptId(input.stage),
    prev,
    appliedVariantId: input.appliedVariantId,
    appliedStage: input.stage,
    appliedBy: input.appliedBy,
    status,
  });

  return { ok: status === 'applied', applyId };
};
