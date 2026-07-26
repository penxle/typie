import { error, redirect } from '@sveltejs/kit';
import { and, eq, inArray, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { deriveFalsePositiveIds, FEEDBACK_LABEL_KEYS } from '$lib/domain/feedback-labels.ts';
import { isEmptyFeedbackVerdict, isEmptyReviewVerdict, isFeedbackComplete, isReviewComplete } from '$lib/domain/verdicts.ts';
import { claimableSummary, claimNextTask } from '$lib/server/claim.ts';
import {
  createDb,
  Documents,
  FeedbackAnchors,
  Feedbacks,
  FeedbackSets,
  FeedbackVerdicts,
  Judgments,
  ReleasedTasks,
  ReviewVerdicts,
  Tasks,
} from '$lib/server/db/index.ts';
import { effectiveProgress } from '$lib/server/progress.ts';
import type { FeedbackLabelMap } from '$lib/domain/feedback-labels.ts';
import type { JudgmentResult } from '$lib/domain/types.ts';
import type { FeedbackVerdictMap, ReviewVerdictMap } from '$lib/domain/verdicts.ts';
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

  // 재설계 파이프라인 세트는 총평(review)을 갖는다 — 이 값의 유무로 판정 형식이 갈린다.
  const analysisSets = sets.filter((s) => s.review !== null);
  const isAnalysis = analysisSets.length > 0;

  const feedbackIds = feedbacks.map((f) => f.id);
  const anchors =
    isAnalysis && feedbackIds.length > 0
      ? await db.select().from(FeedbackAnchors).where(inArray(FeedbackAnchors.feedbackId, feedbackIds))
      : [];

  const verdicts = isAnalysis && draft ? await db.select().from(FeedbackVerdicts).where(eq(FeedbackVerdicts.judgmentId, draft.id)) : [];
  const reviewVerdicts = isAnalysis && draft ? await db.select().from(ReviewVerdicts).where(eq(ReviewVerdicts.judgmentId, draft.id)) : [];

  const orderedSets = task.setIds.map((setId) => ({
    setId,
    review: sets.find((s) => s.id === setId)?.review ?? null,
    feedbacks: feedbacks
      .filter((f) => f.setId === setId)
      .map((f) => ({
        ...f,
        anchors: anchors
          .filter((a) => a.feedbackId === f.id)
          .toSorted((a, b) => a.ord - b.ord)
          .map((a) => ({ startText: a.startText, endText: a.endText, matchStart: a.matchStart, matchEnd: a.matchEnd })),
      })),
  }));

  // 내 판정 분자·분모는 현재(최신) 라운드만 센다 — 지난 라운드 판정을 이월하지 않는다.
  const round = await effectiveProgress(db, platform.env.ADMIN_EMAILS ?? '');
  const roundScope = round.roundId ? eq(Tasks.roundId, round.roundId) : sql`0 = 1`;
  const [myDone] = await db
    .select({ n: sql<number>`count(*)` })
    .from(Judgments)
    .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, false), roundScope));
  const [myDrafts] = await db
    .select({ n: sql<number>`count(*)` })
    .from(Judgments)
    .innerJoin(Tasks, eq(Tasks.id, Judgments.taskId))
    .where(and(eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, true), roundScope));
  const { potential } = await claimableSummary(db, locals.email, platform.env.ADMIN_EMAILS ?? '');

  return {
    isAnalysis,
    verdicts: Object.fromEntries(
      verdicts.map((v) => [v.feedbackId, { correct: v.correct, needed: v.needed, useful: v.useful, ...(v.note && { note: v.note }) }]),
    ) as FeedbackVerdictMap,
    reviewVerdicts: Object.fromEntries(
      reviewVerdicts.map((v) => [
        v.setId,
        { readCorrectly: v.readCorrectly, priorityUseful: v.priorityUseful, ...(v.note && { note: v.note }) },
      ]),
    ) as ReviewVerdictMap,
    task: { id: task.id, kind: task.kind, setIds: task.setIds },
    document: { content: document.content, characterCount: document.characterCount },
    sets: orderedSets,
    // 배열 구조분해는 undefined를 타입에 남기지 않아 ?? null이 지워진다. 명시하지 않으면
    // draft 없는 화면(어드민 미리보기)이 같은 형태를 만들 수 없다.
    draft: (draft ?? null) as typeof Judgments.$inferSelect | null,
    setCount: sets.length,
    progress: {
      done: myDone?.n ?? 0,
      myTotal: (myDone?.n ?? 0) + (myDrafts?.n ?? 0) + potential,
      roundDone: round.done,
      roundRequired: round.required,
    },
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

  const [row] = await db
    .select({ id: Judgments.id })
    .from(Judgments)
    .where(and(eq(Judgments.taskId, input.taskId), eq(Judgments.evaluatorEmail, input.email)));
  if (!row) {
    error(500, 'judgment missing after upsert');
  }
  return row.id;
};

