import { describe, expect, it } from 'vitest';
import { pickMatchSelection } from './editor.svelte';

const matchSelection = {
  anchor: { node_id: 't1', offset: 4 },
  head: { node_id: 't1', offset: 6 },
} as const;

describe('pickMatchSelection', () => {
  it('uses the live range when it is valid (reflects positions shifted by edits)', () => {
    const liveRange = {
      anchor: { node_id: 't1', offset: 10 },
      head: { node_id: 't1', offset: 12 },
      invalid: false,
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

  it('falls back to the match selection when the live range is invalid', () => {
    const invalidLive = {
      anchor: { node_id: 't1', offset: 2 },
      head: { node_id: 't1', offset: 4 },
      invalid: true,
    };
    expect(pickMatchSelection(matchSelection, invalidLive)).toEqual(matchSelection);
  });
});
