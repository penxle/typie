import { describe, expect, it } from 'vitest';
import { assembleItems } from './run-view.ts';

const rows: { id: string; kind: string; ord: number; body: string; facets: Record<string, string> }[] = [
  { id: 'a', kind: 'finding', ord: 0, body: '지적', facets: { layer: 'plan' } },
  { id: 'b', kind: 'pattern', ord: 0, body: '무늬', facets: {} },
];
const anchors = [
  { itemId: 'a', ord: 1, startText: '나', endText: '라', matchStart: 30, matchEnd: 40, note: null },
  { itemId: 'a', ord: 0, startText: '가', endText: '다', matchStart: 10, matchEnd: 20, note: null },
];
const links = [{ itemId: 'b', targetItemId: 'a', ord: 0 }];

describe('assembleItems', () => {
  it('앵커를 ord 순으로 붙인다', () => {
    expect(assembleItems(rows, anchors, links)[0].anchors.map((a) => a.matchStart)).toEqual([10, 30]);
  });

  it('링크를 항목 id 배열로 붙인다', () => {
    expect(assembleItems(rows, anchors, links)[1].links).toEqual(['a']);
  });

  it('앵커도 링크도 없으면 빈 배열이다', () => {
    const items = assembleItems([rows[1]], [], []);
    expect(items[0].anchors).toEqual([]);
    expect(items[0].links).toEqual([]);
  });

  it('링크를 ord 순으로 붙인다', () => {
    const many = [
      { itemId: 'b', targetItemId: 'z', ord: 1 },
      { itemId: 'b', targetItemId: 'a', ord: 0 },
    ];
    expect(assembleItems(rows, [], many)[1].links).toEqual(['a', 'z']);
  });
});
