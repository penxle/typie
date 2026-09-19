import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { registerNavigationInterceptor, runNavigation } from '$lib/navigation';
import { DocumentEditingSession, DocumentPreparation } from './session';
import { documentEditing } from './state.svelte';

function fixture(id = 'document') {
  let revision = 0;
  let protectedChanges = false;
  let synced = false;
  let pendingInput = false;
  let stops = 0;
  const changes = new Set<() => void>();
  const protectedListeners = new Set<() => void>();
  const editor: DocumentEditingSession['editor'] = {
    terminal: false,
    get documentRevision() {
      return revision;
    },
    finalizeInput: vi.fn(),
    settlePendingEdits: vi.fn(),
    localEdits: {
      isInputAllowed: () => true,
      get pending() {
        return pendingInput;
      },
      stop: () => {
        stops++;
        return () => {
          stops--;
        };
      },
      onChange: (listener) => {
        changes.add(listener);
        return () => {
          changes.delete(listener);
        };
      },
    },
  };
  const pusher: DocumentEditingSession['pusher'] = {
    stop: vi.fn(),
    captureFailures: 0,
    pushFailed: false,
    unprotectedSince: null,
    unconfirmedSince: null,
    isProtected: () => protectedChanges,
    isSynced: () => synced,
    pushNow: vi.fn(async () => {
      throw new Error('server unavailable');
    }),
    checkpoint: vi.fn(async () => {
      throw new Error('storage unavailable');
    }),
    captureNow: vi.fn(async () => {
      throw new Error('storage unavailable');
    }),
    schedule: vi.fn(),
    onProtectionChange: (listener) => {
      protectedListeners.add(listener);
      return () => {
        protectedListeners.delete(listener);
      };
    },
  };
  const session = new DocumentEditingSession(id, `${id}-pane`, () => id, editor, pusher);
  return {
    session,
    editor,
    pusher,
    get stops() {
      return stops;
    },
    edit: () => {
      revision++;
      for (const notify of changes) notify();
    },
    input: (pending: boolean) => {
      pendingInput = pending;
      for (const notify of changes) notify();
    },
    protect: () => {
      protectedChanges = true;
      for (const notify of protectedListeners) notify();
    },
    sync: () => {
      synced = true;
      protectedChanges = true;
      for (const notify of protectedListeners) notify();
    },
  };
}

