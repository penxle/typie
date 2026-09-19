import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { Editor } from '$lib/editor-ffi/editor.svelte';
import { registerNavigationInterceptor, runNavigation } from '$lib/navigation';
import { Pusher } from '../../routes/website/(dashboard)/[slug]/v2/sync/pusher.svelte';
import { IndexeddbDeltaStore } from '../../routes/website/(dashboard)/[slug]/v2/sync/store';
import { watchDesktopDocumentSave } from './desktop';
import { DocumentEditingSession } from './session';
import { documentEditing } from './state.svelte';
import type { DocumentSaveRequest, DocumentSaveResult } from '@typie/lib/desktop';

const bridge = vi.hoisted(() => ({
  save: undefined as ((request: DocumentSaveRequest) => Promise<DocumentSaveResult>) | undefined,
  cleanupPush: vi.fn<() => Promise<void>>(),
}));
vi.mock('$lib/push', () => ({ cleanupBrowserPushForLogout: bridge.cleanupPush }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/desktop', () => ({
  desktop: {
    on: () => vi.fn(),
    onDocumentSave: (save: (request: DocumentSaveRequest) => Promise<DocumentSaveResult>) => {
      bridge.save = save;
      return () => {
        bridge.save = undefined;
      };
    },
  },
}));

let release: (() => void) | undefined;
let unregisterNavigation: (() => void) | undefined;

beforeEach(() => {
  unregisterNavigation = registerNavigationInterceptor(({ reason, paneIds }) =>
    documentEditing.prepareDeparture(
      () => (paneIds ? documentEditing.sessions.filter((session) => paneIds.includes(session.paneId)) : documentEditing.sessions),
      reason,
    ),
  );
  bridge.cleanupPush.mockResolvedValue(undefined);
  release = watchDesktopDocumentSave();
});

afterEach(() => {
  release?.();
  unregisterNavigation?.();
  documentEditing.nativePreparations = 0;
});

it('keeps native departure pending when one locally preserved pane has a server failure and another is still saving', async () => {
  const dispose: (() => void)[] = [];
  const write = Promise.withResolvers<undefined>();
  const request = { id: 'request', operationId: 'mixed-storage' };
  try {
    for (const id of ['local', 'pending']) {
      const editor = await Editor.createFromDoc(
        {
          root: {
            node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
            modifiers: {} as never,
            carry: [],
            children: [{ node: { type: 'paragraph' }, modifiers: {} as never, carry: [], children: [] }],
          },
        },
        { width: 320, height: 180, scale_factor: 1 },
      );
      const store = new IndexeddbDeltaStore();
      if (id === 'pending') vi.spyOn(store, 'put').mockImplementation(() => write.promise);
      const pusher = new Pusher({
        editor,
        store,
        documentId: crypto.randomUUID(),
        initialServerHeads: editor.currentHeads(),
        initialDurableHeads: editor.currentHeads(),
        pushFn: async () => {
          throw new Error('offline');
        },
      });
      const session = new DocumentEditingSession(id, id, () => id, editor, pusher);
      const off = documentEditing.register(session);
      dispose.push(() => {
        off();
        store.destroy();
        editor.destroy();
      });
      editor.updateNow((update) => {
        update.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
        update.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } });
      });
      if (id === 'local') {
        await expect.poll(() => session.isProtected()).toBe(true);
        await expect.poll(() => session.saveStatus).toBe('sync-failed');
      }
    }
    const prepared = await bridge.save?.({ ...request, phase: 'prepare' });
    expect(prepared?.status).toBe('pending');
    expect(prepared?.documents.map(({ id, status, protectedChanges }) => ({ id, status, protectedChanges }))).toEqual([
      { id: 'local', status: 'sync-failed', protectedChanges: true },
      { id: 'pending', status: 'pending', protectedChanges: false },
    ]);
  } finally {
    await bridge.save?.({ ...request, phase: 'release' });
    write.resolve(undefined);
    for (const release of dispose) release();
  }
});

