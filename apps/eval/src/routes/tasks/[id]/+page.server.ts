import { error, redirect } from '@sveltejs/kit';
import { and, eq, inArray, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { claimNextTask } from '$lib/server/claim.ts';
import { createDb, Documents, Feedbacks, FeedbackSets, Judgments, Tasks } from '$lib/server/db/index.ts';
import type { JudgmentResult } from '$lib/domain/types.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform, locals }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const [task] = await db.select().from(Tasks).where(eq(Tasks.id, params.id));
  if (!task) {
    error(404, 'task not found');
  }

  const [document] = await db.select().from(Documents).where(eq(Documents.id, task.documentId));
  if (!document) {
    error(500, 'document missing');
  }

  const sets = await db.select().from(FeedbackSets).where(inArray(FeedbackSets.id, task.setIds));
  const feedbacks = await db.select().from(Feedbacks).where(inArray(Feedbacks.setId, task.setIds)).orderBy(Feedbacks.ord);

  const [draft] = await db
    .select()
    .from(Judgments)
    .where(and(eq(Judgments.taskId, task.id), eq(Judgments.evaluatorEmail, locals.email)));

  if (draft && !draft.draft) {
    redirect(302, '/');
  }

  const orderedSets = task.setIds.map((setId) => ({
    setId,
    feedbacks: feedbacks.filter((f) => f.setId === setId),
  }));

  const [roundRequired] = await db.select({ n: sql<number>`coalesce(sum(coalesce(${Tasks.requiredJudgments}, 1)), 0)` }).from(Tasks);
  const [roundDone] = await db
    .select({ n: sql<number>`count(*)` })
    .from(Judgments)
    .where(eq(Judgments.draft, false));
  const [myDone] = await db
    .select({ n: sql<number>`count(*)` })
    .from(Judgments)
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, false)));

  return {
    task: { id: task.id, kind: task.kind, setIds: task.setIds },
    document: { content: document.content, characterCount: document.characterCount },
    sets: orderedSets,
    draft: draft ?? null,
    setCount: sets.length,
    progress: { done: myDone?.n ?? 0, roundDone: roundDone?.n ?? 0, roundRequired: roundRequired?.n ?? 0 },
  };
};

// 원자적 upsert — select-후-insert는 제출 연타 시 동시 요청이 겹쳐 UNIQUE 위반 500을 낸다.
// save(draft)는 이미 확정된 판정을 되돌리지 못한다: 느린 임시 저장 응답이 제출보다 늦게
// 도착해도 확정을 draft로 강등하지 않는다(setWhere).
const upsertJudgment = async (
  db: ReturnType<typeof createDb>,
  input: {
    taskId: string;
    email: string;
    result: JudgmentResult | null;
    falsePositiveFeedbackIds: string[];
    comment: string;
    elapsedSeconds: number;
    draft: boolean;
  },
) => {
  await db
    .insert(Judgments)
    .values({
      id: nanoid(),
      taskId: input.taskId,
      evaluatorEmail: input.email,
      result: input.result,
      falsePositiveFeedbackIds: input.falsePositiveFeedbackIds,
      comment: input.comment,
      elapsedSeconds: input.elapsedSeconds,
      draft: input.draft,
    })
    .onConflictDoUpdate({
      target: [Judgments.taskId, Judgments.evaluatorEmail],
      set: {
        ...(input.result && { result: input.result }),
        falsePositiveFeedbackIds: input.falsePositiveFeedbackIds,
        comment: input.comment,
        elapsedSeconds: input.elapsedSeconds,
        draft: input.draft,
        updatedAt: new Date(),
      },
      setWhere: input.draft ? sql`${Judgments.draft} = 1` : undefined,
    });
};

const parseForm = async (request: Request) => {
  const form = await request.formData();
  return {
    result: form.get('result') ? (JSON.parse(form.get('result') as string) as JudgmentResult) : null,
    falsePositiveFeedbackIds: JSON.parse((form.get('falsePositiveFeedbackIds') as string) || '[]') as string[],
    comment: (form.get('comment') as string) || '',
    elapsedSeconds: Number(form.get('elapsedSeconds') ?? 0),
  };
};

// 클라이언트가 보낸 판정에서 이 태스크에 속하지 않는 setId·피드백 id를 걷어낸다 — 폼 상태가
// 태스크 간에 새는 클라이언트 버그가 재발해도 다른 태스크의 항목이 저장되지 않게 하는 방어선.
const sanitizeForTask = async (db: ReturnType<typeof createDb>, taskId: string, input: Awaited<ReturnType<typeof parseForm>>) => {
  const [task] = await db.select({ setIds: Tasks.setIds }).from(Tasks).where(eq(Tasks.id, taskId));
  if (!task) {
    error(404, 'task not found');
  }

  const feedbackRows = await db.select({ id: Feedbacks.id }).from(Feedbacks).where(inArray(Feedbacks.setId, task.setIds));
  const validFeedbackIds = new Set(feedbackRows.map((f) => f.id));

  let result = input.result;
  if (result?.kind === 'ranking') {
    result = { kind: 'ranking', ranks: result.ranks.filter((r) => task.setIds.includes(r.setId)) };
  }

  return {
    input: {
      ...input,
      result,
      falsePositiveFeedbackIds: input.falsePositiveFeedbackIds.filter((id) => validFeedbackIds.has(id)),
    },
    taskSetIds: task.setIds,
  };
};

const isComplete = (result: JudgmentResult | null, taskSetIds: string[]): boolean => {
  if (!result) return false;
  if (result.kind === 'pair') return true;
  const ranked = new Map(result.ranks.map((r) => [r.setId, r.rank]));
  return taskSetIds.every((setId) => (ranked.get(setId) ?? 0) > 0);
};

export const actions: Actions = {
  save: async ({ params, request, platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }

    const db = createDb(platform.env.DB);
    const { input } = await sanitizeForTask(db, params.id, await parseForm(request));
    await upsertJudgment(db, { taskId: params.id, email: locals.email, ...input, draft: true });
    return { saved: true };
  },
  submit: async ({ params, request, platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }

    const db = createDb(platform.env.DB);
    const { input, taskSetIds } = await sanitizeForTask(db, params.id, await parseForm(request));
    if (!isComplete(input.result, taskSetIds)) {
      error(400, 'result required');
    }
    await upsertJudgment(db, { taskId: params.id, email: locals.email, ...input, draft: false });
    const nextTaskId = await claimNextTask(db, locals.email);
    redirect(302, nextTaskId ? `/tasks/${nextTaskId}` : '/?finished=1');
  },
};
