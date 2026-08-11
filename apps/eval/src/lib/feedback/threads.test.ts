import { describe, expect, it } from 'vitest';
import { canClose, canReopen, hasCurrentAnchors, settledGroups } from './threads.ts';

describe('thread state guards', () => {
  it('open만 닫을 수 있다', () => {
    expect(canClose('open')).toBe(true);
    expect(canClose('closed')).toBe(false);
    expect(canClose('resolved')).toBe(false);
    expect(canClose('withdrawn')).toBe(false);
  });

  it('closed만 다시 열 수 있다', () => {
    expect(canReopen('closed')).toBe(true);
    expect(canReopen('open')).toBe(false);
    expect(canReopen('resolved')).toBe(false);
    expect(canReopen('withdrawn')).toBe(false);
  });
});

describe('hasCurrentAnchors', () => {
  it('표시 회차가 만든 스레드는 상태와 무관하게 이 원고 기준이다', () => {
    for (const state of ['open', 'closed', 'resolved', 'withdrawn'] as const) {
      expect(hasCurrentAnchors({ reviewRound: 2, state }, 2)).toBe(true);
    }
  });

  it('승계 스레드는 kept로 갱신된 open만 이 원고 기준이다', () => {
    expect(hasCurrentAnchors({ reviewRound: 1, state: 'open' }, 2)).toBe(true);
    expect(hasCurrentAnchors({ reviewRound: 1, state: 'closed' }, 2)).toBe(false);
    expect(hasCurrentAnchors({ reviewRound: 1, state: 'resolved' }, 2)).toBe(false);
    expect(hasCurrentAnchors({ reviewRound: 1, state: 'withdrawn' }, 2)).toBe(false);
  });
});

describe('settledGroups', () => {
  const thread = (id: string, settledRoundNumber: number | null, start: number) => ({
    id,
    settledRoundNumber,
    anchors: [{ start }],
  });

  it('정리 회차 내림차로 묶고 그룹 안은 문서 순이다', () => {
    const groups = settledGroups([thread('a', 1, 50), thread('b', 2, 30), thread('c', 2, 10), thread('d', 1, 20)]);
    expect(groups.map((group) => group.number)).toEqual([2, 1]);
    expect(groups[0].threads.map((t) => t.id)).toEqual(['c', 'b']);
    expect(groups[1].threads.map((t) => t.id)).toEqual(['d', 'a']);
  });

  it('서수 없는 구 데이터는 최말단 그룹으로 강등된다', () => {
    const groups = settledGroups([thread('a', null, 10), thread('b', 1, 20)]);
    expect(groups.map((group) => group.number)).toEqual([1, null]);
  });

  it('앵커 없는 스레드도 그룹 앞머리로 안전하게 선다', () => {
    const groups = settledGroups([{ id: 'a', settledRoundNumber: 1, anchors: [] }, thread('b', 1, 5)]);
    expect(groups[0].threads.map((t) => t.id)).toEqual(['a', 'b']);
  });
});
