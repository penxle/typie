import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import NoteListReorderTestHost from './note-list-reorder-test-host.svelte';
import { NoteListState } from './note-list-state.svelte';

const { captureException, move, onTerminalDelete, retainRelatedEntity, terminal, toastError } = vi.hoisted(() => ({
  captureException: vi.fn(),
  move: vi.fn(),
  onTerminalDelete: vi.fn<(options: { listener: (noteId: string) => void }) => () => void>(),
  retainRelatedEntity: vi.fn(),
  terminal: {
    listener: undefined as ((noteId: string) => void) | undefined,
    noteIds: new Set<string>(),
  },
  toastError: vi.fn(),
}));

vi.mock('@sentry/sveltekit', () => ({ captureException }));
vi.mock('@typie/ui/notification', () => ({ Toast: { error: toastError } }));
vi.mock('./note-mutation', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./note-mutation')>()),
  getNoteOperationsContext: () => ({ move }),
}));
vi.mock('./note-sync.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./note-sync.svelte')>()),
  getNoteSyncContext: () => ({ onTerminalDelete, retainRelatedEntity }),
}));

type Note = {
  id: string;
  order: string;
  status: string;
};

const note = (id: string, order: string): Note => ({ id, order, status: 'OPEN' });

const moveSuccess = (noteId: string, order: string, siteId = 'site-1') => ({
  status: 'success' as const,
  value: {
    id: noteId,
    content: '',
    color: 'gray',
    order,
    status: 'OPEN',
    updatedAt: '2026-07-29T00:00:00.000Z',
    site: { id: siteId },
  },
});

