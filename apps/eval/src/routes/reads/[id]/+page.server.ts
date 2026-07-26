import { error } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import { createDb, Documents, FeedbackAnchors, Feedbacks, FeedbackSets, Judgments } from '$lib/server/db/index.ts';
import type { TaskKind } from '$lib/domain/types.ts';
import type { FeedbackVerdictMap, ReviewVerdictMap } from '$lib/domain/verdicts.ts';
import type { PageServerLoad } from './$types';

// 라운드에 속하지 않은 피드백 세트를 평가 화면과 같은 모양으로 읽기만 한다. 태스크 행을 만들지
// 않는 것이 핵심이다 — 태스크는 라운드에 매여 있고, 라운드를 새로 만들면 effectiveProgress가
// 최신 라운드를 그리로 옮겨 진행 중인 라운드의 진행률이 통째로 뒤바뀐다.
//
// 반환 타입은 평가 화면과 같아야 한다. 형 변환으로 맞추면 키 하나만 빠져도 런타임에서야 터진다.
export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  const [set] = await db.select().from(FeedbackSets).where(eq(FeedbackSets.id, params.id));
  if (!set) {
    error(404, 'set not found');
  }

  const [document] = await db.select().from(Documents).where(eq(Documents.id, set.documentId));
  if (!document) {
    error(500, 'document missing');
  }

  const feedbacks = await db.select().from(Feedbacks).where(eq(Feedbacks.setId, set.id)).orderBy(Feedbacks.ord);
  const feedbackIds = feedbacks.map((f) => f.id);
  const anchors =
    feedbackIds.length > 0 ? await db.select().from(FeedbackAnchors).where(inArray(FeedbackAnchors.feedbackId, feedbackIds)) : [];

  return {
    isAnalysis: set.review !== null,
    verdicts: {} as FeedbackVerdictMap,
    reviewVerdicts: {} as ReviewVerdictMap,
    // 태스크가 없으므로 세트 id를 그대로 쓴다 — 화면은 이 값을 키로만 쓴다.
    task: { id: set.id, kind: 'ranking' as TaskKind, setIds: [set.id] },
    document: { content: document.content, characterCount: document.characterCount },
    sets: [
      {
        setId: set.id,
        review: set.review,
        feedbacks: feedbacks.map((f) => ({
          ...f,
          anchors: anchors
            .filter((a) => a.feedbackId === f.id)
            .toSorted((a, b) => a.ord - b.ord)
            .map((a) => ({ startText: a.startText, endText: a.endText, matchStart: a.matchStart, matchEnd: a.matchEnd })),
        })),
      },
    ],
    draft: null as typeof Judgments.$inferSelect | null,
    setCount: 1,
    progress: { done: 0, myTotal: 0, roundDone: 0, roundRequired: 0 },
  };
};
