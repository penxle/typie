import { error } from '@sveltejs/kit';
import { eq, inArray } from 'drizzle-orm';
import { createDb, Documents, Feedbacks, FeedbackSets, Tasks } from '$lib/server/db/index.ts';
import { effectiveProgress } from '$lib/server/progress.ts';
import type { PageServerLoad } from './$types';

// 평가 화면(/tasks/[id])과 동일한 데이터 형태를 만들되, 클레임·draft를 일절 건드리지 않는 읽기 전용 로드.
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

  const orderedSets = task.setIds.map((setId) => ({
    setId,
    feedbacks: feedbacks.filter((f) => f.setId === setId),
  }));

  const round = await effectiveProgress(db);

  return {
    task: { id: task.id, kind: task.kind, setIds: task.setIds },
    document: { content: document.content, characterCount: document.characterCount },
    sets: orderedSets,
    draft: null,
    setCount: sets.length,
    progress: {
      done: 0,
      myTotal: 0,
      roundDone: round.done,
      roundRequired: round.required,
    },
  };
};
