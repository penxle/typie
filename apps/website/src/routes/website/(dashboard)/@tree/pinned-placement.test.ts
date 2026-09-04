import { describe, expect, it } from 'vitest';
import { resolvePinnedOrders } from './pinned-placement';

const items = [
  { id: 'A', pinnedOrder: 'a' },
  { id: 'B', pinnedOrder: 'b' },
  { id: 'C', pinnedOrder: 'c' },
  { id: 'D', pinnedOrder: 'd' },
];

describe('pinned placement', () => {
  it('places before a target using the previous item as the lower bound', () => {
    expect(resolvePinnedOrders(items, ['D'], { targetId: 'B', position: 'before' })).toEqual({ lowerOrder: 'a', upperOrder: 'b' });
  });

  it('places after a target using the next item as the upper bound', () => {
    expect(resolvePinnedOrders(items, ['A'], { targetId: 'B', position: 'after' })).toEqual({ lowerOrder: 'b', upperOrder: 'c' });
  });

  it('uses hidden trailing items as the upper bound', () => {
    expect(resolvePinnedOrders(items, ['A'], { targetId: 'C', position: 'after' })).toEqual({ lowerOrder: 'c', upperOrder: 'd' });
    expect(resolvePinnedOrders(items, ['A'], { targetId: 'D', position: 'after' })).toEqual({ lowerOrder: 'd', upperOrder: null });
  });

  it('moves to the front with no lower bound', () => {
    expect(resolvePinnedOrders(items, ['C'], { targetId: 'A', position: 'before' })).toEqual({ lowerOrder: null, upperOrder: 'a' });
  });

  it('inserts entities that are not pinned yet', () => {
    expect(resolvePinnedOrders(items, ['Z'], { targetId: 'B', position: 'before' })).toEqual({ lowerOrder: 'a', upperOrder: 'b' });
    expect(resolvePinnedOrders(items, ['Z'], { targetId: 'D', position: 'after' })).toEqual({ lowerOrder: 'd', upperOrder: null });
    expect(resolvePinnedOrders([], ['Z'], { targetId: 'B', position: 'before' })).toBeNull();
  });

  it('excludes every dragged entity from the bounds', () => {
    expect(resolvePinnedOrders(items, ['A', 'B'], { targetId: 'C', position: 'after' })).toEqual({ lowerOrder: 'c', upperOrder: 'd' });
    expect(resolvePinnedOrders(items, ['B', 'Z'], { targetId: 'D', position: 'before' })).toEqual({ lowerOrder: 'c', upperOrder: 'd' });
  });

  it('returns null when the drop would keep the current position', () => {
    expect(resolvePinnedOrders(items, ['B'], { targetId: 'A', position: 'after' })).toBeNull();
    expect(resolvePinnedOrders(items, ['B'], { targetId: 'C', position: 'before' })).toBeNull();
    expect(resolvePinnedOrders(items, ['B'], { targetId: 'B', position: 'before' })).toBeNull();
    expect(resolvePinnedOrders(items, ['B'], { targetId: 'B', position: 'after' })).toBeNull();
    expect(resolvePinnedOrders(items, ['A', 'B'], { targetId: 'C', position: 'before' })).toBeNull();
  });

  it('returns null for an unknown target or an empty drag', () => {
    expect(resolvePinnedOrders(items, ['A'], { targetId: 'Z', position: 'before' })).toBeNull();
    expect(resolvePinnedOrders(items, [], { targetId: 'A', position: 'before' })).toBeNull();
  });
});
