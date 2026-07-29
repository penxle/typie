import { error } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import { createDb, Documents, FeedbackAnchors, Feedbacks, FeedbackSets, Judgments, Tasks } from '$lib/server/db/index.ts';
import { effectiveProgress } from '$lib/server/progress.ts';
import type { FeedbackVerdictMap, ReviewVerdictMap } from '$lib/domain/verdicts.ts';
import type { PageServerLoad } from './$types';

// 평가 화면(/tasks/[id])과 동일한 데이터 형태를 만들되, 클레임·draft를 일절 건드리지 않는 읽기 전용 로드.
// 반환 타입을 평가 화면의 행 타입으로 못박아 둔다 — 미리보기가 TaskView에 넘길 때 형 변환을
// 쓰면 키 하나만 빠져도 런타임에서야 터진다(실제로 그렇게 터졌다).
export const load: PageServerLoad = async ({ params, platform }) => {
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

  const isAnalysis = sets.some((s) => s.review !== null);
  const feedbackIds = feedbacks.map((f) => f.id);
  const anchors =
    isAnalysis && feedbackIds.length > 0
      ? await db.select().from(FeedbackAnchors).where(inArray(FeedbackAnchors.feedbackId, feedbackIds))
      : [];

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

  const round = await effectiveProgress(db);

  return {
    isAnalysis,
    verdicts: {} as FeedbackVerdictMap,
    reviewVerdicts: {} as ReviewVerdictMap,
    task: { id: task.id, kind: task.kind, setIds: task.setIds },
    document: { content: document.content, characterCount: document.characterCount },
    sets: orderedSets,
    draft: null as typeof Judgments.$inferSelect | null,
    setCount: sets.length,
    progress: {
      done: 0,
      myTotal: 0,
      roundDone: round.done,
      roundRequired: round.required,
    },
  };
};