describe('document editing preparation', () => {
  let unregisterNavigation: () => void;
  beforeEach(() => {
    unregisterNavigation = registerNavigationInterceptor(({ reason, paneIds }) =>
      documentEditing.prepareDeparture(
        () => (paneIds ? documentEditing.sessions.filter((session) => paneIds.includes(session.paneId)) : documentEditing.sessions),
        reason,
      ),
    );
  });
  afterEach(() => unregisterNavigation());
  afterEach(() => vi.useRealTimers());

  it('shares the active confirmation status with the header through retry and cancellation', async () => {
    const doc = fixture();
    const write = Promise.withResolvers<undefined>();
    doc.pusher.captureFailures = 1;
    doc.pusher.pushFailed = true;
    doc.pusher.checkpoint = () => write.promise;
    expect(doc.session.inspectedSaveStatus).toBeNull();
    const preparation = new DocumentPreparation([doc.session]);
    try {
      const checking = preparation.checkpoint();
      expect(doc.session.inspectedSaveStatus).toBe('pending');
      expect(preparation.getSessionStatus(doc.session)).toBe('pending');
      doc.protect();
      write.resolve(undefined);
      await checking;
      expect(doc.session.inspectedSaveStatus).toBe('sync-failed');
      expect(doc.session.isProtected()).toBe(true);
      preparation.release();
      expect(doc.session.inspectedSaveStatus).toBeNull();
    } finally {
      write.resolve(undefined);
      preparation.release();
      doc.session.dispose();
    }
  });

  it('distinguishes local preservation from server acknowledgement while allowing locally safe departure', () => {
    const doc = fixture();
    const departure = new DocumentPreparation([doc.session]);
    const serverSave = new DocumentPreparation([doc.session], true);
    try {
      doc.protect();
      expect(departure.isProtected()).toBe(true);
      expect(serverSave.isProtected()).toBe(false);
      expect(departure.getSessionStatus(doc.session)).toBe('protected');
      expect(serverSave.getSessionStatus(doc.session)).toBe('pending');

      doc.pusher.pushFailed = true;
      expect(departure.isProtected()).toBe(true);
      expect(departure.getSessionStatus(doc.session)).toBe('sync-failed');
      expect(serverSave.getSessionStatus(doc.session)).toBe('sync-failed');

      doc.sync();
      expect(serverSave.isProtected()).toBe(true);
      expect(departure.getSessionStatus(doc.session)).toBe('synced');
      expect(serverSave.getSessionStatus(doc.session)).toBe('synced');
    } finally {
      departure.release();
      serverSave.release();
      doc.session.dispose();
    }
  });

  it('keeps a known server failure visible during composition without declaring pending input protected', () => {
    const doc = fixture();
    doc.protect();
    doc.pusher.pushFailed = true;
    doc.input(true);

    expect(doc.session.saveStatus).toBe('sync-failed');
    expect(doc.session.isProtected()).toBe(false);
    expect(doc.session.isSynced()).toBe(false);
    doc.input(false);
    expect(doc.session.saveStatus).toBe('sync-failed');
    expect(doc.session.isProtected()).toBe(true);
    doc.session.dispose();
  });

  it('reports confirmed document storage while active composition still blocks departure', () => {
    const doc = fixture();
    doc.sync();
    doc.input(true);

    expect(doc.session.saveStatus).toBe('synced');
    expect(doc.session.isProtected()).toBe(false);
    expect(doc.session.isSynced()).toBe(false);
    doc.input(false);
    expect(doc.session.saveStatus).toBe('synced');
    expect(doc.session.isProtected()).toBe(true);
    doc.session.dispose();
  });

  it('distinguishes pending storage from actual local failure while input is pending', () => {
    const doc = fixture();
    doc.input(true);
    expect(doc.session.saveStatus).toBe('pending');
    doc.pusher.captureFailures = 1;
    expect(doc.session.saveStatus).toBe('pending');
    doc.pusher.pushFailed = true;
    expect(doc.session.saveStatus).toBe('failed');
    doc.protect();
    expect(doc.session.saveStatus).toBe('sync-failed');
    expect(doc.session.isProtected()).toBe(false);
    doc.session.dispose();
  });

  it('keeps a local capture failure pending until the server confirms the changes', () => {
    const doc = fixture();
    doc.pusher.captureFailures = 1;
    expect(doc.session.saveStatus).toBe('pending');
    expect(doc.session.isProtected()).toBe(false);
    doc.sync();
    expect(doc.session.saveStatus).toBe('synced');
    expect(doc.session.isProtected()).toBe(true);
    doc.session.dispose();
  });

  it('does not hide a failed input checkpoint behind server confirmation of already applied edits', () => {
    const doc = fixture();
    doc.sync();
    doc.input(true);
    doc.session.markSaveFailure();
    expect(doc.session.saveStatus).toBe('failed');
    expect(doc.session.isProtected()).toBe(false);
    doc.input(false);
    expect(doc.session.saveStatus).toBe('synced');
    doc.session.dispose();
  });

  it('reports unreadable storage coverage without interrupting the save observer', () => {
    const doc = fixture();
    vi.spyOn(doc.pusher, 'isSynced').mockImplementation(() => {
      throw new Error('changeset encoding failed');
    });
    expect(doc.session.saveStatus).toBe('failed');
    expect(doc.session.isSynced()).toBe(false);
    doc.session.dispose();
  });

  it('captures applied edits immediately without a Svelte effect', () => {
    const doc = fixture();
    doc.edit();
    expect(doc.pusher.schedule).toHaveBeenCalledOnce();
    doc.session.dispose();
    doc.edit();
    expect(doc.pusher.schedule).toHaveBeenCalledOnce();
  });

  it('keeps server-only save details open without making locally protected edits block departure', async () => {
    const doc = fixture();
    doc.protect();
    const off = documentEditing.register(doc.session);
    const syncing = documentEditing.showSaveStatus([doc.session], true);
    expect(documentEditing.operations[0]?.phase).toBe('blocked');
    expect(doc.pusher.pushNow).not.toHaveBeenCalled();
    expect(documentEditing.hasUnprotectedChanges()).toBe(false);
    expect(await runNavigation({ reason: 'leave', paneIds: [doc.session].map((session) => session.paneId) }, () => true)).toBe(true);
    doc.protect();
    expect(documentEditing.operations[0]?.unprotectedSessions).toHaveLength(1);
    doc.sync();
    expect(await syncing).toBe(true);
    off();
  });

  it('holds successful siblings and reports every failed session without disposing either', async () => {
    const first = fixture('first');
    const second = fixture('second');
    first.protect();
    const preparation = new DocumentPreparation([first.session, second.session]);
    expect(await preparation.checkpoint()).toEqual([second.session]);
    expect(first.stops).toBe(1);
    expect(second.stops).toBe(1);
    expect(first.session.disposed).toBe(false);
    expect(second.session.disposed).toBe(false);
    preparation.release();
    expect(first.stops).toBe(0);
    expect(second.stops).toBe(0);
  });

  it('hands an already protected web confirmation to native departure without waiting for a hidden countdown', async () => {
    const doc = fixture();
    const off = documentEditing.register(doc.session);
    try {
      const leaving = runNavigation({ reason: 'reload' }, () => true);
      await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
      documentEditing.markDialogShown();
      doc.protect();
      expect(documentEditing.completed).toBe(true);
      documentEditing.nativePreparations++;
      expect(await leaving).toBe(true);
      expect(doc.stops).toBe(0);
      expect(doc.editor.localEdits.isInputAllowed()).toBe(false);
    } finally {
      documentEditing.nativePreparations = 0;
      expect(doc.stops).toBe(0);
      expect(doc.editor.localEdits.isInputAllowed()).toBe(true);
      off();
    }
  });

  it('one cancelled owner cannot reopen another owner stop', () => {
    const doc = fixture();
    const leave = new DocumentPreparation([doc.session]);
    const reload = new DocumentPreparation([doc.session]);
    expect(doc.editor.finalizeInput).toHaveBeenCalledOnce();
    leave.release();
    leave.release();
    expect(doc.stops).toBe(1);
    reload.release();
    expect(doc.stops).toBe(0);
  });

  it('observes background protection while the checkpoint is pending', async () => {
    vi.useFakeTimers();
    const doc = fixture();
    const checkpoint = Promise.withResolvers<undefined>();
    vi.mocked(doc.pusher.checkpoint).mockReturnValue(checkpoint.promise);
    const preparation = new DocumentPreparation([doc.session]);
    const attempt = preparation.checkpoint();
    await vi.advanceTimersByTimeAsync(1500);
    doc.protect();
    checkpoint.resolve(undefined);
    expect(await attempt).toEqual([]);
    preparation.release();
  });

  it('times out without turning missing coverage into success', async () => {
    vi.useFakeTimers();
    const doc = fixture();
    const checkpoint = Promise.withResolvers<undefined>();
    vi.mocked(doc.pusher.checkpoint).mockReturnValue(checkpoint.promise);
    const preparation = new DocumentPreparation([doc.session]);
    const attempt = preparation.checkpoint();
    let completed = false;
    void attempt.then(() => (completed = true));
    await vi.advanceTimersByTimeAsync(2999);
    expect(completed).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(await attempt).toHaveLength(1);
    expect(preparation.isProtected()).toBe(false);
    preparation.release();
  });

  it('returns an explicit retry error immediately without showing loading feedback', async () => {
    vi.useFakeTimers();
    const doc = fixture();
    const off = documentEditing.register(doc.session);
    const saving = documentEditing.showSaveStatus([doc.session]);
    documentEditing.retry();
    await vi.advanceTimersByTimeAsync(0);
    expect(documentEditing.operations[0]?.phase).toBe('blocked');
    expect(documentEditing.operations[0]?.showProgress).toBe(false);
    documentEditing.cancel();
    expect(await saving).toBe(false);
    off();
  });

  it('distinguishes a pending checkpoint timeout from a later error', async () => {
    vi.useFakeTimers();
    const doc = fixture();
    const checkpoint = Promise.withResolvers<undefined>();
    vi.mocked(doc.pusher.checkpoint).mockImplementationOnce(() => checkpoint.promise);
    const preparation = new DocumentPreparation([doc.session]);
    const attempt = preparation.checkpoint();
    await vi.advanceTimersByTimeAsync(3000);
    expect(await attempt).toHaveLength(1);
    expect(preparation.isProtected()).toBe(false);
    expect(doc.session.saveFailed).toBe(false);

    checkpoint.reject(new Error('storage unavailable'));
    await vi.advanceTimersByTimeAsync(0);
    expect(doc.session.saveFailed).toBe(true);
    doc.protect();
    expect(doc.session.saveFailed).toBe(false);
    preparation.release();
  });

  it('cancel keeps the whole operation and a later successful checkpoint never replays it', async () => {
    const first = fixture('first');
    const second = fixture('second');
    first.protect();
    const off = [documentEditing.register(first.session), documentEditing.register(second.session)];
    const commit = vi.fn();
    const leaving = runNavigation({ reason: 'leave', paneIds: [first.session, second.session].map((session) => session.paneId) }, commit);
    await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
    documentEditing.cancel();
    expect(await leaving).toBe(false);
    second.protect();
    expect(commit).not.toHaveBeenCalled();
    expect(first.stops).toBe(0);
    expect(second.stops).toBe(0);
    for (const dispose of off) dispose();
  });

  it('completes an uncancelled departure only after every failed document recovers', async () => {
    const first = fixture('first');
    const second = fixture('second');
    const off = [documentEditing.register(first.session), documentEditing.register(second.session)];
    const commit = vi.fn(() => true);
    const leaving = runNavigation({ reason: 'leave', paneIds: [first.session, second.session].map((session) => session.paneId) }, commit);
    await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
    first.protect();
    expect(commit).not.toHaveBeenCalled();
    expect(first.stops).toBe(1);
    second.protect();
    expect(await leaving).toBe(true);
    expect(commit).toHaveBeenCalledOnce();
    expect(documentEditing.operations).toHaveLength(0);
    for (const dispose of off) dispose();
  });

  it('a replaced session invalidates an approval instead of committing against its replacement', async () => {
    const doc = fixture();
    const off = documentEditing.register(doc.session);
    const commit = vi.fn();
    const leaving = runNavigation({ reason: 'leave', paneIds: [doc.session].map((session) => session.paneId) }, commit);
    off();
    expect(await leaving).toBe(false);
    expect(commit).not.toHaveBeenCalled();
  });

  it('includes an editor loaded during a global departure before authorizing navigation', async () => {
    const first = fixture('first');
    const late = fixture('late');
    const off = [documentEditing.register(first.session)];
    const commit = vi.fn(() => true);
    try {
      const leaving = runNavigation({ reason: 'leave' }, commit);
      await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
      off.push(documentEditing.register(late.session));
      first.protect();

      await vi.waitFor(() => expect(late.stops).toBe(1));
      expect(first.stops).toBe(1);
      expect(commit).not.toHaveBeenCalled();
      late.protect();
      expect(await leaving).toBe(true);
      expect(first.stops).toBe(0);
      expect(late.stops).toBe(0);
    } finally {
      documentEditing.cancel();
      for (const dispose of off) dispose();
    }
  });

  it('includes only late editors in the requested panes and releases them all on cancellation', async () => {
    const first = fixture('first');
    const late = fixture('late');
    const unrelated = fixture('unrelated');
    const off = [documentEditing.register(first.session)];
    const commit = vi.fn();
    try {
      const leaving = runNavigation({ reason: 'leave', paneIds: [first.session.paneId, late.session.paneId] }, commit);
      await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
      off.push(documentEditing.register(late.session), documentEditing.register(unrelated.session));
      first.protect();

      await vi.waitFor(() => expect(late.stops).toBe(1));
      expect(unrelated.stops).toBe(0);
      expect(commit).not.toHaveBeenCalled();
      documentEditing.cancel();
      expect(await leaving).toBe(false);
      expect(first.stops).toBe(0);
      expect(late.stops).toBe(0);
      late.protect();
      expect(commit).not.toHaveBeenCalled();
    } finally {
      documentEditing.cancel();
      for (const dispose of off) dispose();
    }
  });

  it('does not extend a discard decision to an editor loaded after that confirmation began', async () => {
    const first = fixture('first');
    const late = fixture('late');
    const off = [documentEditing.register(first.session)];
    const commit = vi.fn(() => true);
    try {
      const leaving = runNavigation({ reason: 'leave' }, commit);
      await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
      off.push(documentEditing.register(late.session));
      documentEditing.discard();

      await vi.waitFor(() => expect(late.stops).toBe(1));
      await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
      expect(commit).not.toHaveBeenCalled();
      documentEditing.discard();
      expect(await leaving).toBe(true);
      expect(first.stops).toBe(0);
      expect(late.stops).toBe(0);
    } finally {
      documentEditing.cancel();
      for (const dispose of off) dispose();
    }
  });

  it('does not block a pane departure on a new editor outside the requested panes', async () => {
    const first = fixture('first');
    const unrelated = fixture('unrelated');
    const off = [documentEditing.register(first.session)];
    const commit = vi.fn(() => true);
    try {
      const leaving = runNavigation({ reason: 'leave', paneIds: [first.session.paneId] }, commit);
      await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
      off.push(documentEditing.register(unrelated.session));
      first.protect();

      expect(await leaving).toBe(true);
      expect(commit).toHaveBeenCalledOnce();
      expect(unrelated.stops).toBe(0);
      expect(unrelated.session.isProtected()).toBe(false);
    } finally {
      documentEditing.cancel();
      for (const dispose of off) dispose();
    }
  });

  it.each([0, 1])('invalidates a completed preparation with %i initial sessions if another editor loads before commit', async (count) => {
    const first = fixture('first');
    const late = fixture('late');
    first.protect();
    const off = count > 0 ? [documentEditing.register(first.session)] : [() => first.session.dispose()];
    const preparation = await documentEditing.prepareDeparture(() => documentEditing.sessions, 'leave');
    try {
      if (!preparation) throw new Error('Preparation was cancelled');
      expect(preparation.isCurrent()).toBe(true);
      off.push(documentEditing.register(late.session));
      expect(preparation.isCurrent()).toBe(false);
    } finally {
      if (preparation) preparation.release();
      for (const dispose of off) dispose();
    }
    expect(first.stops).toBe(0);
  });

  it('releases stopped editors if the navigation owner is replaced before approval', async () => {
    const doc = fixture();
    const off = documentEditing.register(doc.session);
    const commit = vi.fn();
    const leaving = runNavigation({ reason: 'leave' }, commit);
    await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
    unregisterNavigation();
    doc.protect();
    expect(await leaving).toBe(false);
    expect(commit).not.toHaveBeenCalled();
    expect(doc.stops).toBe(0);
    off();
  });

  it('releases a successful preparation when the actual navigation fails', async () => {
    const doc = fixture();
    doc.protect();
    const off = documentEditing.register(doc.session);
    await expect(
      runNavigation({ reason: 'leave' }, () => {
        expect(doc.stops).toBe(1);
        throw new Error('navigation failed');
      }),
    ).rejects.toThrow('navigation failed');
    expect(doc.stops).toBe(0);
    expect(documentEditing.operations).toHaveLength(0);
    off();
  });

  it('a reload failure stays stopped and marked failed until protection recovers, even if cancel is requested', async () => {
    const doc = fixture();
    const off = documentEditing.register(doc.session);
    const commit = vi.fn(() => true);
    const reload = runNavigation({ reason: 'reload', paneIds: [doc.session].map((session) => session.paneId) }, commit);
    await vi.waitFor(() => expect(documentEditing.operations[0]?.phase).toBe('blocked'));
    expect(doc.session.saveFailed).toBe(true);
    documentEditing.cancel();
    expect(doc.stops).toBe(1);
    expect(commit).not.toHaveBeenCalled();
    doc.protect();
    expect(await reload).toBe(true);
    expect(doc.session.saveFailed).toBe(false);
    expect(doc.stops).toBe(0);
    off();
  });
});
