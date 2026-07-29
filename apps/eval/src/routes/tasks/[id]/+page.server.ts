import { error, redirect } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import { loadArtifacts } from '$lib/server/artifacts.ts';
import { claimTask, releaseTask } from '$lib/server/assign.ts';
import { loadRunView } from '$lib/server/run-view.ts';
import { createDb, inChunks, JudgmentItems, Judgments, Rounds, Tasks } from '../../../../core/db.ts';
import { judgmentGaps, stageTargetFor } from '../../../../core/evaluation.ts';
import { evaluationById } from '../../../../core/registry.ts';
import { judgmentLock, LOCK_MESSAGE, releasable, stageIndexOf } from './lock.ts';
import type { Actions, PageServerLoad } from './$types';

const context = async (platform: App.Platform | undefined, taskId: string) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  const [task] = await db.select().from(Tasks).where(eq(Tasks.id, taskId));
  if (!task) error(404, 'task not found');
  const [round] = await db.select().from(Rounds).where(eq(Rounds.id, task.roundId));
  if (!round) error(500, 'round missing');

  const resolved = evaluationById(round.evaluationId);
  if (!resolved) error(500, `unknown evaluation: ${round.evaluationId}`);

  return { db, task, round, evaluation: resolved.evaluation };
};

export const load: PageServerLoad = async ({ params, platform, locals }) => {
  const { db, task, round, evaluation } = await context(platform, params.id);

  const [judgment] = await db.select().from(Judgments).where(eq(Judgments.taskId, task.id));
  if (!judgment || judgment.evaluatorEmail !== locals.email) error(404, 'task not assigned to you');
  if (!judgment.draft) redirect(302, '/');

  const view = await loadRunView(db, task.runId);
  if (!view) error(500, 'run missing');

  const entries = await db.select().from(JudgmentItems).where(eq(JudgmentItems.judgmentId, judgment.id));

  // 헤더의 진행 표시 — 내 판정과 라운드 전체를 함께 보여야 내 몫이 전체 어디쯤인지 읽힌다.
  const roundTasks = await db.select({ id: Tasks.id }).from(Tasks).where(eq(Tasks.roundId, round.id));
  const roundJudgments = await inChunks(
    roundTasks.map((t) => t.id),
    (chunk) =>
      db
        .select({ evaluatorEmail: Judgments.evaluatorEmail, draft: Judgments.draft })
        .from(Judgments)
        .where(inArray(Judgments.taskId, chunk)),
  );

  return {
    // 평가 정의는 함수를 품어 직렬화되지 않는다 — id만 넘기고 화면이 레지스트리에서 해석한다.
    evaluationId: round.evaluationId,
    round: { id: round.id, label: round.label, active: round.active },
    progress: {
      mine: roundJudgments.filter((j) => j.evaluatorEmail === locals.email && !j.draft).length,
      roundDone: roundJudgments.filter((j) => !j.draft).length,
      roundTotal: roundTasks.length,
    },
    view,
    answers: Object.fromEntries(entries.map((e) => [e.itemId, e.payload])),
    runAnswer: judgment.payload,
    elapsedSeconds: judgment.elapsedSeconds,
    lock: judgmentLock(round, judgment),
    stageIndex: stageIndexOf(evaluation, judgment),
    // 산출물은 항상 싣는다 — 언제 무엇을 보여줄지는 세대 UI가 정한다.
    artifacts: await loadArtifacts(db, view.generationId, task.runId),
  };
};

const parse = async (request: Request) => {
  const form = await request.formData();
  return {
    answers: JSON.parse((form.get('answers') as string) || '{}') as Record<string, Record<string, unknown>>,
    runAnswer: JSON.parse((form.get('runAnswer') as string) || '{}') as Record<string, unknown>,
    elapsedSeconds: Number(form.get('elapsedSeconds') ?? 0),
  };
};

