import { describe, expect, it } from 'vitest';
import { CARD_GAP, layoutCards } from './column-layout.ts';
import type { CardEntry } from './column-layout.ts';

const entry = (id: string, desired: number, height = 100): CardEntry => ({ id, desired, height });

describe('layoutCards', () => {
  it('겹치지 않으면 각자 앵커 높이에 선다', () => {
    const { tops } = layoutCards([entry('a', 0), entry('b', 300)], null);
    expect(tops).toEqual({ a: 0, b: 300 });
  });

  it('겹치면 아래로 밀린다', () => {
    const { tops } = layoutCards([entry('a', 0), entry('b', 40)], null);
    expect(tops.b).toBe(100 + CARD_GAP);
  });

  it('활성 카드는 자기 앵커에 고정되고 위 카드가 물러난다', () => {
    const { tops } = layoutCards([entry('a', 200), entry('b', 260)], 'b');
    expect(tops.b).toBe(260);
    expect(tops.a).toBe(260 - CARD_GAP - 100);
  });

  it('위 공간이 모자라면 컬럼 상단에서 멈춘다', () => {
    const { tops } = layoutCards([entry('a', 0), entry('b', 10)], 'b');
    expect(tops.a).toBe(0);
    expect(tops.b).toBe(10);
  });

  it('앵커 없는 활성 카드는 상향 양보를 유발하지 않는다', () => {
    const { tops } = layoutCards([entry('a', 0), entry('b', Infinity)], 'b');
    expect(tops.a).toBe(0);
    expect(tops.b).toBe(100 + CARD_GAP);
  });

  it('앵커 없는 카드는 말미에 쌓인다', () => {
    const { tops } = layoutCards([entry('lost', Infinity), entry('a', 500)], null);
    expect(tops.a).toBe(500);
    expect(tops.lost).toBe(500 + 100 + CARD_GAP);
  });

  it('스페이서는 가장 아래 카드의 바닥을 덮는다', () => {
    const { spacer } = layoutCards([entry('a', 0), entry('b', 300)], null);
    expect(spacer).toBe(300 + 100 + 8);
  });
});
