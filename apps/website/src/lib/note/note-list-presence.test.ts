import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import NoteListPresenceTestHost from './note-list-presence-test-host.svelte';
import { NoteListState } from './note-list-state.svelte';
import { NoteSync } from './note-sync.svelte';

const { animateFlip, syncContext } = vi.hoisted(() => ({
  animateFlip: vi.fn(() => Promise.resolve()),
  syncContext: { current: undefined as NoteSync | undefined },
}));

vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('@typie/ui/utils', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@typie/ui/utils')>()),
  animateFlip,
}));
vi.mock('./note-mutation', () => ({ getNoteOperationsContext: () => ({ move: vi.fn() }) }));
vi.mock('./note-sync.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./note-sync.svelte')>()),
  getNoteSyncContext: () => syncContext.current,
}));

const createSync = () =>
  new NoteSync({
    invalidateGlobal: vi.fn(),
    invalidateEntity: vi.fn(),
  });

describe('NoteList exit presence', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    document.body.replaceChildren();
  });

  it('does not start another FLIP after an exit transition removes its wrapper', async () => {
    vi.useFakeTimers();
    const sync = createSync();
    syncContext.current = sync;
    const state = new NoteListState<{ id: string; order: string; status: string }>({
      isTerminallyDeleted: (siteId, noteId) => sync.isTerminallyDeleted(siteId, noteId),
    });
    const deleted = { id: 'a', order: '100', status: 'OPEN' };
    const remaining = { id: 'b', order: '200', status: 'OPEN' };
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListPresenceTestHost, {
      target,
      props: { state, authoritativeNotes: [deleted, remaining] },
    });

    try {
      await tick();
      sync.receiveRemote({ kind: 'DELETED', noteId: deleted.id, siteId: 'site-1' });
      await tick();
      await vi.advanceTimersByTimeAsync(250);
      await tick();

      const wrapper = target.querySelector<HTMLElement>('[data-note-id="a"]');
      const presence = wrapper?.firstElementChild;
      expect(presence).toBeInstanceOf(HTMLElement);
      if (!(presence instanceof HTMLElement)) return;

      expect(presence.style.marginBottom).toBe('calc(-8px)');
      expect(presence.style.transition).toContain('margin-bottom 180ms ease');

      animateFlip.mockClear();
      state.sync({ siteId: 'site-1', status: 'OPEN' }, [remaining]);
      await tick();
      expect(animateFlip).not.toHaveBeenCalled();
      expect(presence.style.gridTemplateRows).toBe('0fr');
      expect(presence.style.marginBottom).toBe('calc(-8px)');
      expect(presence.style.transition).toContain('grid-template-rows 180ms ease');

      const transitionEnd = new Event('transitionend');
      Object.defineProperty(transitionEnd, 'propertyName', { value: 'grid-template-rows' });
      presence.dispatchEvent(transitionEnd);
      await tick();
      expect(state.visibleNotes().map(({ note }) => note.id)).toEqual(['a', 'b']);

      await vi.advanceTimersByTimeAsync(180);
      await tick();
      expect(state.visibleNotes().map(({ note }) => note.id)).toEqual(['b']);
      expect(animateFlip).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps the height collapse when a remote delete and server removal arrive together', async () => {
    vi.useFakeTimers();
    const sync = createSync();
    syncContext.current = sync;
    const state = new NoteListState<{ id: string; order: string; status: string }>({
      isTerminallyDeleted: (siteId, noteId) => sync.isTerminallyDeleted(siteId, noteId),
    });
    const deleted = { id: 'a', order: '100', status: 'OPEN' };
    const remaining = { id: 'b', order: '200', status: 'OPEN' };
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListPresenceTestHost, {
      target,
      props: { state, authoritativeNotes: [deleted, remaining] },
    });

    try {
      await tick();
      animateFlip.mockClear();

      sync.receiveRemote({ kind: 'DELETED', noteId: deleted.id, siteId: 'site-1' });
      state.sync({ siteId: 'site-1', status: 'OPEN' }, [remaining]);
      await tick();
      expect(animateFlip).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(250);
      await tick();

      const presence = target.querySelector<HTMLElement>('[data-note-id="a"]')?.firstElementChild;
      expect(presence).toBeInstanceOf(HTMLElement);
      if (!(presence instanceof HTMLElement)) return;

      expect(presence.style.gridTemplateRows).toBe('0fr');
      expect(presence.style.marginBottom).toBe('calc(-8px)');
      expect(presence.style.transition).toContain('grid-template-rows 180ms ease');

      const transitionEnd = new Event('transitionend');
      Object.defineProperty(transitionEnd, 'propertyName', { value: 'grid-template-rows' });
      presence.dispatchEvent(transitionEnd);
      await tick();
      expect(state.visibleNotes().map(({ note }) => note.id)).toEqual(['a', 'b']);

      await vi.advanceTimersByTimeAsync(180);
      await tick();
      expect(state.visibleNotes().map(({ note }) => note.id)).toEqual(['b']);
      expect(animateFlip).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('preserves a pending deletion while the surface-owned state is remounted', async () => {
    const sync = createSync();
    syncContext.current = sync;
    let pendingDeletion = false;
    const state = new NoteListState<{ id: string; order: string; status: string }>({
      isTerminallyDeleted: (siteId, noteId) => sync.isTerminallyDeleted(siteId, noteId),
      isPendingDeletion: () => pendingDeletion,
    });
    const authoritativeNotes = [{ id: 'a', order: '100', status: 'OPEN' }];
    const target = document.createElement('div');
    document.body.append(target);
    const first = mount(NoteListPresenceTestHost, {
      target,
      props: { state, authoritativeNotes },
    });

    await tick();
    state.markDeleting('a');
    pendingDeletion = true;
    expect(state.visibleNotes()).toMatchObject([{ note: { id: 'a' }, deleting: true }]);
    await unmount(first);

    const second = mount(NoteListPresenceTestHost, {
      target,
      props: { state, authoritativeNotes },
    });
    try {
      await tick();
      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'a' }, presence: 'settled', deleting: true }]);
    } finally {
      await unmount(second);
      target.remove();
    }
  });

  it('continues an exit when the surface-owned state is remounted', async () => {
    const sync = createSync();
    syncContext.current = sync;
    const state = new NoteListState<{ id: string; order: string; status: string }>({
      isTerminallyDeleted: (siteId, noteId) => sync.isTerminallyDeleted(siteId, noteId),
    });
    const identity = { siteId: 'site-1', status: 'OPEN' };
    const authoritativeNotes = [{ id: 'a', order: '100', status: 'OPEN' }];
    const target = document.createElement('div');
    document.body.append(target);
    const first = mount(NoteListPresenceTestHost, {
      target,
      props: { state, authoritativeNotes },
    });

    await tick();
    state.sync(identity, []);
    expect(state.visibleNotes()).toMatchObject([{ note: { id: 'a' }, presence: 'exiting' }]);
    await unmount(first);

    const second = mount(NoteListPresenceTestHost, {
      target,
      props: { state, authoritativeNotes: [] },
    });
    try {
      await tick();
      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'a' }, presence: 'exiting' }]);
    } finally {
      await unmount(second);
      target.remove();
    }
  });
});
