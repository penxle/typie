import { describe, expect, it } from 'vitest';
import { pickScenes, planVerifyBatches } from './verify-batch.ts';

const SCENES = [
  { start: 0, end: 100 },
  { start: 100, end: 200 },
  { start: 200, end: 300 },
  { start: 300, end: 400 },
];

describe('pickScenes', () => {
  it('앵커가 놓인 장면과 앞뒤 한 장면씩을 고른다', () => {
    expect(pickScenes(SCENES, [{ matchStart: 150 }])).toEqual([0, 1, 2]);
  });

  it('첫 장면·끝 장면에서는 있는 쪽만 고른다', () => {
    expect(pickScenes(SCENES, [{ matchStart: 10 }])).toEqual([0, 1]);
    expect(pickScenes(SCENES, [{ matchStart: 350 }])).toEqual([2, 3]);
  });

  it('앵커가 여럿이면 합집합이다', () => {
    expect(pickScenes(SCENES, [{ matchStart: 10 }, { matchStart: 350 }])).toEqual([0, 1, 2, 3]);
  });

  // null은 전문을 뜻한다. 장면에 얹지 못한 앵커를 임의로 버리면 검증이 근거 없이 통과한다.
  it('위치를 못 찾은 앵커가 있으면 전문을 요구한다', () => {
    expect(pickScenes(SCENES, [{ matchStart: null }])).toBeNull();
    expect(pickScenes(SCENES, [{ matchStart: 10 }, { matchStart: 9999 }])).toBeNull();
  });

  it('장면 지도가 없으면 전문을 요구한다', () => {
    expect(pickScenes([], [{ matchStart: 10 }])).toBeNull();
  });
});

describe('planVerifyBatches', () => {
  it('같은 장면 집합을 읽는 지적을 한 묶음으로 만든다', () => {
    const batches = planVerifyBatches(
      SCENES,
      [{ anchors: [{ matchStart: 150 }] }, { anchors: [{ matchStart: 160 }] }, { anchors: [{ matchStart: 350 }] }],
      8,
    );
    expect(batches).toEqual([
      { sceneIndexes: [0, 1, 2], items: [0, 1] },
      { sceneIndexes: [2, 3], items: [2] },
    ]);
  });

  it('상한을 넘으면 같은 원문으로 호출을 나눈다', () => {
    const groups = Array.from({ length: 5 }, () => ({ anchors: [{ matchStart: 150 }] }));
    const batches = planVerifyBatches(SCENES, groups, 2);
    expect(batches.map((b) => b.items)).toEqual([[0, 1], [2, 3], [4]]);
    expect(batches.every((b) => b.sceneIndexes?.join(',') === '0,1,2')).toBe(true);
  });

  it('전문이 필요한 지적끼리도 함께 묶인다', () => {
    const batches = planVerifyBatches(SCENES, [{ anchors: [{ matchStart: null }] }, { anchors: [{ matchStart: 9999 }] }], 8);
    expect(batches).toEqual([{ sceneIndexes: null, items: [0, 1] }]);
  });

  // 판정 결과를 원래 지적으로 되돌려야 하므로 인덱스가 보존돼야 한다.
  it('모든 지적이 정확히 한 번씩 담긴다', () => {
    const groups = [
      { anchors: [{ matchStart: 10 }] },
      { anchors: [{ matchStart: 350 }] },
      { anchors: [{ matchStart: null }] },
      { anchors: [{ matchStart: 20 }] },
    ];
    const seen = planVerifyBatches(SCENES, groups, 8)
      .flatMap((b) => b.items)
      .toSorted((a, b) => a - b);
    expect(seen).toEqual([0, 1, 2, 3]);
  });

  it('상한이 0 이하여도 최소 1건씩은 나눈다', () => {
    const batches = planVerifyBatches(SCENES, [{ anchors: [{ matchStart: 10 }] }, { anchors: [{ matchStart: 20 }] }], 0);
    expect(batches.map((b) => b.items)).toEqual([[0], [1]]);
  });

  it('지적이 없으면 빈 계획', () => {
    expect(planVerifyBatches(SCENES, [], 8)).toEqual([]);
  });
});
