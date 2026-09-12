import { describe, expect, it } from 'vitest';
import { appendOrder } from './neighbor-order.ts';

const orders = ['a0', 'b0', 'c0'];

describe('appendOrder', () => {
  it('마지막 뒤에 붙인다', () => {
    expect(appendOrder(orders)).toEqual({ lowerOrder: 'c0', upperOrder: null });
    expect(appendOrder([])).toEqual({ lowerOrder: null, upperOrder: null });
  });
});