it('includes late panes, rejects a released commit, and retains a concurrent sync reload after native cancellation', async () => {
  const editor = await Editor.createFromDoc(
    {
      root: {
        node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
        modifiers: {} as never,
        carry: [],
        children: [{ node: { type: 'paragraph' }, modifiers: {} as never, carry: [], children: [] }],
      },
    },
    { width: 320, height: 180, scale_factor: 1 },
  );
  const store = new IndexeddbDeltaStore();
  const write = Promise.withResolvers<undefined>();
  vi.spyOn(store, 'put').mockImplementation(() => write.promise);
  const pusher = new Pusher({
    editor,
    store,
    documentId: crypto.randomUUID(),
    initialServerHeads: editor.currentHeads(),
    initialDurableHeads: editor.currentHeads(),
    pushFn: async () => {
      throw new Error('offline');
    },
  });
  const entity = new Proxy({ id: 'entity', icon: 'book-open', iconColor: 'blue', node: new Proxy({ id: 'document' }, {}) }, {});
  const session = new DocumentEditingSession(
    'document',
    'pane',
    () => '저장 중인 문서',
    editor,
    pusher,
    () => entity,
  );
  let off: (() => void) | undefined;
  const request = { id: 'request', operationId: 'native-departure' };
  try {
    const initial = await bridge.save?.({ ...request, phase: 'prepare' });
    expect(initial?.status).toBe('protected');
    // A pane can finish loading after native preparation captured the session list.
    off = documentEditing.register(session);
    const replaced = await bridge.save?.({ ...request, phase: 'commit', discard: true });
    expect(replaced?.status).toBe('unknown');
    expect(replaced?.generation).not.toBe(initial?.generation);
    const checked = await bridge.save?.({ ...request, phase: 'check' });
    expect(checked?.generation).toBe(replaced?.generation);
    expect(checked?.status).toBe('protected');
    await bridge.save?.({ ...request, phase: 'release' });
    editor.updateNow((update) => {
      update.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
      update.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } });
    });
    const prepared = await bridge.save?.({ ...request, phase: 'prepare' });
    expect(prepared?.generation).toBe(replaced?.generation);
    expect(prepared?.status).toBe('pending');
    expect(prepared?.documents[0].id).toBe('document');
    expect(prepared?.documents[0].sessionId).toBe(session.id);
    expect(() => structuredClone(prepared)).not.toThrow();
    expect(prepared?.documents[0]).not.toHaveProperty('node');
    write.reject(new Error('storage unavailable'));
    await expect
      .poll(async () => {
        const checked = await bridge.save?.({ ...request, phase: 'check' });
        return checked?.status;
      })
      .toBe('failed');
    expect(session.isProtected()).toBe(false);
    const cleanup = Promise.withResolvers<undefined>();
    bridge.cleanupPush.mockReturnValue(cleanup.promise);
    const lateCommit = bridge.save?.({ ...request, phase: 'commit', reason: 'logout', discard: true });
    await bridge.save?.({ ...request, phase: 'release' });
    cleanup.resolve(undefined);
    const lateResult = await lateCommit;
    expect(lateResult?.status).toBe('unknown');
    expect(documentEditing.hasUnprotectedChanges()).toBe(true);
    expect(editor.editable).toBe(true);
    await bridge.save?.({ ...request, phase: 'prepare' });
    const commit = vi.fn(() => true);
    const reloading = runNavigation({ reason: 'reload', paneIds: [session].map((session) => session.paneId) }, commit);
    await expect.poll(() => documentEditing.operations[0]?.phase).toBe('blocked');
    await bridge.save?.({ ...request, phase: 'release' });
    expect(documentEditing.nativePreparations).toBe(0);
    expect(editor.editable).toBe(false);
    expect(commit).not.toHaveBeenCalled();
    documentEditing.cancel();
    expect(editor.editable).toBe(false);
    expect(commit).not.toHaveBeenCalled();
    documentEditing.discard();
    expect(await reloading).toBe(true);
    expect(commit).toHaveBeenCalledOnce();
    expect(editor.editable).toBe(true);
  } finally {
    await bridge.save?.({ ...request, phase: 'release' });
    off?.();
    pusher.stop();
    store.destroy();
    editor.destroy();
  }
});

