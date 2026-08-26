import { describe, expect, it } from 'vitest';
import { lineageRowLabel, pickDefaultLineage } from './lineage-view.ts';

const L = (id: string, latestOrdinal: number, tier: 'low' | 'medium' | 'high', locked = false) => ({ id, tier, latestOrdinal, locked });

describe('pickDefaultLineage', () => {
  it('여백에 표시 중인 회차의 계보가 목록에 있으면 그것', () => {
    expect(pickDefaultLineage([L('a', 1, 'high'), L('b', 2, 'low')], 'b')).toBe('b');
  });
  it('없으면 첫 항목(가장 최근 활동 계보)', () => {
    expect(pickDefaultLineage([L('a', 1, 'high'), L('b', 2, 'low')], null)).toBe('a');
    expect(pickDefaultLineage([L('a', 1, 'high')], 'zzz')).toBe('a');
  });
  it('잠긴 계보는 기본값이 못 된다', () => {
    expect(pickDefaultLineage([L('a', 1, 'high', true), L('b', 2, 'low')], 'a')).toBe('b');
    expect(pickDefaultLineage([L('a', 1, 'high', true)], null)).toBeNull();
  });
});

describe('lineageRowLabel', () => {
  it('N회차에 이어서 · 깊이 라벨', () => {
    expect(lineageRowLabel(L('a', 3, 'high'))).toBe('3회차에 이어서 · 심층 검토');
  });
});
