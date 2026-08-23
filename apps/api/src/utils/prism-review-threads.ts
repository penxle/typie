import { asc, count, eq } from 'drizzle-orm';
import {
  db,
  first,
  firstOrThrow,
  PrismReviewDocumentVersions,
  PrismReviewRounds,
  PrismReviewThreadComments,
  PrismReviewThreads,
} from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { threadsFromResult } from './prism-review-core.ts';

type ThreadComment = typeof PrismReviewThreadComments.$inferSelect;

const loadRound = async (roundId: string) =>
  await db
    .select({
      id: PrismReviewRounds.id,
      documentId: PrismReviewRounds.documentId,
      round: PrismReviewRounds.round,
      result: PrismReviewRounds.result,
    })
    .from(PrismReviewRounds)
    .where(eq(PrismReviewRounds.id, roundId))
    .then(first);

// 회차의 리뷰 시점 원고 — 인용을 자를 때만 필요하다. 한 회차의 스레드가 전부 같은 판본을 쓰므로
// 요청 안에서 한 번만 읽는다.
const versionCache = new WeakMap<object, Map<string, Promise<string>>>();

export const roundVersionContent = async (scope: object, roundId: string): Promise<string> => {
  let byRound = versionCache.get(scope);
  if (!byRound) {
    byRound = new Map();
    versionCache.set(scope, byRound);
  }

  const cached = byRound.get(roundId);
  if (cached) return cached;

  const loading = db
    .select({ content: PrismReviewDocumentVersions.content })
    .from(PrismReviewRounds)
    .innerJoin(PrismReviewDocumentVersions, eq(PrismReviewRounds.documentVersionId, PrismReviewDocumentVersions.id))
    .where(eq(PrismReviewRounds.id, roundId))
    .then(firstOrThrow)
    .then((row) => row.content);

  byRound.set(roundId, loading);
  return loading;
};

// 여백은 회차의 스레드를 한꺼번에 펼친다 — 스레드마다 읽으면 왕복이 스레드 수만큼 늘어나므로
// 회차의 댓글을 한 번에 읽어 스레드별로 묶는다.
const commentCache = new WeakMap<object, Map<string, Promise<Map<string, ThreadComment[]>>>>();

export const roundThreadComments = async (scope: object, roundId: string): Promise<Map<string, ThreadComment[]>> => {
  let byRound = commentCache.get(scope);
  if (!byRound) {
    byRound = new Map();
    commentCache.set(scope, byRound);
  }

  const cached = byRound.get(roundId);
  if (cached) return cached;

  const loading = db
    .select({ comment: PrismReviewThreadComments })
    .from(PrismReviewThreadComments)
    .innerJoin(PrismReviewThreads, eq(PrismReviewThreadComments.threadId, PrismReviewThreads.id))
    .where(eq(PrismReviewThreads.roundId, roundId))
    .orderBy(asc(PrismReviewThreadComments.createdAt))
    .then((rows) => {
      const grouped = new Map<string, ThreadComment[]>();
      for (const { comment } of rows) {
        const bucket = grouped.get(comment.threadId);
        if (bucket) {
          bucket.push(comment);
        } else {
          grouped.set(comment.threadId, [comment]);
        }
      }

      return grouped;
    });

  byRound.set(roundId, loading);
  return loading;
};

// 구독 ctx는 소켓만큼 오래 산다 — 요청 단위로 접으려고 만든 메모가 거기서는 첫 이벤트의 값을 영구히 붙든다.
export const clearRoundMemos = (scope: object): void => {
  versionCache.delete(scope);
  commentCache.delete(scope);
};

export const projectRoundThreads = async (roundId: string): Promise<void> => {
  const round = await loadRound(roundId);
  if (!round) return;

  const projected = threadsFromResult(round.result);
  if (projected.length === 0) return;

  await db
    .insert(PrismReviewThreads)
    .values(
      projected.map((thread) => ({
        documentId: round.documentId,
        bornRound: round.round,
        roundId: round.id,
        ...thread,
      })),
    )
    .onConflictDoNothing({
      target: [PrismReviewThreads.documentId, PrismReviewThreads.bornRound, PrismReviewThreads.issueIndex],
    });

  pubsub.publish('prism:review', round.documentId, { roundId: round.id });
};

// 배포 전에 끝난 회차와 사영 중 죽은 회차를 첫 조회가 메운다. 멱등이라 여러 번 불러도 안전하다.
export const ensureRoundThreads = async (roundId: string): Promise<void> => {
  const existing = await db
    .select({ count: count() })
    .from(PrismReviewThreads)
    .where(eq(PrismReviewThreads.roundId, roundId))
    .then(firstOrThrow);

  if (existing.count > 0) return;
  await projectRoundThreads(roundId);
};
