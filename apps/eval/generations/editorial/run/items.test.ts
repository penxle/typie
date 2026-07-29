import { describe, expect, it } from 'vitest';
import { buildItems } from './items.ts';
import type { BuildItemsInput } from './items.ts';

const input: BuildItemsInput = {
  characterization: '이 원고는 인물의 내면 변화를 사건보다 앞세운다',
  feedbacks: [
    {
      category: '인물 동기',
      layer: 'plan',
      body: '물러서는 동기가 제시되지 않는다',
      anchors: [{ quoteStart: '가', quoteEnd: '나', matchStart: 10, matchEnd: 20 }],
    },
    {
      category: '문장 결',
      layer: 'local',
      body: '같은 수식이 세 문단에서 반복된다',
      anchors: [{ quoteStart: '다', quoteEnd: '라', matchStart: 30, matchEnd: 40 }],
    },
  ],
  strengths: [{ body: '장면 전환을 설명 없이 넘긴다', quoteStart: '마', quoteEnd: '바', matchStart: 5, matchEnd: 9 }],
  cleared: [{ axis: '시간 표기', note: '전부 대조했으나 어긋나는 곳은 없었다' }],
  patterns: [{ theme: '사후 설명', body: '이유를 뒤에서 덧붙인다', feedbackIndexes: [0, 1] }],
  priority: [{ body: '동기 제시부터 손대라', feedbackIndexes: [0] }],
};

describe('buildItems', () => {
  it('여섯 종류를 모두 만든다', () => {
    expect(buildItems(input).map((i) => i.kind)).toEqual([
      'characterization',
      'finding',
      'finding',
      'strength',
      'cleared',
      'pattern',
      'priority',
    ]);
  });

  it('kind 안에서 ord를 0부터 매긴다', () => {
    const items = buildItems(input);
    expect(items.filter((i) => i.kind === 'finding').map((i) => i.ord)).toEqual([0, 1]);
    expect(items.find((i) => i.kind === 'priority')?.ord).toBe(0);
  });

  it('지적의 facets에 축과 층위를 담는다', () => {
    expect(buildItems(input)[1].facets).toEqual({ axis: '인물 동기', layer: 'plan' });
  });

  it('패턴의 링크가 지적의 배열 인덱스를 가리킨다', () => {
    expect(buildItems(input).find((i) => i.kind === 'pattern')?.links).toEqual([1, 2]);
  });

  it('범위 밖 feedbackIndexes는 버린다', () => {
    const items = buildItems({ ...input, priority: [{ body: '우선', feedbackIndexes: [0, 9] }] });
    expect(items.find((i) => i.kind === 'priority')?.links).toEqual([1]);
  });

  it('빈 본문은 항목을 만들지 않는다', () => {
    const items = buildItems({ ...input, characterization: ' '.repeat(3) });
    expect(items.some((i) => i.kind === 'characterization')).toBe(false);
  });

  it('강점의 위치를 앵커 한 개로 담는다', () => {
    expect(buildItems(input).find((i) => i.kind === 'strength')?.anchors).toEqual([
      { quoteStart: '마', quoteEnd: '바', matchStart: 5, matchEnd: 9 },
    ]);
  });

  it('지적이 없으면 총평의 참조는 전부 버려진다', () => {
    const items = buildItems({ ...input, feedbacks: [] });
    expect(items.find((i) => i.kind === 'pattern')?.links).toEqual([]);
  });
});
