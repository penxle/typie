import { describe, expect, it } from 'vitest';
import { displayRoundNumbers, isRejectedResult, pickRounds, rejectionOf, settledRoundOf } from './rounds.ts';

const round = (round: number, status: 'running' | 'completed' | 'failed' | 'canceled', tier = 'high', rejected = false) => ({
  round,
  status,
  tier,
  rejected,
});

describe('pickRounds', () => {
  it('단일 완료 회차 — 그대로 표시하고 재리뷰 가능', () => {
    const picked = pickRounds([round(1, 'completed')]);
    expect(picked.display.round).toBe(1);
    expect(picked.runningLatest).toBeNull();
    expect(picked.failedLatest).toBeNull();
    expect(picked.canRereview).toBe(true);
  });

  it('재리뷰 진행 중 — 최신 running이 표시 회차', () => {
    const picked = pickRounds([round(1, 'completed'), round(2, 'running')]);
    expect(picked.display.round).toBe(2);
    expect(picked.runningLatest?.round).toBe(2);
    expect(picked.canRereview).toBe(false);
  });

  it('재리뷰 실패 — 이전 완료 회차를 표시하고 실패를 배너로', () => {
    const picked = pickRounds([round(1, 'completed'), round(2, 'failed')]);
    expect(picked.display.round).toBe(1);
    expect(picked.failedLatest?.round).toBe(2);
    expect(picked.canRereview).toBe(true);
  });

  it('1회차부터 실패 — 실패 회차가 표시 회차이고 배너 없음', () => {
    const picked = pickRounds([round(1, 'failed')]);
    expect(picked.display.round).toBe(1);
    expect(picked.failedLatest).toBeNull();
    expect(picked.canRereview).toBe(false);
  });

  it('중단도 실패와 같은 배너 규칙, 완료 3회차 누적도 최신 완료가 표시 회차', () => {
    expect(pickRounds([round(1, 'completed'), round(2, 'canceled')]).failedLatest?.round).toBe(2);
    expect(pickRounds([round(1, 'completed'), round(2, 'completed'), round(3, 'completed')]).display.round).toBe(3);
  });

  it('재리뷰는 전 티어에 열린다 — 구 구성 세션의 차단은 구세션 가드의 몫이다', () => {
    expect(pickRounds([round(1, 'completed', 'low')]).canRereview).toBe(true);
    expect(pickRounds([round(1, 'completed', 'medium')]).canRereview).toBe(true);
  });

  it('재검토 거부 — 이전 완료 회차를 표시하고 거부를 배너로, 재리뷰는 여전히 가능', () => {
    const picked = pickRounds([round(1, 'completed'), round(2, 'completed', 'high', true)]);
    expect(picked.display.round).toBe(1);
    expect(picked.rejectedLatest?.round).toBe(2);
    expect(picked.failedLatest).toBeNull();
    expect(picked.canRereview).toBe(true);
  });

  it('1회차부터 거부 — 거부 회차가 본체이고 배너 없음, 재리뷰 불가', () => {
    const picked = pickRounds([round(1, 'completed', 'high', true)]);
    expect(picked.display.round).toBe(1);
    expect(picked.display.rejected).toBe(true);
    expect(picked.rejectedLatest).toBeNull();
    expect(picked.canRereview).toBe(false);
  });
});

describe('isRejectedResult · rejectionOf', () => {
  const rejection = { version: 1, kind: 'rejected', rejected: { category: 'diary', message: '문면', basis: null } };

  it('kind === rejected로만 판별한다 — kind 없는 구 결과·정상 결과는 거부가 아니다', () => {
    expect(isRejectedResult(rejection)).toBe(true);
    expect(isRejectedResult({ version: 1, issues: [] })).toBe(false);
    expect(isRejectedResult({ version: 1, kind: 'feedback', issues: [] })).toBe(false);
    expect(isRejectedResult(null)).toBe(false);
  });

  it('rejectionOf는 거부 본문만 꺼낸다', () => {
    expect(rejectionOf(rejection)).toEqual({ category: 'diary', message: '문면', basis: null });
    expect(rejectionOf({ version: 1, issues: [] })).toBeNull();
    expect(rejectionOf(null)).toBeNull();
  });
});

describe('displayRoundNumbers', () => {
  it('실패 회차는 번호를 얻지 않는다 — 재시도가 실패한 회차의 번호를 이어받는다', () => {
    expect(displayRoundNumbers([round(1, 'completed'), round(2, 'failed'), round(3, 'completed')])).toEqual({ 1: 1, 3: 2 });
  });

  it('중단도 실패와 같다 — 재시도 running이 중단된 회차의 번호로 선다', () => {
    expect(displayRoundNumbers([round(1, 'completed'), round(2, 'canceled'), round(3, 'running')])).toEqual({ 1: 1, 3: 2 });
  });

  it('연속 실패 후 재시도도 같은 번호를 유지한다', () => {
    expect(displayRoundNumbers([round(1, 'completed'), round(2, 'failed'), round(3, 'failed'), round(4, 'running')])).toEqual({
      1: 1,
      4: 2,
    });
  });

  it('전 회차 완료면 내부 번호와 일치한다', () => {
    expect(displayRoundNumbers([round(1, 'completed'), round(2, 'completed')])).toEqual({ 1: 1, 2: 2 });
  });

  it('1회차부터 실패한 세션은 빈 매핑이다', () => {
    expect(displayRoundNumbers([round(1, 'failed')])).toEqual({});
  });

  it('거부 회차도 번호를 얻지 않는다 — 다음 정상 리뷰가 그 번호로 선다', () => {
    expect(displayRoundNumbers([round(1, 'completed'), round(2, 'completed', 'high', true), round(3, 'completed')])).toEqual({
      1: 1,
      3: 2,
    });
  });
});

describe('settledRoundOf', () => {
  const review = (round: number, status: string, startedAt: number, rejected = false) => ({ round, status, startedAt, rejected });

  it('완료 회차의 시작 시각 경계로 정리 회차를 정한다', () => {
    const reviews = [review(1, 'completed', 100), review(2, 'completed', 200)];
    expect(settledRoundOf(150, reviews)).toBe(1); // 1회차 표시 중 작가가 닫음
    expect(settledRoundOf(250, reviews)).toBe(2); // 2회차 재리뷰의 처분(사영은 시작 이후)
  });

  it('실패·중단·실행 중 회차는 경계에서 제외한다', () => {
    const reviews = [review(1, 'completed', 100), review(2, 'failed', 200), review(3, 'running', 300)];
    expect(settledRoundOf(250, reviews)).toBe(1);
  });

  it('거부 회차도 경계에서 제외한다 — 처분을 내리지 않는 회차다', () => {
    const reviews = [review(1, 'completed', 100), review(2, 'completed', 200, true)];
    expect(settledRoundOf(250, reviews)).toBe(1);
  });

  it('시각이 없거나 첫 완료 이전이면 null이다', () => {
    const reviews = [review(1, 'completed', 100)];
    expect(settledRoundOf(null, reviews)).toBeNull();
    expect(settledRoundOf(50, reviews)).toBeNull();
  });
});
