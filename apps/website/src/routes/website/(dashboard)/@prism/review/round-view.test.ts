import { describe, expect, it } from 'vitest';
import { describeHeader, describeResult, findRound, isRoundStale, recheckMode } from './round-view.ts';
import type { ReviewRound } from './round-view.ts';

const makeRound = (over: Partial<ReviewRound> = {}): ReviewRound => ({
  id: 'round-1',
  state: 'COMPLETED',
  tier: 'HIGH',
  ordinal: 1,
  issueCount: 0,
  hasDetail: false,
  reaction: null,
  reactionNote: null,
  workflow: { id: 'row-1', prismWorkflowId: 'prism-1' },
  document: { id: 'document-1', title: '제목', entity: { id: 'entity-1', slug: 'slug-1' } },
  rejection: null,
  conclusion: null,
  ...over,
});

const conclusion = (over: Partial<NonNullable<ReviewRound['conclusion']>> = {}) => ({
  patternsCount: 0,
  prioritiesCount: 0,
  strengthsCount: 0,
  ...over,
});

describe('findRound', () => {
  it('workflow.prismWorkflowId로 회차를 찾는다', () => {
    const target = makeRound({ id: 'round-2', workflow: { id: 'row-2', prismWorkflowId: 'prism-2' } });
    expect(findRound([makeRound(), target], 'prism-2')).toBe(target);
  });

  it('workflow.id로는 찾지 않는다', () => {
    const rounds = [makeRound({ workflow: { id: 'row-9', prismWorkflowId: 'prism-1' } })];
    expect(findRound(rounds, 'row-9')).toBeNull();
  });

  it('워크플로가 없는 회차는 건너뛴다', () => {
    expect(findRound([makeRound({ workflow: null })], 'prism-1')).toBeNull();
  });

  it('맞는 회차가 없으면 null이다', () => {
    expect(findRound([makeRound()], 'prism-404')).toBeNull();
  });
});

describe('isRoundStale', () => {
  it('행이 없으면 낡은 것으로 본다', () => {
    expect(isRoundStale(null)).toBe(true);
  });

  it('아직 RUNNING인 행은 낡은 것으로 본다', () => {
    expect(isRoundStale(makeRound({ state: 'RUNNING' }))).toBe(true);
  });

  it('종결된 행은 낡지 않았다', () => {
    expect(isRoundStale(makeRound({ state: 'COMPLETED' }))).toBe(false);
    expect(isRoundStale(makeRound({ state: 'FAILED' }))).toBe(false);
    expect(isRoundStale(makeRound({ state: 'CANCELED' }))).toBe(false);
  });
});

describe('describeResult', () => {
  it('아직 RUNNING인 행은 결과가 없다', () => {
    expect(describeResult(makeRound({ state: 'RUNNING' }))).toBeNull();
  });

  it('중단·실패로 끝난 행은 결과가 없다', () => {
    expect(describeResult(makeRound({ state: 'CANCELED' }))).toBeNull();
    expect(describeResult(makeRound({ state: 'FAILED' }))).toBeNull();
  });

  it('결론이 있어도 COMPLETED가 아니면 결과가 없다', () => {
    const round = makeRound({ state: 'CANCELED', issueCount: 3, conclusion: conclusion({ patternsCount: 1 }) });
    expect(describeResult(round)).toBeNull();
  });

  it('거절된 회차는 사유 없이 거절 표지만 낸다', () => {
    const round = makeRound({ rejection: { message: '거절 사유' } });
    expect(describeResult(round)).toEqual({ kind: 'rejected' });
  });

  it('거절이 아니면 완료 표지를 낸다', () => {
    const round = makeRound({ issueCount: 7, conclusion: conclusion({ patternsCount: 3, prioritiesCount: 2, strengthsCount: 1 }) });
    expect(describeResult(round)).toEqual({ kind: 'completed' });
  });

  it('결론이 없어도 완료면 완료 표지다', () => {
    const round = makeRound({ issueCount: 4 });
    expect(describeResult(round)).toEqual({ kind: 'completed' });
  });
});

describe('describeHeader', () => {
  it('제목이 비면 제목 없음으로 적는다', () => {
    const round = makeRound({ document: { id: 'document-1', title: '', entity: { id: 'entity-1', slug: 'slug-1' } } });
    expect(describeHeader(round).title).toBe('「제목 없음」');
  });

  it('헤더는 제목과 티어 라벨을 낸다', () => {
    const round = makeRound({ tier: 'MEDIUM', ordinal: 3 });
    expect(describeHeader(round)).toEqual({ title: '「제목」', tier: '일반 검토' });
  });
});

describe('recheckMode', () => {
  it('쿼리가 아직 답하지 않았으면 idle이다', () => {
    expect(recheckMode(null, false, 'completed')).toBe('idle');
    expect(recheckMode(makeRound({ state: 'RUNNING' }), false, 'completed')).toBe('idle');
  });

  it('행이 없으면 진행 중이어도 stale이고, RUNNING 행이 있으면 진행 중엔 idle이다', () => {
    expect(recheckMode(null, true, 'running')).toBe('stale');
    expect(recheckMode(makeRound({ state: 'RUNNING' }), true, 'running')).toBe('idle');
  });

  it('종결된 행을 받았으면 settled다', () => {
    expect(recheckMode(makeRound({ state: 'COMPLETED' }), true, 'completed')).toBe('settled');
    expect(recheckMode(makeRound({ state: 'FAILED' }), true, 'running')).toBe('settled');
  });

  it('완료된 워크플로인데 행이 없거나 아직 RUNNING이면 stale이다', () => {
    expect(recheckMode(null, true, 'completed')).toBe('stale');
    expect(recheckMode(makeRound({ state: 'RUNNING' }), true, 'completed')).toBe('stale');
  });

  it('중단·실패로 끝난 워크플로는 RUNNING 행을 다시 조회하지 않지만, 행이 없으면 조회한다', () => {
    expect(recheckMode(makeRound({ state: 'RUNNING' }), true, 'canceled')).toBe('idle');
    expect(recheckMode(makeRound({ state: 'RUNNING' }), true, 'failed')).toBe('idle');
    expect(recheckMode(null, true, 'canceled')).toBe('stale');
    expect(recheckMode(null, true, 'failed')).toBe('stale');
  });
});
