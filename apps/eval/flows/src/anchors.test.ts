import { describe, expect, it } from 'vitest';
import { mergeAnchors } from './anchors.ts';

const anchor = (quoteStart: string, matchStart: number | null, matchEnd: number | null, quoteEnd = quoteStart) => ({
  quoteStart,
  quoteEnd,
  matchStart,
  matchEnd,
});

describe('mergeAnchors', () => {
  it('겹치는 구간을 하나로 합치고 바깥 경계를 남긴다', () => {
    const merged = mergeAnchors([anchor('가', 100, 200), anchor('나', 150, 320)]);
    expect(merged).toHaveLength(1);
    expect(merged[0].matchStart).toBe(100);
    expect(merged[0].matchEnd).toBe(320);
    expect(merged[0].quoteStart).toBe('가');
    expect(merged[0].quoteEnd).toBe('나');
  });

  it('한 문장이 안 되는 간격은 같은 대목으로 본다', () => {
    expect(mergeAnchors([anchor('가', 100, 200), anchor('나', 202, 300)])).toHaveLength(1);
  });

  it('떨어진 구간은 반복 발생이므로 남긴다', () => {
    const merged = mergeAnchors([anchor('가', 100, 200), anchor('나', 3000, 3100)]);
    expect(merged).toHaveLength(2);
    expect(merged.map((a) => a.matchStart)).toEqual([100, 3000]);
  });

  it('입력 순서와 무관하게 위치 순으로 정렬한다', () => {
    const merged = mergeAnchors([anchor('나', 3000, 3100), anchor('가', 100, 200)]);
    expect(merged.map((a) => a.quoteStart)).toEqual(['가', '나']);
  });

  it('한쪽이 다른 쪽을 완전히 품으면 넓은 쪽 경계를 유지한다', () => {
    const merged = mergeAnchors([anchor('넓게', 100, 900), anchor('좁게', 300, 400)]);
    expect(merged).toHaveLength(1);
    expect(merged[0].matchEnd).toBe(900);
    expect(merged[0].quoteEnd).toBe('넓게');
  });

  it('위치를 못 찾은 앵커는 인용문이 같을 때만 합친다', () => {
    const merged = mergeAnchors([anchor('미해결', null, null), anchor('미해결', null, null), anchor('다른 미해결', null, null)]);
    expect(merged).toHaveLength(2);
  });

  it('빈 입력은 빈 결과를 낸다', () => {
    expect(mergeAnchors([])).toEqual([]);
  });
});