// 판정은 지우고 다시 넣는다 — 평가자가 껐다 켠 항목이 이전 저장분으로 남지 않게 한다.
const replaceVerdicts = async (
  db: ReturnType<typeof createDb>,
  judgmentId: string,
  verdicts: FeedbackVerdictMap,
  reviewVerdicts: ReviewVerdictMap,
) => {
  await db.delete(FeedbackVerdicts).where(eq(FeedbackVerdicts.judgmentId, judgmentId));
  await db.delete(ReviewVerdicts).where(eq(ReviewVerdicts.judgmentId, judgmentId));

  const feedbackRows = Object.entries(verdicts).map(([feedbackId, v]) => ({
    id: nanoid(),
    judgmentId,
    feedbackId,
    correct: v.correct,
    needed: v.needed,
    useful: v.useful,
    note: v.note ?? null,
  }));
  if (feedbackRows.length > 0) {
    await db.insert(FeedbackVerdicts).values(feedbackRows);
  }

  const reviewRows = Object.entries(reviewVerdicts).map(([setId, v]) => ({
    id: nanoid(),
    judgmentId,
    setId,
    readCorrectly: v.readCorrectly,
    priorityUseful: v.priorityUseful,
    note: v.note ?? null,
  }));
  if (reviewRows.length > 0) {
    await db.insert(ReviewVerdicts).values(reviewRows);
  }
};

