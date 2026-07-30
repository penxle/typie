import { describe, expect, it } from 'vitest';
import { NoteListState } from './note-list-state.svelte';
import type { NoteListIdentity, VisibleNote } from './note-list-state.svelte';

type Note = {
  id: string;
  order: string;
  status: string;
  content: string;
};

const GLOBAL: NoteListIdentity = { siteId: 'site-1', status: 'OPEN' };
const RESOLVED: NoteListIdentity = { siteId: 'site-1', status: 'RESOLVED' };
const RELATED: NoteListIdentity = { siteId: 'site-1', entityId: 'entity-1', status: 'OPEN' };

const note = (id: string, order: string, overrides: Partial<Note> = {}): Note => ({
  id,
  order,
  status: 'OPEN',
  content: id,
  ...overrides,
});

const summary = (visible: readonly VisibleNote<Note>[]) =>
  visible.map(({ note: item, presence, deleting }) => ({
    id: item.id,
    presence,
    deleting,
    content: item.content,
  }));

const ids = (state: NoteListState<Note>) => state.visibleNotes().map(({ note: item }) => item.id);

const createState = (tombstones = new Set<string>()) => ({
  state: new NoteListState<Note>({
    isTerminallyDeleted: (siteId, noteId) => tombstones.has(`${siteId}:${noteId}`),
  }),
  tombstones,
});

describe('NoteListState server presence', () => {
  it('settles the first sync without entry animation and animates a later server addition once', () => {
    const { state } = createState();
    const first = note('a', '100');
    const added = note('b', '200');

    state.sync(GLOBAL, [first]);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'settled', deleting: false, content: 'a' }]);

    state.sync(GLOBAL, [first, added]);
    expect(summary(state.visibleNotes())).toEqual([
      { id: 'a', presence: 'settled', deleting: false, content: 'a' },
      { id: 'b', presence: 'entering', deleting: false, content: 'b' },
    ]);

    state.finishEntering('b');
    state.sync(GLOBAL, [first, added]);
    expect(state.visibleNotes().find(({ note: item }) => item.id === 'b')?.presence).toBe('settled');
  });

  it('retains the previous snapshot as exiting after omission until exit finishes', () => {
    const { state } = createState();
    const original = note('a', '100', { content: 'retained snapshot' });
    state.sync(GLOBAL, [original]);

    state.sync(GLOBAL, []);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'exiting', deleting: false, content: 'retained snapshot' }]);

    state.finishExiting('a');
    expect(state.visibleNotes()).toEqual([]);
  });

  it('suppresses tombstoned notes from stale responses and never clears the shared tombstone on exit completion', () => {
    const { state, tombstones } = createState();
    const deleted = note('a', '100');
    state.sync(GLOBAL, [deleted]);

    expect(state.markDeleting('a')).toBe(true);
    tombstones.add('site-1:a');
    expect(state.markDeleted(deleted)).toBe(true);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'exiting', deleting: false, content: 'a' }]);

    state.finishExiting('a');
    state.sync(GLOBAL, [deleted]);

    expect(state.visibleNotes()).toEqual([]);
    expect(tombstones.has('site-1:a')).toBe(true);
  });

  it('starts an immediate exit for a remotely deleted visible note without a local deleting phase', () => {
    const { state, tombstones } = createState();
    const deleted = note('a', '100');
    state.sync(GLOBAL, [deleted]);
    tombstones.add('site-1:a');

    expect(state.markDeleted(deleted)).toBe(true);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'exiting', deleting: false, content: 'a' }]);
  });

  it('resets without cross-identity enter or exit animations', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('global', '100'), note('second', '200')]);
    state.markDeleting('global');
    state.beginReorder(['second', 'global']);
    state.setDesiredOrder(['second', 'global']);

    state.sync(RELATED, [note('related', '100')]);

    expect(summary(state.visibleNotes())).toEqual([{ id: 'related', presence: 'settled', deleting: false, content: 'related' }]);

    state.sync(RELATED, [note('related', '100'), note('later', '200')]);
    expect(state.visibleNotes().find(({ note: item }) => item.id === 'later')?.presence).toBe('entering');
  });
});