it.each(['request release', 'bridge cleanup', 'session disposal'] as const)(
  'blocks edits in late panes through overlapping native departures and releases the stop on %s',
  async (ending) => {
    const editor = await Editor.createFromDoc(
      {
        root: {
          node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
          modifiers: {} as never,
          carry: [],
          children: [{ node: { type: 'paragraph' }, modifiers: {} as never, carry: [], children: [] }],
        },
      },
      { width: 320, height: 180, scale_factor: 1 },
    );
    const store = new IndexeddbDeltaStore();
    const pusher = new Pusher({
      editor,
      store,
      documentId: crypto.randomUUID(),
      initialServerHeads: editor.currentHeads(),
      initialDurableHeads: editor.currentHeads(),
      pushFn: async () => {
        throw new Error('offline');
      },
    });
    const session = new DocumentEditingSession('document', 'pane', () => '늦게 열린 문서', editor, pusher);
    const first = { id: 'first-request', operationId: 'first-departure' };
    const second = { id: 'second-request', operationId: 'second-departure' };
    let resume: (() => void) | undefined;
    try {
      for (const request of [first, second]) {
        const prepared = await bridge.save?.({ ...request, phase: 'prepare' });
        expect(prepared?.status).toBe('protected');
        const committed = await bridge.save?.({ ...request, phase: 'commit' });
        expect(committed?.status).toBe('protected');
      }
      documentEditing.register(session);
      const text = editor.proseText();
      editor.updateNow((update) => {
        update.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
        update.enqueue({ type: 'insertion', op: { type: 'text', text: 'must not enter' } });
      });
      expect(editor.proseText()).toBe(text);
      expect(editor.editable).toBe(false);
      // Stopping a newly loaded pane does not authorize it for departure.
      const changed = await bridge.save?.({ ...first, phase: 'commit' });
      expect(changed?.status).toBe('unknown');
      await bridge.save?.({ ...first, phase: 'release' });
      expect(editor.editable).toBe(false);

      resume = editor.localEdits.stop();
      if (ending === 'request release') await bridge.save?.({ ...second, phase: 'release' });
      else if (ending === 'bridge cleanup') release?.();
      else session.dispose();
      expect(editor.editable).toBe(false);
      resume();
      expect(editor.editable).toBe(true);
      if (!session.disposed) {
        editor.updateNow((update) => update.enqueue({ type: 'insertion', op: { type: 'text', text: 'resumed edit' } }));
        expect(editor.proseText()).toContain('resumed edit');
        await pusher.captureNow();
      }
    } finally {
      await bridge.save?.({ ...first, phase: 'release' });
      await bridge.save?.({ ...second, phase: 'release' });
      resume?.();
      session.dispose();
      store.destroy();
      editor.destroy();
    }
  },
);

it('keeps editing stopped while refreshing the pane set and rejects a commit for the previous set', async () => {
  const dispose: (() => void)[] = [];
  const create = async (id: string) => {
    const editor = await Editor.createFromDoc(
      {
        root: {
          node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
          modifiers: {} as never,
          carry: [],
          children: [{ node: { type: 'paragraph' }, modifiers: {} as never, carry: [], children: [] }],
        },
      },
      { width: 320, height: 180, scale_factor: 1 },
    );
    const store = new IndexeddbDeltaStore();
    const pusher = new Pusher({
      editor,
      store,
      documentId: crypto.randomUUID(),
      initialServerHeads: editor.currentHeads(),
      initialDurableHeads: editor.currentHeads(),
      pushFn: async () => {
        throw new Error('offline');
      },
    });
    const session = new DocumentEditingSession(id, id, () => id, editor, pusher);
    const off = documentEditing.register(session);
    dispose.push(() => {
      off();
      store.destroy();
      editor.destroy();
    });
    return { editor, session };
  };
  const request = { id: 'request', operationId: 'refresh-panes' };
  const cleanup = Promise.withResolvers<undefined>();
  let offObserver: (() => void) | undefined;
  try {
    const first = await create('first');
    first.editor.updateNow((update) => update.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
    let composing = true;
    dispose.push(
      first.editor.localEdits.registerInput({
        pending: () => composing,
        finalize: () => {
          if (!composing) return;
          composing = false;
          first.editor.enqueue({ type: 'insertion', op: { type: 'text', text: '한' } });
        },
      }),
    );
    const initial = await bridge.save?.({ ...request, phase: 'prepare' });
    expect(initial?.status).toBe('protected');
    expect(first.editor.proseText()).toBe('한');
    bridge.cleanupPush.mockReturnValue(cleanup.promise);
    const oldCommit = bridge.save?.({ ...request, phase: 'commit', reason: 'logout' });
    const second = await create('second');
    let resumed = false;
    // Save-state subscribers must not observe an editable gap when the checked
    // pane list changes inside the same native departure.
    offObserver = first.session.onChange(() => {
      if (resumed || !first.editor.editable) return;
      resumed = true;
      first.editor.updateNow((update) => {
        update.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
        update.enqueue({ type: 'insertion', op: { type: 'text', text: 'must not enter' } });
      });
    });
    const checked = await bridge.save?.({ ...request, phase: 'check' });
    expect(checked?.status).toBe('protected');
    expect(checked?.generation).not.toBe(initial?.generation);
    expect(first.editor.proseText()).toBe('한');
    expect(first.editor.editable).toBe(false);
    expect(second.editor.editable).toBe(false);
    cleanup.resolve(undefined);
    const oldResult = await oldCommit;
    expect(oldResult?.status).toBe('unknown');
    const committed = await bridge.save?.({ ...request, phase: 'commit' });
    expect(committed?.status).toBe('protected');
    offObserver();
    await bridge.save?.({ ...request, phase: 'release' });
    expect(first.editor.editable).toBe(true);
    expect(second.editor.editable).toBe(true);
  } finally {
    offObserver?.();
    cleanup.resolve(undefined);
    for (const release of dispose.toReversed()) release();
    await bridge.save?.({ ...request, phase: 'release' });
  }
});
