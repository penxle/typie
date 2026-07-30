import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteActions } from './note-actions.svelte';
import { NoteListState } from './note-list-state.svelte';

const { deleteNote, onTerminalDelete, terminal, toastError, updateNote } = vi.hoisted(() => ({
  deleteNote: vi.fn(),
  onTerminalDelete: vi.fn<(options: { listener: (noteId: string) => void }) => () => void>(),
  terminal: {
    listener: undefined as ((noteId: string) => void) | undefined,
    noteIds: new Set<string>(),
  },
  toastError: vi.fn(),
  updateNote: vi.fn(),
}));

vi.mock('@typie/ui/notification', () => ({ Toast: { error: toastError } }));
vi.mock('./note-mutation', () => ({
  getNoteOperationsContext: () => ({
    delete: deleteNote,
    update: updateNote,
  }),
}));
vi.mock('./note-sync.svelte', () => ({
  getNoteSyncContext: () => ({
    isTerminallyDeleted: (_siteId: string, noteId: string) => terminal.noteIds.has(noteId),
    onTerminalDelete,
  }),
}));

type Note = {
  id: string;
  order: string;
  status: 'OPEN' | 'RESOLVED';
  updatedAt: string;
};

const note = (overrides: Partial<Note> = {}): Note => ({
  id: 'note-1',
  order: '100',
  status: 'OPEN',
  updatedAt: '2026-07-29T00:00:00.000Z',
  ...overrides,
});

const observeStatus = (actions: NoteActions<Note>, notes: readonly Note[]) => {
  actions.syncStatus({ siteId: 'site-1', notes, visibleStatuses: ['OPEN', 'RESOLVED'] });
};