const parseForm = async (request: Request) => {
  const form = await request.formData();
  return {
    result: form.get('result') ? (JSON.parse(form.get('result') as string) as JudgmentResult) : null,
    feedbackLabels: JSON.parse((form.get('feedbackLabels') as string) || '{}') as FeedbackLabelMap,
    verdicts: JSON.parse((form.get('verdicts') as string) || '{}') as FeedbackVerdictMap,
    reviewVerdicts: JSON.parse((form.get('reviewVerdicts') as string) || '{}') as ReviewVerdictMap,
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

  // 세 값 모두 3상태다 — true/false만 신뢰하고 나머지는 미판정(null)으로 떨어뜨린다.
  const asVerdict = (value: unknown) => (value === true ? true : value === false ? false : null);
  const asNote = (value: unknown) => (typeof value === 'string' && value.trim() ? { note: value.trim().slice(0, 1000) } : {});

  const sanitizedVerdicts: FeedbackVerdictMap = {};
  for (const [feedbackId, entry] of Object.entries(input.verdicts)) {
    if (!validFeedbackIds.has(feedbackId)) continue;
    const verdict = {
      correct: asVerdict(entry.correct),
      needed: asVerdict(entry.needed),
      useful: asVerdict(entry.useful),
      ...asNote(entry.note),
    };
    if (isEmptyFeedbackVerdict(verdict)) continue;
    sanitizedVerdicts[feedbackId] = verdict;
  }

  const sanitizedReviewVerdicts: ReviewVerdictMap = {};
  for (const [setId, entry] of Object.entries(input.reviewVerdicts)) {
    if (!task.setIds.includes(setId)) continue;
    const verdict = {
      readCorrectly: asVerdict(entry.readCorrectly),
      priorityUseful: asVerdict(entry.priorityUseful),
      ...asNote(entry.note),
    };
    if (isEmptyReviewVerdict(verdict)) continue;
    sanitizedReviewVerdicts[setId] = verdict;
  }

  return {
    input: {
      ...input,
      result,
      feedbackLabels: sanitizedLabels,
      verdicts: sanitizedVerdicts,
      reviewVerdicts: sanitizedReviewVerdicts,
      falsePositiveFeedbackIds: deriveFalsePositiveIds(sanitizedLabels),
    },
    taskSetIds: task.setIds,
  };
};

// 제출은 화면에서도 막지만 서버가 다시 본다 — 폼을 직접 던지면 화면 조건은 우회된다.
// 일부만 답한 판정이 들어오면 비율의 분모가 무너져 라운드 전체의 수치를 못 쓰게 된다.
const assertAnalysisComplete = async (
  db: ReturnType<typeof createDb>,
  taskSetIds: string[],
  verdicts: FeedbackVerdictMap,
  reviewVerdicts: ReviewVerdictMap,
) => {
  const sets = await db
    .select({ id: FeedbackSets.id, review: FeedbackSets.review })
    .from(FeedbackSets)
    .where(inArray(FeedbackSets.id, taskSetIds));
  const analysisSets = sets.filter((s) => s.review !== null);
  if (analysisSets.length === 0) return;

  const feedbacks = await db
    .select({ id: Feedbacks.id })
    .from(Feedbacks)
    .where(
      inArray(
        Feedbacks.setId,
        analysisSets.map((s) => s.id),
      ),
    );
  const missing = feedbacks.filter((f) => !isFeedbackComplete(verdicts[f.id])).length;
  if (missing > 0) {
    error(400, `${missing} feedbacks not fully judged`);
  }
  const missingReviews = analysisSets.filter((s) => !isReviewComplete(reviewVerdicts[s.id])).length;
  if (missingReviews > 0) {
    error(400, `${missingReviews} reviews not judged`);
  }
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
    const { verdicts, reviewVerdicts, ...judgment } = input;
    const judgmentId = await upsertJudgment(db, { taskId: params.id, email: locals.email, ...judgment, draft: true });
    await replaceVerdicts(db, judgmentId, verdicts, reviewVerdicts);
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
    const { verdicts, reviewVerdicts, ...judgment } = input;
    await assertAnalysisComplete(db, taskSetIds, verdicts, reviewVerdicts);
    const judgmentId = await upsertJudgment(db, { taskId: params.id, email: locals.email, ...judgment, draft: false });
    await replaceVerdicts(db, judgmentId, verdicts, reviewVerdicts);
    const nextTaskId = await claimNextTask(db, locals.email, platform.env.ADMIN_EMAILS ?? '');
    redirect(302, nextTaskId ? `/tasks/${nextTaskId}` : '/?finished=1');
  },
  release: async ({ params, platform, locals }) => {
    if (!platform) {
      error(500, 'platform unavailable');
    }

    const db = createDb(platform.env.DB);
    // 반납 기록 — 이 평가자에게는 같은 태스크를 다시 배정하지 않는다(타 평가자 배정은 정상).
    await db.insert(ReleasedTasks).values({ taskId: params.id, evaluatorEmail: locals.email }).onConflictDoNothing();
    const [dropped] = await db
      .delete(Judgments)
      .where(and(eq(Judgments.taskId, params.id), eq(Judgments.evaluatorEmail, locals.email), eq(Judgments.draft, true)))
      .returning({ id: Judgments.id });
    if (dropped) {
      await db.delete(FeedbackVerdicts).where(eq(FeedbackVerdicts.judgmentId, dropped.id));
      await db.delete(ReviewVerdicts).where(eq(ReviewVerdicts.judgmentId, dropped.id));
    }
    redirect(303, '/');
  },
};
