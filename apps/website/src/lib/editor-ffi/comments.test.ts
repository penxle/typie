import { describe, expect, it } from 'vitest';
import { canDeleteComment, canManageThread, canUpdateComment, isRootComment, reconcileComments } from './comments';

describe('reconcileComments', () => {
  it('adds desired-not-registered and removes registered-not-desired', () => {
    expect(reconcileComments(['a', 'b'], ['b', 'c'])).toEqual({ toAdd: ['c'], toRemove: ['a'] });
  });
  it('empty when in sync (order-independent)', () => {
    expect(reconcileComments(['b', 'a'], ['a', 'b'])).toEqual({ toAdd: [], toRemove: [] });
  });
});

describe('permission helpers', () => {
  const me = 'user-1';
  const stranger = 'user-2';

  it('isRootComment: first comment is root', () => {
    const thread = { comments: [{ id: 'c1' }, { id: 'c2' }] };
    expect(isRootComment(thread, 'c1')).toBe(true);
    expect(isRootComment(thread, 'c2')).toBe(false);
  });

  it('canUpdateComment: author only', () => {
    expect(canUpdateComment({ user: { id: me } }, me)).toBe(true);
    expect(canUpdateComment({ user: { id: stranger } }, me)).toBe(false);
  });

  it('canDeleteComment: author or owner, never root', () => {
    const thread = {
      comments: [
        { id: 'r', user: { id: me } },
        { id: 'x', user: { id: stranger } },
      ],
    };
    expect(canDeleteComment(thread, 'r', me, true)).toBe(false);
    expect(canDeleteComment(thread, 'x', stranger, false)).toBe(true);
    expect(canDeleteComment(thread, 'x', me, true)).toBe(true);
    expect(canDeleteComment(thread, 'x', 'user-3', false)).toBe(false);
  });

  it('canManageThread: thread author or owner', () => {
    expect(canManageThread({ user: { id: me } }, me, false)).toBe(true);
    expect(canManageThread({ user: { id: stranger } }, me, true)).toBe(true);
    expect(canManageThread({ user: { id: stranger } }, me, false)).toBe(false);
  });
});
