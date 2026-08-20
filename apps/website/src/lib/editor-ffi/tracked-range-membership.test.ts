import { describe, expect, it, vi } from 'vitest';
import { selectTrackedRangeMember, semanticMembershipForStateChange, trackedRangeMembershipIds } from './tracked-range-membership';
import type { Position, Selection, StateField, TrackedRangeEndpoints } from '@typie/editor-ffi/browser';

const position = (offset: number): Position => ({ node: 'paragraph', offset, affinity: 'downstream' });
const collapsed = (offset: number): Selection => ({ anchor: position(offset), head: position(offset) });
const member = (id: string, group: string): TrackedRangeEndpoints => ({
  id,
  group,
  anchor: position(0),
  head: position(1),
});

describe('tracked range membership', () => {
  it.each<StateField>(['selection', 'doc', 'tracked_ranges'])('%s change refreshes membership under a stationary cursor', (field) => {
    const query = vi.fn(() => [member('a', 'comment')]);

    const selection = collapsed(3);
    expect(semanticMembershipForStateChange(new Set([field]), selection, query)).toEqual([member('a', 'comment')]);
    expect(query).toHaveBeenCalledWith(selection);
  });

  it('queries membership for non-collapsed selections', () => {
    const query = vi.fn(() => [member('a', 'comment')]);
    const expanded: Selection = { anchor: position(1), head: position(2) };

    expect(semanticMembershipForStateChange(new Set<StateField>(['selection']), expanded, query)).toEqual([member('a', 'comment')]);
    expect(query).toHaveBeenCalledWith(expanded);
  });

  it('does not query when the selection is unavailable', () => {
    const query = vi.fn(() => [member('a', 'comment')]);

    expect(semanticMembershipForStateChange(new Set<StateField>(['selection']), undefined, query)).toBeUndefined();
    expect(query).not.toHaveBeenCalled();
  });

  it('does not query for layout-only changes', () => {
    const query = vi.fn(() => [member('a', 'comment')]);

    expect(semanticMembershipForStateChange(new Set<StateField>(['page_sizes', 'cursor']), collapsed(1), query)).toBeUndefined();
    expect(query).not.toHaveBeenCalled();
  });

  it('prefers an eligible active ID, otherwise keeps Core fallback order', () => {
    const members = [member('before', 'comment'), member('after', 'comment-active')];
    const groups = new Set(['comment', 'comment-active']);

    expect(selectTrackedRangeMember(members, groups, 'after')?.id).toBe('after');
    expect(selectTrackedRangeMember(members, groups, null)?.id).toBe('before');
    expect(selectTrackedRangeMember(members, groups, 'outside')?.id).toBe('before');
  });

  it('filters feature ownership before applying active preference', () => {
    const members = [member('missing', 'spellcheck-active'), member('owned', 'spellcheck')];

    expect(selectTrackedRangeMember(members, new Set(['spellcheck', 'spellcheck-active']), 'missing', new Set(['owned']))?.id).toBe(
      'owned',
    );
  });

  it('keeps membership identity stable across normal-active group projection', () => {
    const groups = new Set(['comment', 'comment-active']);

    expect(trackedRangeMembershipIds([member('a', 'comment'), member('b', 'comment-active')], groups)).toEqual(
      trackedRangeMembershipIds([member('a', 'comment-active'), member('b', 'comment')], groups),
    );
  });
});