describe('NoteActions', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    deleteNote.mockReset();
    onTerminalDelete.mockReset();
    terminal.listener = undefined;
    terminal.noteIds.clear();
    onTerminalDelete.mockImplementation(({ listener }: { listener: (noteId: string) => void }) => {
      terminal.listener = listener;
      return vi.fn();
    });
    toastError.mockReset();
    updateNote.mockReset();
  });

  it('restores a pending delete after an ordinary failure and reports it once', async () => {
    const error = new Error('offline');
    deleteNote.mockResolvedValue({ status: 'failure', error });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [note()]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      const deletion = actions.delete({
        noteId: 'note-1',
        siteId: 'site-1',
        state,
      });

      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'settled', deleting: true }]);
      await deletion;

      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'settled', deleting: false }]);
      expect(toastError).toHaveBeenCalledOnce();
      expect(toastError).toHaveBeenCalledWith('노트를 삭제하지 못했어요.');
    } finally {
      dispose();
    }
  });

  it('clears pending state and invokes surface cleanup once through terminal dispatch', async () => {
    deleteNote.mockImplementation(async () => {
      terminal.listener?.('note-1');
      return { status: 'success', value: note() };
    });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [note()]);
    const onTerminal = vi.fn();
    const dispose = actions.activate({ siteId: 'site-1', onTerminal });

    try {
      await actions.delete({
        noteId: 'note-1',
        siteId: 'site-1',
        state,
      });

      expect(actions.isPendingDeletion('note-1')).toBe(false);
      expect(onTerminal).toHaveBeenCalledOnce();
      expect(onTerminal).toHaveBeenCalledWith('note-1');
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      dispose();
    }
  });

  it('ignores a late delete failure after terminal deletion has already converged', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    deleteNote.mockReturnValue(completion.promise);
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: (_siteId, noteId) => terminal.noteIds.has(noteId),
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [note()]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      const deletion = actions.delete({
        noteId: 'note-1',
        siteId: 'site-1',
        state,
      });
      terminal.noteIds.add('note-1');
      terminal.listener?.('note-1');

      completion.resolve({ status: 'failure', error: new Error('late failure') });
      await deletion;

      expect(toastError).not.toHaveBeenCalled();
    } finally {
      dispose();
    }
  });

  it('rolls a subscription-gated resolve back without reporting a failure', async () => {
    updateNote.mockResolvedValue({ status: 'subscription_gated' });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    const openNote = note();
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      const update = actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      expect(actions.isResolving('note-1')).toBe(true);
      await update;

      expect(actions.isResolving('note-1')).toBe(false);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      dispose();
    }
  });

  it('rolls an ordinary status failure back and reports it once', async () => {
    const error = new Error('offline');
    updateNote.mockResolvedValue({ status: 'failure', error });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    const openNote = note();
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      expect(actions.isResolving('note-1')).toBe(false);
      expect(toastError).toHaveBeenCalledOnce();
      expect(toastError).toHaveBeenCalledWith('상태를 바꾸지 못했어요.');
    } finally {
      dispose();
    }
  });

  it('does not send a duplicate OPEN update while a resolved-note reopen is pending', async () => {
    const completion = Promise.withResolvers<{ status: 'success'; value: Note }>();
    updateNote.mockReturnValue(completion.promise);
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    const resolvedNote = note({ status: 'RESOLVED' });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, []);
    observeStatus(actions, [resolvedNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      const first = actions.toggleStatus({
        note: resolvedNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      const second = actions.toggleStatus({
        note: resolvedNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      expect(updateNote).toHaveBeenCalledOnce();
      completion.resolve({ status: 'success', value: note() });
      await Promise.all([first, second]);
    } finally {
      dispose();
    }
  });

  it('does not recreate a completed local source transfer when the authoritative target arrives later', async () => {
    const resolvedNote = note({ status: 'RESOLVED' });
    updateNote.mockResolvedValue({ status: 'success', value: resolvedNote });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    const openNote = note();
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      actions.finishStatusTransfer('note-1', 'OPEN');

      observeStatus(actions, [resolvedNote]);

      expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(true);
      expect(actions.isResolving('note-1')).toBe(false);
    } finally {
      dispose();
    }
  });

  it('ignores a late status failure after terminal deletion has already converged', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    updateNote.mockReturnValue(completion.promise);
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: (_siteId, noteId) => terminal.noteIds.has(noteId),
    });
    const openNote = note();
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      const update = actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      terminal.noteIds.add('note-1');
      terminal.listener?.('note-1');

      completion.resolve({ status: 'failure', error: new Error('late failure') });
      await update;

      expect(actions.isResolving('note-1')).toBe(false);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      dispose();
    }
  });

  it('resumes an active resolve exit when cancellation is subscription-gated', async () => {
    updateNote.mockResolvedValueOnce({ status: 'success', value: note({ status: 'RESOLVED' }) });
    updateNote.mockResolvedValueOnce({ status: 'subscription_gated' });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    const openNote = note();
    const resolvedNote = note({ status: 'RESOLVED' });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      state.sync({ siteId: 'site-1', status: 'OPEN' }, [resolvedNote]);
      observeStatus(actions, [resolvedNote]);

      const cancellation = actions.toggleStatus({
        note: resolvedNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      expect(actions.isCancelling('note-1')).toBe(true);
      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'settled' }]);
      await cancellation;

      expect(actions.isCancelling('note-1')).toBe(false);
      expect(actions.isResolving('note-1')).toBe(true);
      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'exiting' }]);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      dispose();
    }
  });

  it('clears cancellation and resolve state after the server confirms OPEN', async () => {
    const openNote = note();
    const resolvedNote = note({ status: 'RESOLVED' });
    updateNote.mockResolvedValueOnce({ status: 'success', value: resolvedNote });
    updateNote.mockResolvedValueOnce({ status: 'success', value: openNote });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      state.sync({ siteId: 'site-1', status: 'OPEN' }, [resolvedNote]);
      observeStatus(actions, [resolvedNote]);
      await actions.toggleStatus({
        note: resolvedNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      expect(actions.isCancelling('note-1')).toBe(true);
      expect(actions.isResolving('note-1')).toBe(true);
      observeStatus(actions, [openNote]);

      expect(actions.isCancelling('note-1')).toBe(false);
      expect(actions.isResolving('note-1')).toBe(false);
    } finally {
      dispose();
    }
  });

  it('clears resolve state when a remotely reopened note cancels its exit', async () => {
    const openNote = note();
    const resolvedNote = note({ status: 'RESOLVED' });
    updateNote.mockResolvedValueOnce({ status: 'success', value: resolvedNote });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      expect(actions.isResolving('note-1')).toBe(true);

      state.sync({ siteId: 'site-1', status: 'OPEN' }, [resolvedNote]);
      observeStatus(actions, [resolvedNote]);
      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'exiting' }]);

      state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
      observeStatus(actions, [openNote]);

      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'settled' }]);
      expect(actions.isResolving('note-1')).toBe(false);
    } finally {
      dispose();
    }
  });

  it('uses a successful RESOLVED response when the latest snapshot has already reopened the note', async () => {
    const openNote = note({ updatedAt: '2026-07-29T00:00:02.000Z' });
    const resolvedNote = note({ status: 'RESOLVED' });
    updateNote.mockResolvedValue({ status: 'success', value: resolvedNote });
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      expect(actions.isResolving('note-1')).toBe(false);
    } finally {
      dispose();
    }
  });

  it('lets a current OPEN snapshot supersede a late cancellation failure', async () => {
    const openNote = note();
    const resolvedNote = note({ status: 'RESOLVED' });
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    updateNote.mockResolvedValueOnce({ status: 'success', value: resolvedNote });
    updateNote.mockReturnValueOnce(completion.promise);
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const dispose = actions.activate({ siteId: 'site-1' });

    try {
      await actions.toggleStatus({
        note: openNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });
      state.sync({ siteId: 'site-1', status: 'OPEN' }, [resolvedNote]);
      observeStatus(actions, [resolvedNote]);
      const cancellation = actions.toggleStatus({
        note: resolvedNote,
        states: { OPEN: state },
        siteId: 'site-1',
      });

      state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
      observeStatus(actions, [openNote]);
      completion.resolve({ status: 'failure', error: new Error('late failure') });
      await cancellation;

      expect(actions.isCancelling('note-1')).toBe(false);
      expect(actions.isResolving('note-1')).toBe(false);
      expect(state.visibleNotes()).toMatchObject([{ note: { id: 'note-1' }, presence: 'settled' }]);
      expect(toastError).not.toHaveBeenCalled();
    } finally {
      dispose();
    }
  });

  it('ignores a stale status completion after identity replacement', async () => {
    const completion = Promise.withResolvers<{ status: 'failure'; error: Error }>();
    updateNote.mockReturnValue(completion.promise);
    const actions = new NoteActions<Note>();
    const state = new NoteListState<Note>({
      isPendingDeletion: (noteId) => actions.isPendingDeletion(noteId),
      isTerminallyDeleted: () => false,
    });
    const openNote = note();
    state.sync({ siteId: 'site-1', status: 'OPEN' }, [openNote]);
    observeStatus(actions, [openNote]);
    const disposeFirst = actions.activate({ siteId: 'site-1' });

    const update = actions.toggleStatus({
      note: openNote,
      states: { OPEN: state },
      siteId: 'site-1',
    });
    expect(actions.isResolving('note-1')).toBe(true);

    disposeFirst();
    const disposeSecond = actions.activate({ siteId: 'site-2' });
    expect(actions.isResolving('note-1')).toBe(false);

    completion.resolve({ status: 'failure', error: new Error('stale failure') });
    await update;

    expect(actions.isResolving('note-1')).toBe(false);
    expect(toastError).not.toHaveBeenCalled();
    disposeSecond();
  });
});

