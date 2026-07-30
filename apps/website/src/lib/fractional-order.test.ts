import { describe, expect, it } from 'vitest';
import { resolveNextFractionalOrderMove } from './fractional-order';

describe('resolveNextFractionalOrderMove', () => {
  it('advances the first desired mismatch using authoritative server bounds', () => {
    const authoritative = new Map([
      ['a', '100'],
      ['b', '200'],
      ['c', '300'],
    ]);

    expect(resolveNextFractionalOrderMove(authoritative, ['b', 'c', 'a'])).toEqual({
      key: 'b',
      lowerOrder: undefined,
      upperOrder: '100',
    });
  });

  it('moves the dragged key directly to its desired neighbors', () => {
    const authoritative = new Map([
      ['a', '100'],
      ['b', '200'],
      ['c', '300'],
    ]);

    expect(resolveNextFractionalOrderMove(authoritative, ['b', 'c', 'a'], 'a')).toEqual({
      key: 'a',
      lowerOrder: '300',
      upperOrder: undefined,
    });
  });

  it('rebases the next move on the actual order returned by the server', () => {
    const authoritative = new Map([
      ['a', '100'],
      ['b', '050'],
      ['c', '300'],
    ]);

    expect(resolveNextFractionalOrderMove(authoritative, ['b', 'c', 'a'])).toEqual({
      key: 'c',
      lowerOrder: '050',
      upperOrder: '100',
    });
  });

  it('returns null for an already authoritative or incomplete desired order', () => {
    const authoritative = new Map([
      ['a', '100'],
      ['b', '200'],
    ]);

    expect(resolveNextFractionalOrderMove(authoritative, ['a', 'b'])).toBeNull();
    expect(resolveNextFractionalOrderMove(authoritative, ['a'])).toBeNull();
  });
});
