import { error, fail } from '@sveltejs/kit';
import { desc, eq, inArray } from 'drizzle-orm';
import { createRound, roundedDocumentIds, setRoundActive } from '$lib/server/rounds.ts';
import { createDb, Documents, inChunks, Judgments, PromptSets, Rounds, Runs, TaskReleases, Tasks } from '../../../../core/db.ts';
import { evaluationById, GENERATIONS, qualifiedEvaluationId } from '../../../../core/registry.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const rounds = await db.select().from(Rounds).orderBy(desc(Rounds.createdAt));
  const tasks = await db.select({ id: Tasks.id, roundId: Tasks.roundId }).from(Tasks);
  const taskIds = tasks.map((t) => t.id);
  const judgments = await inChunks(taskIds, (chunk) =>
    db.select({ taskId: Judgments.taskId, draft: Judgments.draft }).from(Judgments).where(inArray(Judgments.taskId, chunk)),
  );
  const roundOf = new Map(tasks.map((t) => [t.id, t.roundId]));
  const doneBy = new Map<string, number>();
  for (const judgment of judgments) {
    if (judgment.draft) continue;
    const roundId = roundOf.get(judgment.taskId);
    if (roundId) doneBy.set(roundId, (doneBy.get(roundId) ?? 0) + 1);
  }

  // 후보는 실행이 아니라 문서로 거른다. 같은 원고를 다시 돌리면 실행 id가 새로 나므로 실행으로만
  // 거르면 한 번 평가한 글이 다음 라운드에 그대로 다시 들어간다. 반입 문서는 공개 관문을 지나지
  // 않았으므로 평가자에게 내보내지 않는다 — 화면에서 빼고 createRound에서도 거부한다.
  const usedDocumentIds = new Set(await roundedDocumentIds(db));
  const done = await db
    .select({
      id: Runs.id,
      documentId: Runs.documentId,
      documentKind: Documents.kind,
      refId: Documents.refId,
      promptSetLabel: PromptSets.label,
      generationId: PromptSets.generationId,
    })
    .from(Runs)
    .leftJoin(Documents, eq(Documents.id, Runs.documentId))
    .leftJoin(PromptSets, eq(PromptSets.id, Runs.promptSetId))
    .where(eq(Runs.status, 'done'))
    .orderBy(desc(Runs.createdAt));

  const candidates = done.filter((c) => c.documentKind === 'sampled' && !usedDocumentIds.has(c.documentId));

  // 왜 안 보이는지 없으면 목록이 비었을 때 실행이 실패한 것처럼 읽힌다. 실행이 아니라 문서로 센다.
  const countDocuments = (rows: typeof done) => new Set(rows.map((r) => r.documentId)).size;
  const excluded = {
    used: countDocuments(done.filter((c) => c.documentKind === 'sampled' && usedDocumentIds.has(c.documentId))),
    intake: countDocuments(done.filter((c) => c.documentKind !== 'sampled')),
  };

  return {
    rounds: rounds.map((r) => ({
      id: r.id,
      label: r.label,
      evaluationId: r.evaluationId,
      evaluationLabel: evaluationById(r.evaluationId)?.evaluation.label ?? `${r.evaluationId} (제거됨)`,
      active: r.active,
      createdAt: r.createdAt.toISOString(),
      total: tasks.filter((t) => t.roundId === r.id).length,
      done: doneBy.get(r.id) ?? 0,
    })),
    candidates,
    excluded,
    evaluations: GENERATIONS.flatMap((g) =>
      g.evaluations.map((e) => ({ id: qualifiedEvaluationId(g.id, e.id), label: `${g.label} · ${e.label}`, generationId: g.id })),
    ),
  };
};

export const actions: Actions = {
  create: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    const result = await createRound(createDb(platform.env.DB), {
      label: String(form.get('label') ?? '').trim(),
      evaluationId: String(form.get('evaluationId') ?? ''),
      runIds: form.getAll('runIds').map(String),
    });
    return 'error' in result ? fail(400, { message: result.error }) : { roundId: result.roundId };
  },

  // 판정이 하나라도 있으면 지우지 않는다 — 사람이 들인 시간이 태스크에 매달려 있다.
  invalidate: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const db = createDb(platform.env.DB);
    const form = await request.formData();
    const roundId = String(form.get('roundId') ?? '');

    const roundTasks = await db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, roundId));
    if (roundTasks.length === 0) return fail(400, { message: '무효화할 태스크가 없습니다' });

    const roundTaskIds = roundTasks.map((t) => t.id);
    const judged = await inChunks(roundTaskIds, (chunk) =>
      db.select({ id: Judgments.id }).from(Judgments).where(inArray(Judgments.taskId, chunk)).limit(1),
    );
    if (judged.length > 0) return fail(409, { message: '판정이 존재하는 라운드는 무효화할 수 없습니다' });

    await inChunks(roundTaskIds, (chunk) =>
      db.delete(TaskReleases).where(inArray(TaskReleases.taskId, chunk)).returning({ taskId: TaskReleases.taskId }),
    );
    await db.delete(Tasks).where(eq(Tasks.roundId, roundId));
    await db.delete(Rounds).where(eq(Rounds.id, roundId));
    return { invalidated: true };
  },

  toggle: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    await setRoundActive(createDb(platform.env.DB), String(form.get('id') ?? ''), form.get('active') === 'true');
    return { ok: true };
  },
};