describe('NoteActions status transfer admission', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    deleteNote.mockReset();
    onTerminalDelete.mockReset();
    terminal.listener = undefined;
    terminal.noteIds.clear();
    onTerminalDelete.mockImplementation(({ listener }: { listener: (noteId: string) => void }) => {
      terminal.listener = listener;
      return vi.fn();
    });
    toastError.mockReset();
    updateNote.mockReset();
  });

  const syncStatus = (
    actions: NoteActions<Note>,
    notes: readonly Note[],
    visibleStatuses: readonly Note['status'][] = ['OPEN', 'RESOLVED'],
    entityId = 'entity-1',
  ) => {
    actions.syncStatus({ siteId: 'site-1', entityId, notes, visibleStatuses });
  };

  it('uses the first observation as a baseline without suppressing it', () => {
    const actions = new NoteActions<Note>();

    syncStatus(actions, [note({ status: 'RESOLVED' })]);

    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(true);
    expect(actions.isResolving('note-1')).toBe(false);
  });

  it('keeps a visible OPEN source while suppressing its remote RESOLVED target until exit completes', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note()]);

    syncStatus(actions, [note({ status: 'RESOLVED' })]);

    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(false);
    expect(actions.isResolving('note-1')).toBe(true);

    actions.finishStatusTransfer('note-1', 'OPEN');
    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(true);
  });

  it('keeps a visible RESOLVED source while suppressing its remote OPEN target until exit completes', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note({ status: 'RESOLVED' })]);

    syncStatus(actions, [note()]);

    expect(actions.isStatusAdmitted('note-1', 'OPEN')).toBe(false);
    actions.finishStatusTransfer('note-1', 'RESOLVED');
    expect(actions.isStatusAdmitted('note-1', 'OPEN')).toBe(true);
  });

  it('admits a destination immediately when the previous source section is hidden', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note({ status: 'RESOLVED' })], ['OPEN']);

    syncStatus(actions, [note()], ['OPEN']);

    expect(actions.isStatusAdmitted('note-1', 'OPEN')).toBe(true);
  });

  it('releases a transfer when its source section becomes hidden', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note()]);
    syncStatus(actions, [note({ status: 'RESOLVED' })]);
    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(false);

    syncStatus(actions, [note({ status: 'RESOLVED' })], ['RESOLVED']);

    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(true);
  });

  it('removes the transfer when the authoritative status reverses before source exit completes', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note()]);
    syncStatus(actions, [note({ status: 'RESOLVED' })]);

    syncStatus(actions, [note({ updatedAt: '2026-07-29T00:00:01.000Z' })]);

    expect(actions.isStatusAdmitted('note-1', 'OPEN')).toBe(true);
    expect(actions.isResolving('note-1')).toBe(false);
  });

  it('finishes only the transfer whose source exit completed', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note()]);
    syncStatus(actions, [note({ status: 'RESOLVED' })]);

    actions.finishStatusTransfer('note-1', 'RESOLVED');
    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(false);

    actions.finishStatusTransfer('note-1', 'OPEN');
    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(true);
  });

  it('treats the first observation after identity replacement as a new baseline', () => {
    const actions = new NoteActions<Note>();
    syncStatus(actions, [note()]);
    syncStatus(actions, [note({ status: 'RESOLVED' })]);
    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(false);

    syncStatus(actions, [note({ status: 'RESOLVED' })], ['OPEN', 'RESOLVED'], 'entity-2');

    expect(actions.isStatusAdmitted('note-1', 'RESOLVED')).toBe(true);
    expect(actions.isResolving('note-1')).toBe(false);
  });
});
