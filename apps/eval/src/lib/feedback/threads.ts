export type ThreadState = 'open' | 'closed' | 'resolved' | 'withdrawn';

export const canClose = (state: ThreadState): boolean => state === 'open';
export const canReopen = (state: ThreadState): boolean => state === 'closed';

// 이 스레드의 앵커가 표시 회차 원고 기준인가 — 본문 마크·레일과 인용 블록이 같은 축을 쓴다.
// 표시 회차가 만든 스레드는 그 원고에서 뽑혔고, 승계 스레드는 kept 처분이 새 좌표로 갱신한 open만 유효하다
// (closed는 처분 자체를 건너뛰고, resolved·withdrawn은 종결이라 갱신 대상이 아니다 — project.ts applyDispositions).
export const hasCurrentAnchors = (thread: { reviewRound: number; state: ThreadState }, round: number): boolean =>
  thread.reviewRound === round || thread.state === 'open';

// 해결된 모드의 회차 그룹 — 정리 회차 서수 내림차(최신이 위), 그룹 안은 옛 문서 순. 그룹 안에는 인접 두 판의
// 좌표가 섞일 수 있으나(회차 중 닫힘=그 회차 판, 재리뷰 처분=직전 판) 목록 순서일 뿐이라 무해하다.
// 서수 없는 구 데이터는 최말단 그룹(number: null)으로 강등된다.
export type SettledGroup<T> = { number: number | null; threads: T[] };

export const settledGroups = <T extends { settledRoundNumber: number | null; anchors: { start: number }[] }>(
  threads: T[],
): SettledGroup<T>[] => {
  const byNumber = new Map<number | null, T[]>();
  for (const thread of threads) {
    byNumber.set(thread.settledRoundNumber, [...(byNumber.get(thread.settledRoundNumber) ?? []), thread]);
  }
  return [...byNumber]
    .toSorted(([a], [b]) => (b ?? -1) - (a ?? -1))
    .map(([number, group]) => ({
      number,
      threads: group.toSorted((a, b) => (a.anchors[0]?.start ?? 0) - (b.anchors[0]?.start ?? 0)),
    }));
};
