import { error } from '@sveltejs/kit';
import { asc, desc, eq, sql } from 'drizzle-orm';
import { sumCost } from '$lib/feedback/pricing.ts';
import { pickRounds } from '$lib/feedback/rounds.ts';
import { sumUsage } from '$lib/feedback/usage.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import { fetchPriceTable } from '$lib/server/prism.ts';
import type { RunUsage } from '$lib/feedback/types.ts';
import type { UsageTotals } from '$lib/feedback/usage.ts';
import type { PageServerLoad } from './$types';

// 한국 표준시는 1988년 이후 서머타임이 없다 — 고정 오프셋이면 런타임 시간대·ICU에 기대지 않고 같은 값이 나온다.
const KST_OFFSET_MS = 9 * 60 * 60 * 1000;

const pad = (value: number) => String(value).padStart(2, '0');

const kstStamp = (at: Date): string => {
  const shifted = new Date(at.getTime() + KST_OFFSET_MS);
  const date = `${shifted.getUTCFullYear()}. ${pad(shifted.getUTCMonth() + 1)}. ${pad(shifted.getUTCDate())}.`;
  return `${date} ${pad(shifted.getUTCHours())}:${pad(shifted.getUTCMinutes())}`;
};

// 세션의 토큰·원가는 전 회차 합산이다 — 재리뷰는 같은 원고에 이어 붙는 비용이라 회차별로 흩어 놓으면 실제 지출이 안 보인다.
// 한 회차라도 값이 없으면 전체를 null로 둔다: 부분합은 "이만큼 썼다"는 거짓 확정이 된다(sumCost의 단가 결측 처리와 같은 성질).
// complete는 전 회차 논리곱이다 — 한 회차라도 미확정이면 합계도 하한이라 화면이 '≥'로 읽는다.
const totalUsage = (perRound: (UsageTotals | null)[]): UsageTotals | null => {
  const total: UsageTotals = { complete: true, inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0 };
  for (const usage of perRound) {
    if (!usage) return null;
    total.complete &&= usage.complete;
    total.inputTokens += usage.inputTokens;
    total.outputTokens += usage.outputTokens;
    total.cacheReadTokens += usage.cacheReadTokens;
    total.cacheWriteTokens += usage.cacheWriteTokens;
  }
  return total;
};

const totalCost = (perRound: ({ krw: number } | null)[]): { krw: number } | null => {
  let krw = 0;
  for (const cost of perRound) {
    if (!cost) return null;
    krw += cost.krw;
  }
  return { krw };
};

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);
  // 단가표는 prism에서 걷는다(정본 단일화) — 수신 실패면 null이라 전 행의 원가가 '단가 미설정'으로 선다.
  const priceTable = await fetchPriceTable(platform.env);

  // 세션과 첫 리뷰는 한 batch로 함께 들어간다 — 리뷰 없는 세션은 없으므로 inner join이 목록을 잃지 않는다.
  // 회차 전량을 가져와 세션마다 표시 회차를 고르고(pickRounds — 세션 화면과 같은 규칙), 비용만 전 회차를 합산한다.
  // 정렬은 세션 묶음이 최신순으로 이어지고 묶음 안이 회차 오름차순이 되게 준다(pickRounds의 전제).
  const rows = await db
    .select({
      id: FeedbackSessions.id,
      refId: FeedbackSessions.refId,
      title: FeedbackSessions.title,
      testerEmail: FeedbackSessions.testerEmail,
      round: Reviews.round,
      tier: Reviews.tier,
      status: Reviews.status,
      startedAt: Reviews.startedAt,
      finishedAt: Reviews.finishedAt,
      usage: Reviews.usage,
      // 거부 표지만 뽑는다 — 표에 회차 전량의 result 전문을 실어 오면 무겁다.
      rejectedKind: sql<string | null>`json_extract(${Reviews.result}, '$.kind')`,
    })
    .from(FeedbackSessions)
    .innerJoin(Reviews, eq(Reviews.sessionId, FeedbackSessions.id))
    .orderBy(desc(FeedbackSessions.createdAt), asc(Reviews.round));

  // 세션별 묶음 — Map의 삽입 순서가 곧 표 순서다. 행이 있는 세션만 묶이므로 빈 묶음은 생기지 않는다.
  const grouped = new Map<string, ((typeof rows)[number] & { rejected: boolean })[]>();
  for (const raw of rows) {
    const row = { ...raw, rejected: raw.rejectedKind === 'rejected' };
    const group = grouped.get(row.id);
    if (group) group.push(row);
    else grouped.set(row.id, [row]);
  }

  return {
    reviews: [...grouped.values()].map((group) => {
      const row = group[0];
      const { display } = pickRounds(group);
      return {
        id: row.id,
        refId: row.refId,
        title: row.title,
        testerEmail: row.testerEmail,
        status: display.status,
        startedAt: kstStamp(display.startedAt),
        finishedAt: display.finishedAt === null ? null : kstStamp(display.finishedAt),
        // usage의 생산자는 사영뿐이다 — 쓴 타입 그대로 좁힌다(project.ts의 projectIfTerminal).
        usage: totalUsage(group.map((entry) => sumUsage(entry.usage as RunUsage | null))),
        // 원가는 토큰과 같은 원본에서 낸다 — usage가 있는데 cost가 null이면 단가 미설정 모델이 섞였다는 뜻이다.
        cost: totalCost(group.map((entry) => sumCost(entry.usage as RunUsage | null, priceTable))),
      };
    }),
  };
};
