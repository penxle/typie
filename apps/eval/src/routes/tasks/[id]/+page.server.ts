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

  const [taskTotal] = await db.select({ n: sql<number>`count(*)` }).from(Tasks);
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
    progress: { done: myDone?.n ?? 0, total: taskTotal?.n ?? 0 },
  };
};

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
  const [existing] = await db
    .select({ id: Judgments.id })
    .from(Judgments)
    .where(and(eq(Judgments.taskId, input.taskId), eq(Judgments.evaluatorEmail, input.email)));

  if (existing) {
    await db
      .update(Judgments)
      .set({
        result: input.result ?? undefined,
        falsePositiveFeedbackIds: input.falsePositiveFeedbackIds,
        comment: input.comment,
        elapsedSeconds: input.elapsedSeconds,
        draft: input.draft,
        updatedAt: new Date(),
      })
      .where(eq(Judgments.id, existing.id));
  } else {
    await db.insert(Judgments).values({
      id: nanoid(),
      taskId: input.taskId,
      evaluatorEmail: input.email,
      result: input.result,
      falsePositiveFeedbackIds: input.falsePositiveFeedbackIds,
      comment: input.comment,
      elapsedSeconds: input.elapsedSeconds,
      draft: input.draft,
    });
  }
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

export const actions: Actions = {
  save: async ({ params, request, platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }

    const db = createDb(platform.env.DB);
    const input = await parseForm(request);
    await upsertJudgment(db, { taskId: params.id, email: locals.email, ...input, draft: true });
    return { saved: true };
  },
  submit: async ({ params, request, platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }

    const db = createDb(platform.env.DB);
    const input = await parseForm(request);
    if (!input.result) {
      error(400, 'result required');
    }
    await upsertJudgment(db, { taskId: params.id, email: locals.email, ...input, draft: false });
    const nextTaskId = await claimNextTask(db, locals.email);
    redirect(302, nextTaskId ? `/tasks/${nextTaskId}` : '/?finished=1');
  },
};
