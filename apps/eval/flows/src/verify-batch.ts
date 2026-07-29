// 검증 단계의 호출 편성. 지적마다 한 번씩 부르면 같은 원문을 지적 수만큼 다시 보내게 되고,
// 실측에서 그것이 파이프라인 입력의 68~87%였다(18,051자 문서 기준 원문을 24번 다시 읽었다).
//
// 읽을 원문이 같은 지적들을 한 호출로 묶어 원문을 한 번만 보낸다. 판정은 여전히 지적마다
// 독립이며, 묶는 기준은 "같은 대목을 읽는가"일 뿐 "서로 견주라"가 아니다.

type SceneRange = { start: number; end: number };

// 한 지적을 검증하는 데 필요한 장면들. 앵커가 놓인 장면과 앞뒤 한 장면씩이면
// "앞에서 이미 나왔다"류의 지적도 확인할 수 있다.
// null은 전문이 필요하다는 뜻이다 — 장면 지도가 없거나 앵커를 장면에 얹지 못한 경우다.
export const pickScenes = (scenes: SceneRange[], anchors: { matchStart: number | null }[]): number[] | null => {
  if (scenes.length === 0) return null;

  const picked = new Set<number>();
  for (const anchor of anchors) {
    const at = anchor.matchStart;
    if (at === null) return null;
    const index = scenes.findIndex((s) => at >= s.start && at < s.end);
    if (index === -1) return null;
    for (const i of [index - 1, index, index + 1]) {
      if (i >= 0 && i < scenes.length) picked.add(i);
    }
  }
  return [...picked].toSorted((a, b) => a - b);
};

export type VerifyBatch = { sceneIndexes: number[] | null; items: number[] };

/**
 * 같은 장면 집합을 읽는 지적끼리 묶는다.
 *
 * maxPerBatch는 한 호출이 판정할 지적 수의 상한이다. 원문을 공유하더라도 한 번에 너무 많이
 * 물으면 뒤쪽 판정이 성의를 잃으므로, 상한을 넘으면 같은 원문으로 호출을 나눈다 —
 * 그래도 원문 전송 횟수는 지적 수가 아니라 호출 수로 줄어든다.
 *
 * 입력 순서를 보존한다. 검증 결과가 지적 번호로 되돌아가야 하므로 items에는 원래 인덱스를 담는다.
 */
export const planVerifyBatches = (
  scenes: SceneRange[],
  groups: { anchors: { matchStart: number | null }[] }[],
  maxPerBatch: number,
): VerifyBatch[] => {
  const buckets = new Map<string, VerifyBatch>();

  for (const [index, group] of groups.entries()) {
    const sceneIndexes = pickScenes(scenes, group.anchors);
    const key = sceneIndexes === null ? 'full' : sceneIndexes.join(',');
    const bucket = buckets.get(key);
    if (bucket) bucket.items.push(index);
    else buckets.set(key, { sceneIndexes, items: [index] });
  }

  const limit = Math.max(1, maxPerBatch);
  const batches: VerifyBatch[] = [];
  for (const bucket of buckets.values()) {
    for (let i = 0; i < bucket.items.length; i += limit) {
      batches.push({ sceneIndexes: bucket.sceneIndexes, items: bucket.items.slice(i, i + limit) });
    }
  }
  return batches;
};
