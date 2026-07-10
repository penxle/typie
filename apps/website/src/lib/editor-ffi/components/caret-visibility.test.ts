import { describe, expect, it } from 'vitest';
import { isCaretVisible } from './caret-visibility';

describe('isCaretVisible', () => {
  it('shows caret when focused with a cursor in edit mode', () => {
    expect(isCaretVisible({ hasCursor: true, hasPoint: true, focused: true, readOnly: false })).toBe(true);
  });

  it('hides caret in read-only mode even when focused with a cursor', () => {
    expect(isCaretVisible({ hasCursor: true, hasPoint: true, focused: true, readOnly: true })).toBe(false);
  });

  it('hides caret when unfocused', () => {
    expect(isCaretVisible({ hasCursor: true, hasPoint: true, focused: false, readOnly: false })).toBe(false);
  });

  it('hides caret without a cursor', () => {
    expect(isCaretVisible({ hasCursor: false, hasPoint: true, focused: true, readOnly: false })).toBe(false);
  });

  it('hides caret without a resolved point', () => {
    expect(isCaretVisible({ hasCursor: true, hasPoint: false, focused: true, readOnly: false })).toBe(false);
  });
});
