import { error, fail } from '@sveltejs/kit';
import { and, desc, eq, inArray, sql } from 'drizzle-orm';
import { createDb, Documents, Judgments, PipelineRuns, Rounds, Tasks, Variants } from '$lib/server/db/index.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const corpusVersionRows = await db
    .select({ corpusVersion: Documents.corpusVersion })
    .from(Documents)
    .groupBy(Documents.corpusVersion)
    .orderBy(sql`max(${Documents.createdAt}) desc`);
  const corpusVersions = corpusVersionRows.map((r) => r.corpusVersion);

  // 대상 variant 자동 제안 = 해당 코퍼스 버전에 succeeded 실행이 있는 legacy Variants 라벨.
  // admin/api/corpus/rounds가 variantLabels/v0Label/candidateLabel을 이 테이블의 label로 조회하므로
  // (resolveLabelSets: eq(Variants.label, label)) 여기서도 동일하게 legacy Variants를 기준으로 삼는다.
  const succeededRuns = await db
    .select({ corpusVersion: PipelineRuns.corpusVersion, variantId: PipelineRuns.variantId })
    .from(PipelineRuns)
    .where(and(inArray(PipelineRuns.kind, ['pipeline', 'analysis']), eq(PipelineRuns.status, 'succeeded')));

  const succeededVariantIds = [...new Set(succeededRuns.map((r) => r.variantId).filter((id): id is string => id !== null))];
  const succeededVariants =
    succeededVariantIds.length > 0
      ? await db.select({ id: Variants.id, label: Variants.label }).from(Variants).where(inArray(Variants.id, succeededVariantIds))
      : [];
  const labelById = new Map(succeededVariants.map((v) => [v.id, v.label]));

  const labelsByCorpusVersion: Record<string, string[]> = {};
  for (const run of succeededRuns) {
    if (!run.variantId) continue;
    const label = labelById.get(run.variantId);
    if (!label) continue;
    const list = (labelsByCorpusVersion[run.corpusVersion] ??= []);
    if (!list.includes(label)) list.push(label);
  }
  for (const list of Object.values(labelsByCorpusVersion)) list.sort((a, b) => a.localeCompare(b));

  const rounds = await db.select().from(Rounds).orderBy(desc(Rounds.createdAt));

  const taskCounts = await db
    .select({ roundId: Tasks.roundId, count: sql<number>`count(*)` })
    .from(Tasks)
    .groupBy(Tasks.roundId);
  const judgmentCounts = await db
    .select({ roundId: Tasks.roundId, count: sql<number>`count(*)` })
    .from(Judgments)
    .innerJoin(Tasks, eq(Judgments.taskId, Tasks.id))
    .groupBy(Tasks.roundId);

  const taskCountByRound = new Map(taskCounts.map((t) => [t.roundId, t.count]));
  const judgmentCountByRound = new Map(judgmentCounts.map((j) => [j.roundId, j.count]));

  const roundSummaries = rounds.map((r) => ({
    id: r.id,
    stage: r.stage,
    config: r.config,
    createdAt: r.createdAt.toISOString(),
    taskCount: taskCountByRound.get(r.id) ?? 0,
    judgmentCount: judgmentCountByRound.get(r.id) ?? 0,
  }));

  return { corpusVersions, labelsByCorpusVersion, rounds: roundSummaries };
};

export const actions: Actions = {
  // admin/api에는 라운드 무효화(태스크 삭제) 라우트가 없어 페이지 서버 action에서 직접 D1 delete를 수행한다.
  // 판정(Judgments)이 하나라도 존재하는 라운드는 평가 이력 보존을 위해 삭제를 거부한다.
  invalidate: async ({ request, platform }) => {
    if (!platform) {
      return fail(500, { error: 'platform unavailable' });
    }

    const form = await request.formData();
    const roundId = form.get('roundId');
    if (typeof roundId !== 'string' || roundId.length === 0) {
      return fail(400, { error: 'roundId가 필요합니다.' });
    }

    const db = createDb(platform.env.DB);

    const tasks = await db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, roundId));
    if (tasks.length === 0) {
      return fail(400, { error: '무효화할 태스크가 없습니다.' });
    }

    const [judgmentCount] = await db
      .select({ count: sql<number>`count(*)` })
      .from(Judgments)
      .where(inArray(Judgments.taskId, db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, roundId))));
    if ((judgmentCount?.count ?? 0) > 0) {
      return fail(409, { error: '판정이 존재하는 라운드는 무효화할 수 없습니다.' });
    }

    await db.delete(Tasks).where(eq(Tasks.roundId, roundId));
    return { success: true };
  },
};
