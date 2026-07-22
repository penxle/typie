import { error, redirect } from '@sveltejs/kit';
import { and, eq, inArray, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { deriveFalsePositiveIds, FEEDBACK_LABEL_KEYS } from '$lib/domain/feedback-labels.ts';
import { claimNextTask } from '$lib/server/claim.ts';
import { createDb, Documents, Feedbacks, FeedbackSets, Judgments, ReleasedTasks, Tasks } from '$lib/server/db/index.ts';
import { effectiveProgress } from '$lib/server/progress.ts';
import type { FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';
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

  const round = await effectiveProgress(db);
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
    progress: { done: myDone?.n ?? 0, roundDone: round.done, roundRequired: round.required },
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
    feedbackLabels: FeedbackLabelMap;
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
      feedbackLabels: input.feedbackLabels,
      comment: input.comment,
      elapsedSeconds: input.elapsedSeconds,
      draft: input.draft,
    })
    .onConflictDoUpdate({
      target: [Judgments.taskId, Judgments.evaluatorEmail],
      set: {
        ...(input.result && { result: input.result }),
        falsePositiveFeedbackIds: input.falsePositiveFeedbackIds,
        feedbackLabels: input.feedbackLabels,
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
    feedbackLabels: JSON.parse((form.get('feedbackLabels') as string) || '{}') as FeedbackLabelMap,
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
  if (result?.kind === 'scores') {
    result = {
      kind: 'scores',
      scores: result.scores
        .filter((s) => task.setIds.includes(s.setId) && Number.isSafeInteger(s.score) && s.score >= 1 && s.score <= 5)
        .map((s) => ({ setId: s.setId, score: s.score })),
    };
  }

  const sanitizedLabels: FeedbackLabelMap = {};
  for (const [feedbackId, entry] of Object.entries(input.feedbackLabels)) {
    if (!validFeedbackIds.has(feedbackId)) continue;
    const labels = [...new Set(entry.labels)].filter((key) => FEEDBACK_LABEL_KEYS.has(key));
    const comment = typeof entry.comment === 'string' ? entry.comment.slice(0, 1000) : undefined;
    if (labels.length === 0 && !comment) continue;
    sanitizedLabels[feedbackId] = { labels, ...(comment && { comment }) };
  }

  return {
    input: {
      ...input,
      result,
      feedbackLabels: sanitizedLabels,
      falsePositiveFeedbackIds: deriveFalsePositiveIds(sanitizedLabels),
    },
    taskSetIds: task.setIds,
  };
};

const isComplete = (result: JudgmentResult | null, taskSetIds: string[]): boolean => {
  if (!result) return false;
  if (result.kind === 'pair') return true;
  if (result.kind === 'scores') {
    const scored = new Map(result.scores.map((s) => [s.setId, s.score]));
    return taskSetIds.every((setId) => scored.has(setId));
  }
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
  release: async ({ params, platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }

    const db = createDb(platform.env.DB);
    // 반납 기록 — 이 평가자에게는 같은 태스크를 다시 배정하지 않는다(타 평가자 배정은 정상).
    await db.insert(ReleasedTasks).values({ taskId: params.id, evaluatorEmail: locals.email }).onConflictDoNothing();
    await db
      .delete(Judgments)
      .where(and(eq(Judgments.taskId, params.id), eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, true)));
    redirect(303, '/');
  },
};
