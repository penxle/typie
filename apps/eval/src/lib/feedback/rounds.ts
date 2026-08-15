import { REREVIEW_TIERS } from './tiers.ts';
import type { FeedbackRejection } from './types.ts';

// rejected는 결과가 거부(kind: 'rejected')로 종결된 완료 회차의 표지다 — status는 completed 그대로라
// 상태만으로는 갈라지지 않아 행마다 결과에서 읽어 얹는다(isRejectedResult).
export type RoundView = { round: number; status: 'running' | 'completed' | 'failed' | 'canceled'; tier: string; rejected: boolean };

// 거부 판별은 kind === 'rejected'로만 한다 — 구 저장 결과에는 kind 키 자체가 없다(부재 = 정상 결과).
export const isRejectedResult = (result: unknown): boolean =>
  typeof result === 'object' && result !== null && (result as { kind?: unknown }).kind === 'rejected';

export const rejectionOf = (result: unknown): FeedbackRejection['rejected'] | null =>
  isRejectedResult(result) ? (result as FeedbackRejection).rejected : null;

// 표시 회차 규칙 — 재리뷰 실패가 이전 완료 결과를 가리면 안 된다: 실패한 최신 회차는 배너로 강등되고
// 화면 본체는 최신 완료 회차가 맡는다. 1회차부터 실패한 세션만 실패 화면 그대로다.
// 거부 회차도 같은 규칙으로 강등된다(오너 결정 2026-08-15) — 정상 완료가 하나도 없는 세션만 거부가 본체다.
export const pickRounds = <T extends RoundView>(
  reviews: T[],
): { display: T; runningLatest: T | null; failedLatest: T | null; rejectedLatest: T | null; canRereview: boolean } => {
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- 호출처가 리뷰 0행을 먼저 404로 막으므로 목록은 비지 않는다
  const latest = reviews.at(-1)!;
  const runningLatest = latest.status === 'running' ? latest : null;
  const reviewedLatest = reviews.findLast((review) => review.status === 'completed' && !review.rejected) ?? null;
  const completedLatest = reviews.findLast((review) => review.status === 'completed') ?? null;
  const display = runningLatest ?? reviewedLatest ?? completedLatest ?? latest;
  const failedLatest = (latest.status === 'failed' || latest.status === 'canceled') && display.round !== latest.round ? latest : null;
  const rejectedLatest = latest.status === 'completed' && latest.rejected && display.round !== latest.round ? latest : null;
  // 거부 회차는 재검토의 기반이 될 산출물이 없다 — 본체가 거부인 세션은 새 세션 시작이 경로다.
  const canRereview =
    runningLatest === null &&
    display.status === 'completed' &&
    !display.rejected &&
    (REREVIEW_TIERS as readonly string[]).includes(display.tier);
  return { display, runningLatest, failedLatest, rejectedLatest, canRereview };
};

// 화면의 회차 서수 — 완료·실행 중 회차만 센다. 작가에게 실패·중단 회차의 재시도는 같은 회차의 반복이라
// 번호를 얻지 않는다. 거부 회차도 리뷰가 진행되지 않았으므로 번호를 얻지 않는다(실행 중에 임시로 얻었던
// 번호는 거부 종결과 함께 사라진다 — 정상 완료 회차의 서수는 여전히 불변이다). 내부 round는 PK·스레드 id의
// 좌표이므로 그대로 두고, 표시만 이 서수를 쓴다.
export const displayRoundNumbers = <T extends RoundView>(reviews: T[]): Record<number, number> => {
  const numbers: Record<number, number> = {};
  let ordinal = 0;
  for (const review of reviews) {
    if (review.status !== 'completed' && review.status !== 'running') continue;
    if (review.rejected) continue;
    ordinal += 1;
    numbers[review.round] = ordinal;
  }
  return numbers;
};

// 정리 회차 — 종결 시각(stateChangedAt)을 완료 회차의 시작 시각 경계에 대응시킨다. 해소·철회는 처분한
// 재리뷰 회차(사영은 완료 후 실행이라 그 회차 시작 이후), 닫힘은 닫은 시점의 표시 회차로 한 규칙에 셋 다
// 맞는다. 실패·중단 회차는 아무것도 정리하지 않으므로 경계에서 제외한다 — 거부 회차도 처분을 내리지
// 않으므로 같다. reviews는 round 오름차 전제.
export const settledRoundOf = (
  settledAt: number | null,
  reviews: { round: number; status: string; startedAt: number; rejected: boolean }[],
): number | null => {
  if (settledAt === null) return null;
  let found: number | null = null;
  for (const review of reviews) {
    if (review.status !== 'completed' || review.rejected) continue;
    if (review.startedAt <= settledAt) found = review.round;
  }
  return found;
};
