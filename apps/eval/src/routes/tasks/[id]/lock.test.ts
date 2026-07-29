import { describe, expect, it } from 'vitest';
import { judgmentLock, releasable, stageIndexOf } from './lock.ts';

describe('judgmentLock', () => {
  it('라운드가 비활성이면 잠긴다', () => {
    expect(judgmentLock({ active: false }, { draft: true })).toBe('round-inactive');
  });

  it('비활성이 제출 여부보다 먼저다', () => {
    expect(judgmentLock({ active: false }, { draft: false })).toBe('round-inactive');
  });

  it('제출된 판정은 잠긴다', () => {
    expect(judgmentLock({ active: true }, { draft: false })).toBe('submitted');
  });

  it('활성 라운드의 임시저장은 열린다', () => {
    expect(judgmentLock({ active: true }, { draft: true })).toBeNull();
  });
});

describe('stageIndexOf', () => {
  const stage = { key: 's', label: '단계', run: [], items: [] };
  const two = { stages: [stage, { ...stage, key: 't' }] };

  it('확정된 단계 수가 곧 현재 단계다', () => {
    expect(stageIndexOf(two, { stage: 0 })).toBe(0);
    expect(stageIndexOf(two, { stage: 1 })).toBe(1);
  });

  it('범위를 벗어나면 마지막 단계로 죈다', () => {
    expect(stageIndexOf(two, { stage: 5 })).toBe(1);
    expect(stageIndexOf({ stages: [stage] }, { stage: 1 })).toBe(0);
  });
});

describe('releasable', () => {
  it('확정된 단계가 있으면 반납할 수 없다', () => {
    expect(releasable({ stage: 0 })).toBe(true);
    expect(releasable({ stage: 1 })).toBe(false);
  });
});
