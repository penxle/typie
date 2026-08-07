import { error } from '@sveltejs/kit';
import { and, desc, eq } from 'drizzle-orm';
import { sumCost } from '$lib/feedback/pricing.ts';
import { sumUsage } from '$lib/feedback/usage.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import type { RunUsage } from '$lib/feedback/types.ts';
import type { PageServerLoad } from './$types';

// 한국 표준시는 1988년 이후 서머타임이 없다 — 고정 오프셋이면 런타임 시간대·ICU에 기대지 않고 같은 값이 나온다.
const KST_OFFSET_MS = 9 * 60 * 60 * 1000;

const pad = (value: number) => String(value).padStart(2, '0');

const kstStamp = (at: Date): string => {
  const shifted = new Date(at.getTime() + KST_OFFSET_MS);
  const date = `${shifted.getUTCFullYear()}. ${pad(shifted.getUTCMonth() + 1)}. ${pad(shifted.getUTCDate())}.`;
  return `${date} ${pad(shifted.getUTCHours())}:${pad(shifted.getUTCMinutes())}`;
};

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  // 세션과 round 1 리뷰는 한 batch로 함께 들어간다 — 리뷰 없는 세션은 없으므로 inner join이 목록을 잃지 않는다.
  const rows = await db
    .select({
      id: FeedbackSessions.id,
      refId: FeedbackSessions.refId,
      title: FeedbackSessions.title,
      testerEmail: FeedbackSessions.testerEmail,
      status: Reviews.status,
      startedAt: Reviews.startedAt,
      finishedAt: Reviews.finishedAt,
      usage: Reviews.usage,
    })
    .from(FeedbackSessions)
    .innerJoin(Reviews, and(eq(Reviews.sessionId, FeedbackSessions.id), eq(Reviews.round, 1)))
    .orderBy(desc(FeedbackSessions.createdAt));

  return {
    reviews: rows.map((row) => ({
      id: row.id,
      refId: row.refId,
      title: row.title,
      testerEmail: row.testerEmail,
      status: row.status,
      startedAt: kstStamp(row.startedAt),
      finishedAt: row.finishedAt === null ? null : kstStamp(row.finishedAt),
      // usage의 생산자는 사영뿐이다 — 쓴 타입 그대로 좁힌다(project.ts:92-97).
      usage: sumUsage(row.usage as RunUsage | null),
      // 원가는 토큰과 같은 원본에서 낸다 — usage가 있는데 cost가 null이면 단가 미설정 모델이 섞였다는 뜻이다.
      cost: sumCost(row.usage as RunUsage | null),
    })),
  };
};