describe('NoteListState restored status exits', () => {
  it('restores an exiting note to settled while its server source remains absent', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100', { content: 'retained' })]);
    state.sync(GLOBAL, []);

    expect(state.restoreExiting('a')).toBe(true);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'settled', deleting: false, content: 'retained' }]);
    expect(state.restoreExiting('a')).toBe(false);
  });

  it('accepts server OPEN confirmation as settled without starting a new entry', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100')]);
    state.sync(GLOBAL, []);
    state.restoreExiting('a');

    state.sync(GLOBAL, [note('a', '100', { content: 'authoritative open' })]);

    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'settled', deleting: false, content: 'authoritative open' }]);
    expect(state.resumeExiting('a')).toBe(false);
  });

  it('resumes only an exit restored by this API when status cancellation fails', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100')]);
    state.sync(GLOBAL, []);
    state.restoreExiting('a');

    expect(state.resumeExiting('a')).toBe(true);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'exiting', deleting: false, content: 'a' }]);
    expect(state.resumeExiting('a')).toBe(false);
  });

  it('does not restore settled notes or resume unrelated and non-restored exits', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('settled', '100'), note('exiting', '200')]);

    expect(state.restoreExiting('settled')).toBe(false);
    expect(state.resumeExiting('settled')).toBe(false);

    state.sync(GLOBAL, [note('settled', '100')]);
    expect(state.resumeExiting('exiting')).toBe(false);
    expect(state.visibleNotes().find(({ note: item }) => item.id === 'exiting')?.presence).toBe('exiting');
  });

  it('cleans restored-exit ownership on identity reset', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100')]);
    state.sync(GLOBAL, []);
    state.restoreExiting('a');

    state.sync(RELATED, [note('related', '100')]);

    expect(state.resumeExiting('a')).toBe(false);
    expect(summary(state.visibleNotes())).toEqual([{ id: 'related', presence: 'settled', deleting: false, content: 'related' }]);
  });

  it('cleans restored-exit ownership when the note becomes terminal and is removed', () => {
    const { state, tombstones } = createState();
    const deleted = note('a', '100');
    state.sync(GLOBAL, [deleted]);
    state.sync(GLOBAL, []);
    state.restoreExiting('a');

    tombstones.add('site-1:a');
    expect(state.markDeleted(deleted)).toBe(true);
    expect(state.resumeExiting('a')).toBe(false);

    state.finishExiting('a');
    expect(state.restoreExiting('a')).toBe(false);
    expect(state.resumeExiting('a')).toBe(false);
  });
});

describe('NoteListState shared pending-deletion admission', () => {
  it('keeps the source settled when an ordinary delete failure releases an unchanged server note', () => {
    const pendingDeletionIds = new Set<string>();
    const state = new NoteListState<Note>({
      isTerminallyDeleted: () => false,
      isPendingDeletion: (noteId) => pendingDeletionIds.has(noteId),
    });
    const original = note('a', '100');
    state.sync(GLOBAL, [original]);
    state.markDeleting('a');
    pendingDeletionIds.add('a');
    state.sync(GLOBAL, [original]);

    pendingDeletionIds.delete('a');
    state.clearDeleting('a');
    state.sync(GLOBAL, [original]);

    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'settled', deleting: false, content: 'a' }]);
  });

  it('keeps the source settled and suppresses target admission until ordinary failure releases the pending deletion', () => {
    const pendingDeletionIds = new Set<string>();
    const options = {
      isTerminallyDeleted: () => false,
      isPendingDeletion: (noteId: string) => pendingDeletionIds.has(noteId),
    };
    const openState = new NoteListState<Note>(options);
    const resolvedState = new NoteListState<Note>(options);
    const open = note('a', '100');
    const resolved = note('a', '100', { status: 'RESOLVED' });
    openState.sync(GLOBAL, [open]);
    resolvedState.sync(RESOLVED, []);

    expect(openState.markDeleting('a')).toBe(true);
    pendingDeletionIds.add('a');
    openState.sync(GLOBAL, [resolved]);
    resolvedState.sync(RESOLVED, [resolved]);

    expect(summary(openState.visibleNotes())).toEqual([{ id: 'a', presence: 'settled', deleting: true, content: 'a' }]);
    expect(resolvedState.visibleNotes()).toEqual([]);

    pendingDeletionIds.delete('a');
    openState.clearDeleting('a');
    openState.sync(GLOBAL, [resolved]);
    resolvedState.sync(RESOLVED, [resolved]);

    expect(summary(openState.visibleNotes())).toEqual([{ id: 'a', presence: 'exiting', deleting: false, content: 'a' }]);
    expect(summary(resolvedState.visibleNotes())).toEqual([{ id: 'a', presence: 'entering', deleting: false, content: 'a' }]);
  });

  it('never admits a stale target snapshot after the pending deletion becomes terminal', () => {
    const pendingDeletionIds = new Set<string>();
    const tombstones = new Set<string>();
    const options = {
      isTerminallyDeleted: (siteId: string, noteId: string) => tombstones.has(`${siteId}:${noteId}`),
      isPendingDeletion: (noteId: string) => pendingDeletionIds.has(noteId),
    };
    const openState = new NoteListState<Note>(options);
    const resolvedState = new NoteListState<Note>(options);
    const open = note('a', '100');
    const resolved = note('a', '100', { status: 'RESOLVED' });
    openState.sync(GLOBAL, [open]);
    resolvedState.sync(RESOLVED, []);
    openState.markDeleting('a');
    pendingDeletionIds.add('a');
    openState.sync(GLOBAL, [resolved]);
    resolvedState.sync(RESOLVED, [resolved]);

    tombstones.add('site-1:a');
    pendingDeletionIds.delete('a');
    expect(openState.markDeleted(open)).toBe(true);
    resolvedState.sync(RESOLVED, [resolved]);

    expect(resolvedState.visibleNotes()).toEqual([]);
  });
});