// 클라이언트가 보낸 판정을 평가 정의로 걸러낸다. 확정된 단계의 답은 저장본이 진실이고,
// 클라이언트 값은 현재 단계의 필드에만 반영된다 — 확정 뒤에는 폼을 직접 던져도 고칠 수 없다.
const sanitizeForTask = async (
  platform: App.Platform | undefined,
  taskId: string,
  email: string,
  raw: Awaited<ReturnType<typeof parse>>,
) => {
  const { db, task, round, evaluation } = await context(platform, taskId);

  const [judgment] = await db.select().from(Judgments).where(eq(Judgments.taskId, task.id));
  if (!judgment || judgment.evaluatorEmail !== email) error(404, 'task not assigned to you');

  const lock = judgmentLock(round, judgment);
  if (lock) error(423, LOCK_MESSAGE[lock]);

  const view = await loadRunView(db, task.runId);
  if (!view) error(500, 'run missing');

  const stageIndex = stageIndexOf(evaluation, judgment);
  const stage = evaluation.stages[stageIndex];
  const stored = judgment.payload as Record<string, unknown>;
  const storedEntries = await db.select().from(JudgmentItems).where(eq(JudgmentItems.judgmentId, judgment.id));

  const itemPayloads: Record<string, Record<string, unknown>> = Object.fromEntries(storedEntries.map((e) => [e.itemId, e.payload]));
  for (const item of view.items) {
    const target = stageTargetFor(stage, item);
    const submitted = raw.answers[item.id];
    if (!target || !submitted) continue;
    itemPayloads[item.id] = Object.fromEntries(target.fields.map((f) => [f.key, f.sanitize(submitted[f.key])]));
  }

  const runPayload: Record<string, unknown> = {};
  for (const [index, s] of evaluation.stages.entries()) {
    if (index > stageIndex) break;
    for (const f of s.run) runPayload[f.key] = index === stageIndex ? f.sanitize(raw.runAnswer[f.key]) : (stored[f.key] ?? null);
  }

  return {
    db,
    task,
    judgment,
    evaluation,
    view,
    stageIndex,
    stage,
    itemPayloads,
    runPayload,
    elapsedSeconds: Math.max(0, Math.round(raw.elapsedSeconds)),
  };
};

const commit = async (
  db: ReturnType<typeof createDb>,
  input: {
    judgmentId: string;
    itemPayloads: Record<string, Record<string, unknown>>;
    runPayload: Record<string, unknown>;
    elapsedSeconds: number;
    draft: boolean;
    stage: number;
  },
): Promise<void> => {
  await db.delete(JudgmentItems).where(eq(JudgmentItems.judgmentId, input.judgmentId));
  for (const [itemId, payload] of Object.entries(input.itemPayloads)) {
    await db.insert(JudgmentItems).values({ id: crypto.randomUUID(), judgmentId: input.judgmentId, itemId, payload });
  }
  await db
    .update(Judgments)
    .set({ payload: input.runPayload, elapsedSeconds: input.elapsedSeconds, draft: input.draft, stage: input.stage, updatedAt: new Date() })
    .where(eq(Judgments.id, input.judgmentId));
};

export const actions: Actions = {
  save: async ({ params, request, platform, locals }) => {
    const ctx = await sanitizeForTask(platform, params.id, locals.email, await parse(request));
    await commit(ctx.db, {
      judgmentId: ctx.judgment.id,
      itemPayloads: ctx.itemPayloads,
      runPayload: ctx.runPayload,
      elapsedSeconds: ctx.elapsedSeconds,
      draft: true,
      stage: ctx.judgment.stage,
    });
    return { saved: true };
  },

  submit: async ({ params, request, platform, locals }) => {
    const ctx = await sanitizeForTask(platform, params.id, locals.email, await parse(request));
    const gaps = judgmentGaps(ctx.stage, ctx.view.items, ctx.runPayload, ctx.itemPayloads);
    if (gaps.run.length > 0 || gaps.items.length > 0) {
      error(400, `아직 답하지 않은 항목이 있습니다 (항목 ${gaps.items.length}건, 문항 ${gaps.run.length}건)`);
    }

    const confirmed = ctx.stageIndex + 1;
    const done = confirmed >= ctx.evaluation.stages.length;
    await commit(ctx.db, {
      judgmentId: ctx.judgment.id,
      itemPayloads: ctx.itemPayloads,
      runPayload: ctx.runPayload,
      elapsedSeconds: ctx.elapsedSeconds,
      draft: !done,
      stage: confirmed,
    });

    // 마지막 단계가 아니면 태스크는 계속된다 — 화면이 다시 로드해 다음 단계로 들어간다.
    if (!done) return { stageConfirmed: true };

    const next = await claimTask(ctx.db, ctx.task.roundId, locals.email);
    redirect(302, next ? `/tasks/${next}` : '/?finished=1');
  },

  release: async ({ params, platform, locals }) => {
    const { db, task } = await context(platform, params.id);
    const [judgment] = await db.select().from(Judgments).where(eq(Judgments.taskId, task.id));
    // 확정된 단계가 있는 판정의 반납은 확정된 답을 버린다 — 첫 단계에서만 허용한다.
    if (judgment && judgment.evaluatorEmail === locals.email && !releasable(judgment)) {
      error(400, '확정한 단계가 있어 반납할 수 없습니다');
    }
    await releaseTask(db, params.id, locals.email);
    redirect(303, '/');
  },
};
