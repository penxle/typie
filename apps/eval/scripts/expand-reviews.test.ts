import { describe, expect, it } from 'vitest';
import { collectRows, expandReview, renderInserts } from './expand-reviews.ts';

const ids = ['f0', 'f1'];

describe('expandReview', () => {
  it('현행 모양을 전개한다', () => {
    const items = expandReview(
      {
        characterization: '작품 파악',
        strengths: [{ body: '강점', quoteStart: '가', quoteEnd: '나', matchStart: 1, matchEnd: 2 }],
        cleared: [{ axis: '축', note: '무혐의' }],
        patterns: [{ theme: '무늬', body: '패턴', feedbackIndexes: [0] }],
        priority: [{ body: '우선', feedbackIndexes: [1] }],
      },
      ids,
    );
    expect(items.map((i) => i.kind)).toEqual(['characterization', 'strength', 'cleared', 'pattern', 'priority']);
    expect(items.find((i) => i.kind === 'pattern')?.links).toEqual(['f0']);
  });

  it('문자열 strengths를 위치 없는 항목 하나로 읽는다', () => {
    const items = expandReview({ characterization: 'c', strengths: '좋았다' }, ids);
    const strength = items.find((i) => i.kind === 'strength');
    expect(strength?.body).toBe('좋았다');
    expect(strength?.anchors).toEqual([]);
  });

  it('cleared가 없어도 전개한다', () => {
    expect(expandReview({ characterization: 'c' }, ids).map((i) => i.kind)).toEqual(['characterization']);
  });

  it('범위 밖 feedbackIndexes는 버린다', () => {
    const items = expandReview({ characterization: 'c', priority: [{ body: 'p', feedbackIndexes: [9] }] }, ids);
    expect(items.find((i) => i.kind === 'priority')?.links).toEqual([]);
  });

  it('본문이 빈 항목은 만들지 않는다', () => {
    expect(expandReview({ characterization: '  ', patterns: [{ theme: 't', body: '', feedbackIndexes: [] }] }, ids)).toEqual([]);
  });

  it('kind 안에서 ord를 0부터 매긴다', () => {
    const items = expandReview(
      {
        characterization: 'c',
        cleared: [
          { axis: 'a', note: 'n1' },
          { axis: 'b', note: 'n2' },
        ],
      },
      ids,
    );
    expect(items.filter((i) => i.kind === 'cleared').map((i) => i.ord)).toEqual([0, 1]);
  });

  it('객체가 아니면 빈 목록이다', () => {
    expect(expandReview(null, ids)).toEqual([]);
    expect(expandReview('문자열', ids)).toEqual([]);
  });
});

describe('collectRows', () => {
  it('wrangler --json 출력에서 행을 추린다', () => {
    const raw = '\n 🌀 실행 중\n[{"results":[{"runId":"r1","review":"{}","findingIds":"[]"}],"success":true}]';
    expect(collectRows(raw)).toEqual([{ runId: 'r1', review: '{}', findingIds: '[]' }]);
  });

  it('배열이 없으면 빈 목록', () => {
    expect(collectRows('오류만 있음')).toEqual([]);
  });
});

describe('renderInserts', () => {
  it('항목·앵커·연결을 모두 낸다', () => {
    const out = renderInserts([
      {
        runId: 'r1',
        review: JSON.stringify({
          characterization: '파악',
          strengths: [{ body: '강점', quoteStart: '가', quoteEnd: '나', matchStart: 1, matchEnd: 2 }],
          priority: [{ body: '우선', feedbackIndexes: [0] }],
        }),
        findingIds: JSON.stringify(['f0']),
      },
    ]);
    expect(out.filter((s) => s.includes('INTO run_items'))).toHaveLength(3);
    expect(out.filter((s) => s.includes('INTO item_anchors'))).toHaveLength(1);
    expect(out.filter((s) => s.includes('INTO item_links'))).toHaveLength(1);
    expect(out.some((s) => s.includes("'f0'"))).toBe(true);
  });

  it('작은따옴표를 이스케이프한다', () => {
    const out = renderInserts([{ runId: 'r1', review: JSON.stringify({ characterization: "그는 '문'을 열었다" }), findingIds: '[]' }]);
    expect(out[0]).toContain("'그는 ''문''을 열었다'");
  });

  it('위치 없는 앵커는 NULL로 낸다', () => {
    const out = renderInserts([
      {
        runId: 'r1',
        review: JSON.stringify({ characterization: 'c', strengths: '문자열 강점' }),
        findingIds: '[]',
      },
    ]);
    expect(out.some((s) => s.includes('INTO item_anchors'))).toBe(false);
  });
});