describe('NoteListState deletion', () => {
  it('keeps a deleting card visible, rejects duplicates, and clears ordinary failure without starting an exit', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100')]);

    expect(state.markDeleting('a')).toBe(true);
    expect(state.markDeleting('a')).toBe(false);
    expect(state.visibleNotes()[0]?.deleting).toBe(true);

    state.clearDeleting('a');
    expect(summary(state.visibleNotes())).toEqual([{ id: 'a', presence: 'settled', deleting: false, content: 'a' }]);
    expect(state.markDeleted(note('missing', '200'))).toBe(false);
  });
});

describe('NoteListState reorder ownership', () => {
  it('previews a local reorder and rolls back to its original snapshot', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);

    expect(state.beginReorder(['c', 'a', 'b'])).toBe(true);
    expect(ids(state)).toEqual(['c', 'a', 'b']);

    state.rollbackOrder();
    expect(ids(state)).toEqual(['a', 'b', 'c']);
  });

  it('does not let an authoritative sync replace an active drag preview and accepts it immediately after rollback', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);
    state.beginReorder(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('b', '100'), note('c', '200'), note('a', '300')]);
    expect(ids(state)).toEqual(['c', 'a', 'b']);

    state.rollbackOrder();
    expect(ids(state)).toEqual(['b', 'c', 'a']);
  });

  it('discards a failed desired order in favor of the latest authoritative order', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);
    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('b', '100'), note('c', '200'), note('a', '300')]);
    expect(ids(state)).toEqual(['c', 'a', 'b']);

    state.rollbackOrder();
    expect(ids(state)).toEqual(['b', 'c', 'a']);
  });

  it('allows another local reorder while the previous commit is awaiting the server', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);
    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);

    expect(state.beginReorder(['c', 'b', 'a'])).toBe(true);
    expect(state.setDesiredOrder(['c', 'b', 'a'])).toBe(true);
    expect(ids(state)).toEqual(['c', 'b', 'a']);
  });

  it('cancels a later drag back to the previous desired order without releasing it', () => {
    const { state } = createState();
    const original = [note('a', '100'), note('b', '200'), note('c', '300')];
    state.sync(GLOBAL, original);
    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);

    state.beginReorder(['c', 'b', 'a']);
    state.rollbackOrder();
    expect(ids(state)).toEqual(['c', 'a', 'b']);

    state.sync(GLOBAL, original);
    expect(ids(state)).toEqual(['c', 'a', 'b']);
  });

  it('keeps a desired order across stale sync, releases it on confirmation, then accepts a later authoritative order', () => {
    const { state } = createState();
    const original = [note('a', '100'), note('b', '200'), note('c', '300')];
    state.sync(GLOBAL, original);

    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);
    state.sync(GLOBAL, original);
    expect(ids(state)).toEqual(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('c', '100'), note('a', '200'), note('b', '300')]);
    expect(ids(state)).toEqual(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('b', '100'), note('c', '200'), note('a', '300')]);
    expect(ids(state)).toEqual(['b', 'c', 'a']);
  });

  it('releases a desired order when the authoritative snapshot confirms common-note order with an added note', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);
    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('c', '100'), note('a', '200'), note('d', '250'), note('b', '300')]);

    expect(ids(state)).toEqual(['c', 'a', 'd', 'b']);
    state.sync(GLOBAL, [note('d', '100'), note('b', '200'), note('c', '300'), note('a', '400')]);
    expect(ids(state)).toEqual(['d', 'b', 'c', 'a']);
  });

  it('releases a desired order when the authoritative snapshot confirms common-note order with a removed note', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);
    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('c', '100'), note('b', '200')]);
    state.finishExiting('a');
    expect(ids(state)).toEqual(['c', 'b']);

    state.sync(GLOBAL, [note('b', '100'), note('c', '200')]);
    expect(ids(state)).toEqual(['b', 'c']);
  });

  it('keeps the optimistic common-note order while the server relative order still conflicts', () => {
    const { state } = createState();
    state.sync(GLOBAL, [note('a', '100'), note('b', '200'), note('c', '300')]);
    state.beginReorder(['c', 'a', 'b']);
    state.setDesiredOrder(['c', 'a', 'b']);

    state.sync(GLOBAL, [note('a', '100'), note('c', '200'), note('b', '300'), note('d', '400')]);

    expect(ids(state)).toEqual(['c', 'a', 'b', 'd']);
  });
});
