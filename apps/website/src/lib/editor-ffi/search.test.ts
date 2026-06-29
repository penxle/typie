import { describe, expect, it } from 'vitest';
import { pickMatchSelection } from './editor.svelte';

const matchSelection = {
  anchor: { node: 't1', offset: 4, affinity: 'downstream' },
  head: { node: 't1', offset: 6, affinity: 'downstream' },
} as const;

describe('pickMatchSelection', () => {
  it('uses the live range when it is present (reflects positions shifted by edits)', () => {
    const liveRange = {
      anchor: { node: 't1', offset: 10, affinity: 'downstream' as const },
      head: { node: 't1', offset: 12, affinity: 'downstream' as const },
    };
    expect(pickMatchSelection(matchSelection, liveRange)).toEqual({
      anchor: liveRange.anchor,
      head: liveRange.head,
    });
  });

  it('falls back to the match selection when the live range is missing', () => {
    // TR-252: search() 직후엔 stale한 tracked range를 넘기지 않으려고 호출부가 live를 가린다.
    // 이때 검색으로 갓 계산한 match 위치를 그대로 써야 한다.
    expect(pickMatchSelection(matchSelection)).toEqual(matchSelection);
  });
});