describe('NoteList reorder reconciliation', () => {
  beforeEach(() => {
    captureException.mockReset();
    move.mockReset();
    onTerminalDelete.mockReset();
    retainRelatedEntity.mockClear();
    terminal.listener = undefined;
    terminal.noteIds.clear();
    onTerminalDelete.mockImplementation(({ listener }: { listener: (noteId: string) => void }) => {
      terminal.listener = listener;
      return vi.fn();
    });
    toastError.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    document.body.replaceChildren();
  });

  it('moves the dragged position immediately while surrounding notes animate', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
      if (Object.hasOwn(this.dataset, 'noteListOwnedItem')) {
        const siblings = [...(this.parentElement?.children ?? [])];
        return new DOMRect(0, siblings.indexOf(this) * 50, 100, 40);
      }
      return new DOMRect(0, 0, 100, 140);
    });

    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-begin-reorder="a"]')?.click();
      await tick();
      await tick();

      const items = [...target.querySelectorAll<HTMLElement>('[data-note-list-owned-item]')];
      const draggedItem = items.find((item) => item.dataset.noteId === 'a');
      const surroundingItems = items.filter((item) => item.dataset.noteId !== 'a');

      expect(items.map((item) => item.dataset.noteId)).toEqual(['b', 'c', 'a']);
      expect(draggedItem?.style.transition).not.toContain('transform 300ms');
      expect(surroundingItems.every((item) => item.style.transition.includes('transform 300ms'))).toBe(true);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('commits the dragged note once when it moves across multiple positions', async () => {
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [
      { id: 'a', order: '100', status: 'OPEN' },
      { id: 'b', order: '200', status: 'OPEN' },
      { id: 'c', order: '300', status: 'OPEN' },
    ];
    const returnedOrder = new Map([
      ['a', '400'],
      ['b', '050'],
      ['c', '075'],
    ]);
    move.mockImplementation(async ({ noteId }: { noteId: string }) => {
      const movedNote = authoritativeNotes.find((note) => note.id === noteId);
      if (!movedNote) throw new Error(`Missing note ${noteId}`);
      return {
        status: 'success',
        value: {
          ...movedNote,
          order: returnedOrder.get(noteId) ?? movedNote.order,
          content: '',
          color: 'gray',
          updatedAt: '2026-07-29T00:00:00.000Z',
          site: { id: 'site-1' },
        },
      };
    });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());

      expect(move).toHaveBeenCalledOnce();
      expect(move).toHaveBeenCalledWith(
        {
          noteId: 'a',
          lowerOrder: '300',
          upperOrder: undefined,
        },
        expect.objectContaining({
          lastKnown: {
            noteId: 'a',
            siteId: 'site-1',
          },
        }),
      );
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('rolls a subscription-gated reorder back without reporting a failure', async () => {
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [
      { id: 'a', order: '100', status: 'OPEN' },
      { id: 'b', order: '200', status: 'OPEN' },
      { id: 'c', order: '300', status: 'OPEN' },
    ];
    move.mockResolvedValue({ status: 'subscription_gated' });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());

      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c']);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('rolls an ordinary reorder failure back and reports it once', async () => {
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [
      { id: 'a', order: '100', status: 'OPEN' },
      { id: 'b', order: '200', status: 'OPEN' },
      { id: 'c', order: '300', status: 'OPEN' },
    ];
    const error = new Error('offline');
    move.mockResolvedValue({ status: 'failure', error });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());

      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c']);
      expect(captureException).not.toHaveBeenCalled();
      expect(toastError).toHaveBeenCalledOnce();
      expect(toastError).toHaveBeenCalledWith('순서를 바꾸지 못했어요.');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('reports an internal reconciliation invariant without double-reporting mutation failures', async () => {
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200')];
    move.mockResolvedValue(moveSuccess('unexpected', '300'));
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'a'],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());

      expect(captureException).toHaveBeenCalledOnce();
      expect(toastError).toHaveBeenCalledOnce();
      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b']);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('restores server order and cancels a later drag when the pending reorder fails', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    move.mockReturnValueOnce(completion.promise);
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
        desiredOrders: [
          ['b', 'c', 'a'],
          ['c', 'b', 'a'],
        ],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());

      target.querySelector<HTMLButtonElement>('[data-test-begin-reorder="b"]')?.click();
      await tick();
      expect(target.querySelector('[data-test-dragging]')).not.toBeNull();

      completion.resolve({ status: 'failure', error: new Error('offline') });
      await vi.waitFor(() => expect(toastError).toHaveBeenCalledOnce());

      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c']);
      expect(target.querySelector('[data-test-dragging]')).toBeNull();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('restores server order when the list unmounts with a pending reorder', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    move.mockReturnValueOnce(completion.promise);
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
      },
    });
    let destroyed = false;

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());
      expect(state.visibleNotes().map(({ note }) => note.id)).toEqual(['b', 'c', 'a']);

      await unmount(component);
      destroyed = true;

      expect(state.visibleNotes().map(({ note }) => note.id)).toEqual(['a', 'b', 'c']);
    } finally {
      completion.resolve({ status: 'failure', error: new Error('late failure') });
      if (!destroyed) await unmount(component);
      target.remove();
    }
  });

  it('discards a pending desired order when terminal deletion changes the list', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    move.mockReturnValueOnce(completion.promise);
    const state = new NoteListState<Note>({
      isTerminallyDeleted: (_siteId, noteId) => terminal.noteIds.has(noteId),
    });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());

      terminal.noteIds.add('b');
      terminal.listener?.('b');
      state.sync({ siteId: 'site-1', status: 'OPEN' }, [note('a', '100'), note('c', '300')]);
      await tick();

      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c']);
      state.finishExiting('b');
      await tick();
      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'c']);

      completion.resolve({ status: 'failure', error: new Error('late failure') });
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('silently adopts the latest server order when list membership changes during reconciliation', async () => {
    const completion = Promise.withResolvers<ReturnType<typeof moveSuccess>>();
    move.mockReturnValueOnce(completion.promise);
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
        membershipChange: [...authoritativeNotes, note('d', '400')],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());

      target.querySelector<HTMLButtonElement>('[data-test-use-membership-change]')?.click();
      await tick();
      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c', 'd']);

      completion.resolve(moveSuccess('a', '400'));
      await tick();
      expect(toastError).not.toHaveBeenCalled();
      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c', 'd']);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('rebases the next move on a same-membership authoritative update during reconciliation', async () => {
    const firstCompletion = Promise.withResolvers<ReturnType<typeof moveSuccess>>();
    const secondCompletion = Promise.withResolvers<ReturnType<typeof moveSuccess>>();
    move.mockReturnValueOnce(firstCompletion.promise).mockReturnValueOnce(secondCompletion.promise);
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300'), note('d', '400')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['d', 'c', 'b', 'a'],
        orderChange: [note('b', '150'), note('a', '250'), note('c', '350'), note('d', '450')],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());

      target.querySelector<HTMLButtonElement>('[data-test-use-order-change]')?.click();
      await tick();
      firstCompletion.resolve(moveSuccess('d', '050'));
      await vi.waitFor(() => expect(move).toHaveBeenCalledTimes(2));

      expect(move.mock.calls[1]?.[0]).toEqual({
        noteId: 'c',
        lowerOrder: '050',
        upperOrder: '150',
      });
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      secondCompletion.resolve(moveSuccess('c', '100'));
      target.remove();
    }
  });

  it('ignores a late failure after an authoritative update has already reached the desired order', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    move.mockReturnValueOnce(completion.promise);
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
        orderChange: [note('b', '100'), note('c', '200'), note('a', '300')],
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());

      target.querySelector<HTMLButtonElement>('[data-test-use-order-change]')?.click();
      await tick();
      completion.resolve({ status: 'failure', error: new Error('late failure') });
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());

      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['b', 'c', 'a']);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('does not apply a late success to a replacement list identity', async () => {
    const completion = Promise.withResolvers<ReturnType<typeof moveSuccess>>();
    move.mockReturnValueOnce(completion.promise);
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
        replacement: {
          identity: { siteId: 'site-2', status: 'OPEN' },
          authoritativeNotes,
          desiredOrders: [['b', 'c', 'a']],
        },
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledOnce());

      target.querySelector<HTMLButtonElement>('[data-test-use-replacement]')?.click();
      await tick();
      completion.resolve(moveSuccess('a', '400'));
      await vi.waitFor(() => expect(target.querySelector('[data-test-reconcile-finished]')).not.toBeNull());

      expect([...target.querySelectorAll('[data-test-reorder]')].map((element) => element.textContent)).toEqual(['a', 'b', 'c']);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('does not let an invalidated reconciliation clear a later drag preference', async () => {
    const oldCompletion = Promise.withResolvers<ReturnType<typeof moveSuccess>>();
    const currentCompletion = Promise.withResolvers<ReturnType<typeof moveSuccess>>();
    let callIndex = 0;
    move.mockImplementation(({ noteId }: { noteId: string }) => {
      const index = callIndex++;
      if (index === 0) return oldCompletion.promise;
      if (index === 1) return currentCompletion.promise;
      return Promise.resolve(moveSuccess(noteId, noteId === 'b' ? '350' : '150', 'site-2'));
    });
    const state = new NoteListState<Note>({ isTerminallyDeleted: () => false });
    const authoritativeNotes = [note('a', '100'), note('b', '200'), note('c', '300')];
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteListReorderTestHost, {
      target,
      props: {
        state,
        authoritativeNotes,
        desiredOrder: ['b', 'c', 'a'],
        replacement: {
          identity: { siteId: 'site-2', status: 'OPEN' },
          authoritativeNotes,
          desiredOrders: [
            ['b', 'c', 'a'],
            ['c', 'b', 'a'],
          ],
        },
      },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledTimes(1));

      target.querySelector<HTMLButtonElement>('[data-test-use-replacement]')?.click();
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-reorder="a"]')?.click();
      await vi.waitFor(() => expect(move).toHaveBeenCalledTimes(2));
      target.querySelector<HTMLButtonElement>('[data-test-reorder="b"]')?.click();
      await tick();

      oldCompletion.resolve(moveSuccess('a', '400'));
      await tick();
      currentCompletion.resolve(moveSuccess('a', '400', 'site-2'));
      await vi.waitFor(() => expect(move).toHaveBeenCalledTimes(3));

      expect(move.mock.calls[2]?.[0]).toMatchObject({ noteId: 'b' });
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
