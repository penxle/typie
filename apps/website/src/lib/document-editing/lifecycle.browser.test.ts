import '../../app.css';

import { Toast } from '@typie/ui/notification';
import { mount, tick, unmount } from 'svelte';
import { toast } from 'svelte-sonner';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, userEvent } from 'vitest/browser';
import { beforeNavigate, goto as svelteGoto } from '$app/navigation';
import { Editor } from '$lib/editor-ffi/editor.svelte';
import EditorFrameSyncTestHost from '$lib/editor-ffi/editor-frame-sync-test-host.svelte';
import { setRootLayoutMode, setRootModifier } from '$lib/editor-ffi/root-attrs';
import { goto, guardNavigation, registerNavigationInterceptor, runNavigation } from '$lib/navigation';
import { IndexeddbDeltaStore } from '../../routes/website/(dashboard)/[slug]/v2/sync/store';
import { guardBrowserUnload } from './browser';
import DocumentSaveTestHost from './document-save-test-host.svelte';
import PaneTestHost from './pane-test-host.svelte';
import { DocumentEditingSession } from './session';
import { documentEditing } from './state.svelte';
import type { PlainDoc } from '@typie/editor-ffi/browser';
import type { SyncConnection } from '$lib/sync/connection';
import type { PaneGroup } from '../../routes/website/(dashboard)/[slug]/@pane/types';
import type { PusherOpts } from '../../routes/website/(dashboard)/[slug]/v2/sync/types';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$app/navigation')>()),
  beforeNavigate: vi.fn(),
  goto: vi.fn(),
}));

declare module 'vitest/browser' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions -- Vitest command augmentation requires interface merging
  interface BrowserCommands {
    dismissDocumentReload: () => Promise<string | undefined>;
  }
}

const doc: PlainDoc = {
  root: {
    node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
    modifiers: {} as never,
    carry: [],
    children: [{ node: { type: 'paragraph' }, modifiers: {} as never, carry: [], children: [] }],
  },
};

describe('document lifecycle in the production browser owners', () => {
  const cleanup: (() => void)[] = [];
  beforeEach(() => {
    cleanup.push(
      registerNavigationInterceptor(({ reason, paneIds }) =>
        documentEditing.prepareDeparture(
          () => (paneIds ? documentEditing.sessions.filter((session) => paneIds.includes(session.paneId)) : documentEditing.sessions),
          reason,
        ),
      ),
    );
  });
  let mounted: Record<string, unknown> | undefined;
  let target: HTMLElement | undefined;
  afterEach(async () => {
    toast.dismiss();
    documentEditing.cancel();
    for (const dispose of cleanup.splice(0).toReversed()) dispose();
    if (mounted) await unmount(mounted);
    mounted = undefined;
    target?.remove();
    vi.restoreAllMocks();
    vi.useRealTimers();
    vi.mocked(svelteGoto).mockReset();
    vi.mocked(beforeNavigate).mockClear();
  });
  const create = async (title: string, paneId: string = crypto.randomUUID(), documentId: string = crypto.randomUUID()) => {
    const editor = await Editor.createFromDoc(doc, { width: 320, height: 180, scale_factor: 1 });
    const store = new IndexeddbDeltaStore();
    const push = vi.fn<PusherOpts['pushFn']>(async () => {
      throw new Error('offline');
    });
    const pull = vi.fn<SyncConnection['pull']>(async () => {
      throw new Error('offline');
    });
    const onReload = vi.fn();
    const onError = vi.fn();
    const session = DocumentEditingSession.create({
      editor,
      store,
      capturedHeads: editor.currentHeads(),
      documentId,
      paneId,
      title: () => title,
      entity: undefined,
      snapshot: { heads: editor.currentHeads(), durableHeads: editor.currentHeads(), seq: '' },
      connection: { push: (_id, changesets) => push(changesets), pull },
      onReload,
      onError,
    });
    const off = documentEditing.register(session);
    cleanup.push(off);
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
    return { editor, store, pusher: session.pusher, session, push, pull, onReload, onError };
  };
  const departure = () => {
    const event = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(event);
    return event.defaultPrevented;
  };

  it('captures immediately after installation and ignores a pull completing after the session is disposed', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const { editor, store, session, pull, onReload } = await create('동기화 수명');
    const result = Promise.withResolvers<Awaited<ReturnType<SyncConnection['pull']>>>();
    pull.mockReturnValue(result.promise);
    const heads = editor.currentHeads();
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    await expect.poll(() => session.isProtected()).toBe(true);
    const records = await store.load(session.documentId);
    expect(records.length).toBeGreaterThan(0);

    await vi.advanceTimersByTimeAsync(10_000);
    expect(pull).toHaveBeenCalled();
    session.dispose();
    expect(documentEditing.sessions).not.toContain(session);
    result.resolve({ changesets: [], seq: 'late', heads, durableHeads: heads, needsReload: true });
    await result.promise;
    await tick();
    expect(onReload).not.toHaveBeenCalled();
    expect(editor.destroyed).toBe(true);
  });

  it('preserves programmatic navigation options and stops editing until that navigation completes', async () => {
    const { editor, store, session } = await create('이동 중인 문서');
    const write = Promise.withResolvers<undefined>();
    const put = store.put.bind(store);
    vi.spyOn(store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    guardNavigation();
    const before = vi.mocked(beforeNavigate).mock.calls.at(-1)?.[0];
    if (!before) throw new Error('Navigation guard missing');
    const complete = Promise.withResolvers<undefined>();
    const cancel = vi.fn();
    vi.mocked(svelteGoto).mockImplementation(async () => {
      before({
        from: null,
        to: { url: new URL('/next', location.href), params: { slug: 'next' }, route: { id: '/website/(dashboard)/[slug]' }, scroll: null },
        type: 'goto',
        willUnload: false,
        complete: complete.promise,
        cancel,
      });
      await complete.promise;
    });
    const options = { state: { entityTreeReveal: 'preserve' as const }, replaceState: true, noScroll: true, keepFocus: true };
    const navigation = goto('/next', options);
    expect(editor.editable).toBe(false);
    expect(svelteGoto).not.toHaveBeenCalled();
    write.resolve(undefined);
    await expect.poll(() => vi.mocked(svelteGoto).mock.calls).toEqual([['/next', options]]);
    expect(session.isProtected()).toBe(true);
    expect(editor.editable).toBe(false);
    expect(cancel).not.toHaveBeenCalled();
    complete.resolve(undefined);
    await navigation;
    expect(editor.editable).toBe(true);
  });

  it('does not start programmatic navigation when the user cancels a failed checkpoint', async () => {
    const { editor, store } = await create('이동 취소');
    vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    const navigation = goto('/next');
    await expect.poll(() => documentEditing.operations[0]?.phase).toBe('blocked');
    documentEditing.cancel();
    await navigation;
    expect(svelteGoto).not.toHaveBeenCalled();
    expect(editor.editable).toBe(true);
    expect(editor.proseText()).toContain('retained');
  });

  it('checks a pane loaded during navigation before leaving and retains its edits when cancelled', async () => {
    const first = await create('기존 문서');
    const write = Promise.withResolvers<undefined>();
    const put = first.store.put.bind(first.store);
    vi.spyOn(first.store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    first.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'first edit' } }));
    const navigation = goto('/next');
    expect(first.editor.editable).toBe(false);

    const late = await create('늦게 열린 문서');
    vi.spyOn(late.store, 'put').mockRejectedValue(new Error('storage unavailable'));
    late.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'late edit' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    write.resolve(undefined);

    await expect.poll(() => document.querySelector('[role="dialog"]')?.textContent).toContain('늦게 열린 문서');
    expect(first.session.isProtected()).toBe(true);
    expect(first.editor.editable).toBe(false);
    expect(late.editor.editable).toBe(false);
    expect(svelteGoto).not.toHaveBeenCalled();
    const cancel = [...document.querySelectorAll('button')].find((button) => button.textContent?.includes('계속 편집'));
    if (!cancel) throw new Error('Cancel button missing');
    cancel.click();
    await navigation;
    expect(first.editor.editable).toBe(true);
    expect(late.editor.editable).toBe(true);
    expect(late.editor.proseText()).toContain('late edit');
    late.push.mockResolvedValue({ heads: late.editor.currentHeads(), durableHeads: late.editor.currentHeads() });
    await late.pusher.pushNow();
    expect(late.session.isProtected()).toBe(true);
    expect(svelteGoto).not.toHaveBeenCalled();
  });

  it('replays back navigation only after history restoration and keeps editing stopped until it completes', async () => {
    const { editor } = await create('뒤로 이동');
    guardNavigation();
    const before = vi.mocked(beforeNavigate).mock.calls.at(-1)?.[0];
    if (!before) throw new Error('Navigation guard missing');
    const complete = Promise.withResolvers<undefined>();
    const destination = { url: new URL('/previous', location.href), params: {}, route: { id: null }, scroll: null };
    const cancel = vi.fn();
    const replayCancel = vi.fn();
    const go = vi.spyOn(history, 'go').mockImplementation(() => {
      before({
        from: null,
        to: destination,
        type: 'popstate',
        event: new PopStateEvent('popstate'),
        delta: -1,
        willUnload: false,
        complete: complete.promise,
        cancel: replayCancel,
      });
    });
    before({
      from: { ...destination, url: new URL(location.href) },
      to: destination,
      type: 'popstate',
      event: new PopStateEvent('popstate'),
      delta: -1,
      willUnload: false,
      complete: Promise.resolve(),
      cancel,
    });
    expect(cancel).toHaveBeenCalledOnce();
    await tick();
    expect(go).not.toHaveBeenCalled();
    window.dispatchEvent(new PopStateEvent('popstate'));
    await expect.poll(() => go.mock.calls).toEqual([[-1]]);
    expect(replayCancel).not.toHaveBeenCalled();
    expect(svelteGoto).not.toHaveBeenCalled();
    expect(editor.editable).toBe(false);
    const repeatedCancel = vi.fn();
    before({
      from: null,
      to: destination,
      type: 'popstate',
      event: new PopStateEvent('popstate'),
      delta: -1,
      willUnload: false,
      complete: Promise.resolve(),
      cancel: repeatedCancel,
    });
    expect(repeatedCancel).toHaveBeenCalledOnce();
    complete.resolve(undefined);
    await expect.poll(() => editor.editable).toBe(true);
  });

  it('protects edits in a pane loaded during history restoration before replaying back navigation', async () => {
    const first = await create('기존 문서');
    guardNavigation();
    const before = vi.mocked(beforeNavigate).mock.calls.at(-1)?.[0];
    if (!before) throw new Error('Navigation guard missing');
    const complete = Promise.withResolvers<undefined>();
    const destination = { url: new URL('/previous', location.href), params: {}, route: { id: null }, scroll: null };
    const go = vi.spyOn(history, 'go').mockImplementation(() => {
      before({
        from: null,
        to: destination,
        type: 'popstate',
        event: new PopStateEvent('popstate'),
        delta: -1,
        willUnload: false,
        complete: complete.promise,
        cancel: vi.fn(),
      });
    });
    before({
      from: { ...destination, url: new URL(location.href) },
      to: destination,
      type: 'popstate',
      event: new PopStateEvent('popstate'),
      delta: -1,
      willUnload: false,
      complete: Promise.resolve(),
      cancel: vi.fn(),
    });
    await tick();
    const late = await create('늦게 열린 문서');
    vi.spyOn(late.store, 'put').mockRejectedValue(new Error('storage unavailable'));
    late.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'late edit' } }));
    expect(late.session.isProtected()).toBe(false);
    window.dispatchEvent(new PopStateEvent('popstate'));
    try {
      await expect
        .poll(() => go.mock.calls.length > 0 || documentEditing.operations.some((operation) => operation.phase === 'blocked'))
        .toBe(true);
      expect(go).not.toHaveBeenCalled();
      expect(first.editor.editable).toBe(false);
      expect(late.editor.editable).toBe(false);
      documentEditing.cancel();
      await expect.poll(() => late.editor.editable).toBe(true);
      expect(first.editor.editable).toBe(true);
      expect(late.editor.proseText()).toContain('late edit');
      expect(go).not.toHaveBeenCalled();
    } finally {
      complete.resolve(undefined);
    }
  });

  it('warns synchronously for a queued edit and clears only after an IndexedDB transaction completes', async () => {
    const { editor, store, pusher } = await create('본문');
    const write = Promise.withResolvers<boolean>();
    const put = store.put.bind(store);
    vi.spyOn(store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    cleanup.push(guardBrowserUnload());
    editor.enqueue({ type: 'insertion', op: { type: 'text', text: 'captured locally' } });
    expect(departure()).toBe(true);
    editor.settlePendingEdits();
    expect(departure()).toBe(true);
    write.resolve(true);
    await pusher.captureNow();
    expect(departure()).toBe(false);
    expect(editor.proseText()).toContain('captured locally');
  });

  it('shows one grouped dialog and keeps the successful sibling when the user cancels', async () => {
    const first = await create('보존된 문서');
    const second = await create('실패한 문서');
    vi.spyOn(second.store, 'put').mockRejectedValue(new Error('quota exceeded'));
    first.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'safe' } }));
    await first.pusher.captureNow();
    second.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const commit = vi.fn();
    const leaving = runNavigation({ reason: 'leave', paneIds: [first.session, second.session].map((session) => session.paneId) }, commit);
    await expect.poll(() => documentEditing.operations[0]?.phase).toBe('blocked');
    await tick();
    expect(document.querySelectorAll('[data-focus-trap]')).toHaveLength(1);
    expect(document.body.textContent).toContain('실패한 문서');
    expect(document.body.textContent).not.toContain('보존된 문서');
    expect(first.editor.editable).toBe(false);
    expect(second.editor.editable).toBe(false);
    const cancel = [...document.querySelectorAll('button')].find((button) => button.textContent?.includes('계속 편집'));
    if (!cancel) throw new Error('Cancel button missing');
    cancel.click();
    expect(await leaving).toBe(false);
    expect(commit).not.toHaveBeenCalled();
    expect(first.editor.editable).toBe(true);
    expect(second.editor.editable).toBe(true);
    expect(second.editor.proseText()).toContain('retained');
  });

  it('updates header feedback directly from session failure and recovery without a copied UI state', async () => {
    const { editor, store, pusher, session, push } = await create('세션 저장 상태');
    const write = Promise.withResolvers<undefined>();
    const server = Promise.withResolvers<Awaited<ReturnType<PusherOpts['pushFn']>>>();
    vi.spyOn(store, 'put').mockImplementation(() => write.promise);
    push.mockImplementation(() => server.promise);
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target, props: { session } });
    await tick();
    expect(target.querySelector('[role="status"]')).toBeNull();
    session.markSaveFailure();
    await expect.poll(() => target?.querySelector('[aria-label="최근 변경사항 저장 상태 확인"]'), { timeout: 1000 }).not.toBeNull();
    server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
    await pusher.pushNow();
    await expect.poll(() => target?.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
    write.resolve(undefined);
  });

  it('shows local preservation after five seconds of server delay, then confirms the server save', async () => {
    const { editor, session, pusher, push } = await create('서버 저장 지연');
    const server = Promise.withResolvers<Awaited<ReturnType<PusherOpts['pushFn']>>>();
    push.mockReturnValue(server.promise);
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target, props: { session } });
    await tick();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
    try {
      editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'saved locally' } }));
      await pusher.captureNow();
      await tick();
      expect(session.isProtected()).toBe(true);
      await vi.advanceTimersByTimeAsync(4999);
      expect(target.querySelector('[role="status"]')).toBeNull();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(target.querySelector('[aria-label^="이 기기에는 안전하게 저장되었지만,"]')).not.toBeNull();
      expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
      await vi.advanceTimersByTimeAsync(4000);
      server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
      await pusher.pushNow();
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      await vi.advanceTimersByTimeAsync(2000);
      vi.useRealTimers();
      await expect.poll(() => target?.querySelector('[role="status"]')).toBeNull();
    } finally {
      vi.useRealTimers();
      server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
      await pusher.captureNow();
    }
  });

  it('recognizes restored local changes before rewriting them and protects later edits separately', async () => {
    const original = await create('복원할 문서');
    const serverHeads = original.editor.currentHeads();
    const serverGraph = original.editor.missingChangesetsFor(new Uint8Array()).bytes;
    original.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'restored' } }));
    await original.pusher.captureNow();
    original.session.dispose();
    await tick();

    const store = new IndexeddbDeltaStore();
    const records = await store.load(original.session.documentId);
    const editor = await Editor.createWithPending(
      serverGraph,
      records.map((record) => record.changeset),
      {
        width: 320,
        height: 180,
        scale_factor: 1,
      },
    );
    const restoredHeads = editor.currentHeads();
    const server = Promise.withResolvers<Awaited<ReturnType<PusherOpts['pushFn']>>>();
    const push = vi.fn<PusherOpts['pushFn']>(() => server.promise);
    const write = Promise.withResolvers<undefined>();
    const put = store.put.bind(store);
    vi.spyOn(store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    const session = DocumentEditingSession.create({
      editor,
      store,
      capturedHeads: restoredHeads,
      documentId: original.session.documentId,
      paneId: original.session.paneId,
      title: () => '복원된 문서',
      entity: undefined,
      snapshot: { heads: serverHeads, durableHeads: serverHeads, seq: '' },
      connection: { push: (_id, changesets) => push(changesets), pull: original.pull },
      onReload: original.onReload,
      onError: original.onError,
    });
    cleanup.push(documentEditing.register(session));
    try {
      expect(editor.proseText()).toContain('restored');
      expect(session.isProtected()).toBe(true);
      expect(session.isSynced()).toBe(false);
      expect(session.unprotectedSince).toBeNull();
      target = document.createElement('div');
      document.body.append(target);
      mounted = mount(DocumentSaveTestHost, { target, props: { session } });
      await tick();
      expect(target.querySelector('[aria-label^="이 기기에는 안전하게 저장되었지만,"]')).not.toBeNull();
      // Restoring local protection must not acknowledge these changes on the server.
      // The initial push sends them even when the user has not edited anything.
      await expect.poll(() => push.mock.calls[0]?.[0].length ?? 0).toBeGreaterThan(0);
      server.resolve({ heads: restoredHeads, durableHeads: serverHeads });
      await expect.poll(() => session.isSynced()).toBe(true);

      editor.updateNow((request) => {
        request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
        request.enqueue({ type: 'insertion', op: { type: 'text', text: 'new' } });
      });
      await tick();
      expect(session.isProtected()).toBe(false);
      expect(session.unprotectedSince).not.toBeNull();
      expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
      write.resolve(undefined);
      await session.pusher.captureNow();
      expect(session.isProtected()).toBe(true);
      expect(session.isSynced()).toBe(false);
    } finally {
      write.resolve(undefined);
      server.resolve({ heads: restoredHeads, durableHeads: serverHeads });
      await session.pusher.captureNow();
    }
  });

  it('keeps local-save feedback for five seconds after a later edit in a real editor session', async () => {
    const { editor, store, pusher, session } = await create('연속 입력 저장 상태');
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'saved' } }));
    await pusher.captureNow();
    await pusher.pushNow().catch(() => null);
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target, props: { session } });
    await tick();
    expect(target.querySelector('[aria-label="서버 저장 상태 확인"]')).not.toBeNull();

    const write = Promise.withResolvers<undefined>();
    const put = store.put.bind(store);
    vi.spyOn(store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
    vi.setSystemTime(Date.now() + 10_000);
    try {
      editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'new' } }));
      await tick();
      await vi.advanceTimersByTimeAsync(0);
      expect(target.querySelector('[aria-label="서버 저장 상태 확인"]')).not.toBeNull();
      expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
      await vi.advanceTimersByTimeAsync(4999);
      expect(target.querySelector('[aria-label="저장 시도 중"]')).toBeNull();
      await vi.advanceTimersByTimeAsync(1);
      await tick();
      expect(target.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();
    } finally {
      write.resolve(undefined);
      vi.useRealTimers();
      await pusher.captureNow();
    }
  });

  it('shows independent recovery for two panes editing the same document', async () => {
    const first = await create('같은 문서', 'first-pane', 'shared-document');
    const second = await create('같은 문서', 'second-pane', 'shared-document');
    for (const { editor, store } of [first, second]) {
      vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
      editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'unsaved' } }));
    }
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const leaving = runNavigation(
      { reason: 'leave', paneIds: [first.session, second.session].map((session) => session.paneId) },
      () => true,
    );
    const rows = () => [...document.querySelectorAll('[role="dialog"] li')];
    await expect.poll(() => rows().length).toBe(2);
    first.push.mockResolvedValue({ heads: first.editor.currentHeads(), durableHeads: first.editor.currentHeads() });
    await first.pusher.pushNow();
    await expect
      .poll(() => rows().map((row) => row.querySelector('[role="status"]')?.getAttribute('aria-label')))
      .toEqual(['서버에 저장했어요', '저장하지 못했어요']);
    expect(document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBeNull();
    documentEditing.cancel();
    expect(await leaving).toBe(false);
  });

  it('shows no progress modal and offers no retry for an optional departure failure', async () => {
    const { editor, store, pusher, session } = await create('지연된 문서');
    const loading = vi.spyOn(Toast, 'loading');
    const success = vi.spyOn(Toast, 'success');
    const write = Promise.withResolvers<undefined>();
    vi.spyOn(store, 'put').mockImplementation(() => write.promise);
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const commit = vi.fn();
    const leaving = runNavigation({ reason: 'leave' }, commit);
    await expect.poll(() => documentEditing.operations[0]?.showProgress).toBe(true);
    await tick();
    expect(document.querySelectorAll('[data-focus-trap]')).toHaveLength(0);
    expect(loading).toHaveBeenCalled();
    await expect.poll(() => documentEditing.operations[0]?.phase, { timeout: 5000 }).toBe('blocked');
    expect(pusher.captureFailures).toBe(0);
    expect(session.saveFailed).toBe(false);
    await tick();
    expect(document.querySelectorAll('[data-focus-trap]')).toHaveLength(1);
    expect([...document.querySelectorAll('button')].some((button) => button.textContent?.includes('다시 시도'))).toBe(false);
    documentEditing.cancel();
    expect(await leaving).toBe(false);
    write.resolve(undefined);
    await pusher.captureNow();
    expect(commit).not.toHaveBeenCalled();
    expect(success).not.toHaveBeenCalled();
  });

  it('opens save details without retrying until the user asks, then confirms recovery', async () => {
    const { editor, store, pusher, session } = await create('저장 상태');
    const success = vi.spyOn(Toast, 'success');
    const write = Promise.withResolvers<undefined>();
    vi.spyOn(store, 'put').mockImplementation(() => write.promise);
    const checkpoint = vi.spyOn(pusher, 'checkpoint');
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const saving = documentEditing.showSaveStatus([session]);
    await tick();
    expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    expect(checkpoint).not.toHaveBeenCalled();
    const buttons = [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')];
    const retry = buttons.find((button) => button.textContent?.includes('다시 시도'));
    if (!retry) throw new Error('Retry button missing');
    retry.click();
    expect(checkpoint).toHaveBeenCalledOnce();
    write.resolve(undefined);
    await expect.poll(() => document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBe('5');
    expect(await saving).toBe(true);
    await expect.poll(() => document.querySelectorAll('[role="dialog"]')).toHaveLength(0);
    expect(success).not.toHaveBeenCalled();
  });

  it('distinguishes an immediate retry failure from a later slow retry', async () => {
    const { editor, store, pusher, session } = await create('저장 실패');
    const put = vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
    const loading = vi.spyOn(Toast, 'loading');
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const saving = documentEditing.showSaveStatus([session]);
    await tick();
    const retry = [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((button) =>
      button.textContent?.includes('다시 시도'),
    );
    if (!retry) throw new Error('Retry button missing');
    retry.click();
    expect(documentEditing.operations[0]?.phase).toBe('saving');
    await expect.poll(() => documentEditing.operations[0]?.phase, { timeout: 2000 }).toBe('blocked');
    await tick();
    expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    expect(loading).not.toHaveBeenCalled();
    expect(session.inspectedSaveStatus).toBe('failed');
    const write = Promise.withResolvers<undefined>();
    put.mockImplementation(() => write.promise);
    documentEditing.retry();
    await expect.poll(() => documentEditing.operations[0]?.phase, { timeout: 5000 }).toBe('blocked');
    await tick();
    expect(session.inspectedSaveStatus).toBe('pending');
    documentEditing.cancel();
    expect(await saving).toBe(false);
    write.resolve(undefined);
    await pusher.captureNow();
  });

  it('explains local protection without blocking departure and retries only the server save', async () => {
    const { editor, store, pusher, session, push } = await create('로컬에 저장된 문서');
    const success = vi.spyOn(Toast, 'success');
    vi.spyOn(store, 'put').mockResolvedValue(undefined);
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'local' } }));
    await pusher.captureNow();
    const pushNow = vi.spyOn(pusher, 'pushNow');
    cleanup.push(guardBrowserUnload());
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const syncing = documentEditing.showSaveStatus([session], true);
    await tick();
    expect(pushNow).not.toHaveBeenCalled();
    expect(departure()).toBe(false);
    const retry = [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((button) =>
      button.textContent?.includes('다시 시도'),
    );
    if (!retry) throw new Error('Retry button missing');
    retry.click();
    await expect.poll(() => documentEditing.operations[0]?.phase, { timeout: 2000 }).toBe('blocked');
    expect(pusher.pushFailed).toBe(true);
    expect(departure()).toBe(false);
    push.mockResolvedValue({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
    await tick();
    const retryAgain = [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((button) =>
      button.textContent?.includes('다시 시도'),
    );
    if (!retryAgain) throw new Error('Retry button missing');
    retryAgain.click();
    await expect.poll(() => document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBe('5');
    expect(await syncing).toBe(true);
    expect(pusher.isSynced()).toBe(true);
    await expect.poll(() => document.querySelectorAll('[role="dialog"]')).toHaveLength(0);
    expect(success).not.toHaveBeenCalled();
  });

  it('updates storage feedback during active composition without releasing departure protection', async () => {
    const { editor, store, pusher, session, push } = await create('조합 중인 문서');
    const write = Promise.withResolvers<undefined>();
    const put = store.put.bind(store);
    vi.spyOn(store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    const server = Promise.withResolvers<Awaited<ReturnType<PusherOpts['pushFn']>>>();
    push.mockReturnValue(server.promise);
    cleanup.push(guardBrowserUnload());
    const inputTarget = document.createElement('div');
    document.body.append(inputTarget);
    const inputHost = mount(EditorFrameSyncTestHost, { target: inputTarget, props: { editor, userId: crypto.randomUUID() } });
    try {
      await tick();
      editor.inputEl?.focus();
      await tick();
      const context = editor.inputEl?.editContext;
      if (!context) throw new Error('Production EditContext input is not mounted');
      target = document.createElement('div');
      document.body.append(target);
      mounted = mount(DocumentSaveTestHost, { target, props: { session } });
      await tick();
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
      context.dispatchEvent(new CompositionEvent('compositionstart'));
      context.updateText(1, 1, '한');
      context.updateSelection(2, 2);
      context.dispatchEvent(
        new TextUpdateEvent('textupdate', {
          updateRangeStart: 1,
          updateRangeEnd: 1,
          text: '한',
          selectionStart: 2,
          selectionEnd: 2,
        }),
      );
      await expect.poll(() => editor.proseText()).toBe('한');
      expect(editor.localEdits.pending).toBe(true);
      await vi.advanceTimersByTimeAsync(5000);
      await tick();
      expect(target.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();

      write.resolve(undefined);
      await pusher.captureNow();
      await tick();
      expect(editor.ime(64, 64)?.composing).toEqual({ start: 1, end: 2 });
      expect(pusher.isProtected()).toBe(true);
      expect(target.querySelector('[aria-label^="이 기기에는 안전하게 저장되었지만,"]')).not.toBeNull();
      expect(departure()).toBe(true);

      server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
      await pusher.pushNow();
      await tick();
      expect(target.querySelector('[aria-label="서버에 저장했어요"]')).not.toBeNull();
      expect(editor.localEdits.pending).toBe(true);
      expect(departure()).toBe(true);
      await vi.advanceTimersByTimeAsync(2000);
      vi.useRealTimers();
      await expect.poll(() => target?.querySelector('[role="status"]')).toBeNull();
      editor.finalizeInput();
      expect(departure()).toBe(false);
    } finally {
      vi.useRealTimers();
      write.resolve(undefined);
      server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
      await pusher.captureNow();
      await unmount(inputHost);
      inputTarget.remove();
    }
  });

  it('shows pending storage while local capture fails but the server acknowledgement is still in flight', async () => {
    const { editor, store, pusher, session, push } = await create('서버 보존 대기');
    vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
    const server = Promise.withResolvers<Awaited<ReturnType<PusherOpts['pushFn']>>>();
    push.mockImplementation(() => server.promise);
    cleanup.push(guardBrowserUnload());
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'server copy' } }));
    const checkpoint = pusher.checkpoint();
    await expect.poll(() => pusher.captureFailures).toBeGreaterThan(0);
    expect(session.saveStatus).toBe('pending');
    expect(departure()).toBe(true);
    server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
    await checkpoint;
    expect(session.saveStatus).toBe('synced');
    expect(departure()).toBe(false);
  });

  it('does not claim local protection in server details until the newly finalized input is captured', async () => {
    const { editor, store, pusher, session } = await create('조합 확정 후 저장');
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'saved' } }));
    await pusher.captureNow();
    await expect(pusher.pushNow()).rejects.toThrow('offline');
    const write = Promise.withResolvers<undefined>();
    const put = store.put.bind(store);
    vi.spyOn(store, 'put').mockImplementation(async (record) => {
      await write.promise;
      await put(record);
    });
    let composing = true;
    cleanup.push(
      editor.localEdits.registerInput({
        pending: () => composing,
        finalize: () => {
          composing = false;
          editor.enqueue({ type: 'insertion', op: { type: 'text', text: '한글' } });
        },
      }),
    );
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const syncing = documentEditing.showSaveStatus([session], true);
    await tick();
    expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    expect(session.isProtected()).toBe(false);
    write.resolve(undefined);
    await pusher.captureNow();
    await tick();
    expect(session.isProtected()).toBe(true);
    expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    documentEditing.cancel();
    expect(await syncing).toBe(false);
  });

  it('keeps recovered documents and counts down only after every document is protected, without a duplicate toast', async () => {
    const success = vi.spyOn(Toast, 'success');
    const first = await create('첫 번째 저장');
    const second = await create('두 번째 저장');
    const firstWrite = Promise.withResolvers<undefined>();
    const secondWrite = Promise.withResolvers<undefined>();
    vi.spyOn(first.store, 'put').mockImplementation(() => firstWrite.promise);
    vi.spyOn(second.store, 'put').mockImplementation(() => secondWrite.promise);
    for (const { editor } of [first, second]) {
      editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    }
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const firstCommit = vi.fn(() => true);
    const secondCommit = vi.fn(() => true);
    const firstLeave = runNavigation({ reason: 'leave', paneIds: [first.session].map((session) => session.paneId) }, firstCommit);
    const secondLeave = runNavigation({ reason: 'leave', paneIds: [second.session].map((session) => session.paneId) }, secondCommit);
    await expect.poll(() => documentEditing.operations.every((operation) => operation.phase === 'blocked'), { timeout: 5000 }).toBe(true);
    await tick();
    expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    firstWrite.resolve(undefined);
    await expect.poll(() => first.session.isProtected()).toBe(true);
    await tick();
    expect(firstCommit).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain('첫 번째 저장');
    await expect
      .poll(
        () =>
          [...document.querySelectorAll('[role="dialog"] li')]
            .find((row) => row.textContent?.includes('첫 번째 저장'))
            ?.querySelector('[aria-label^="이 기기에는 안전하게 저장되었지만,"]') ?? null,
      )
      .not.toBeNull();
    expect(document.querySelector('[data-save-countdown]')).toBeNull();
    secondWrite.resolve(undefined);
    await expect.poll(() => document.querySelector('[data-save-countdown]')).not.toBeNull();
    expect(firstCommit).not.toHaveBeenCalled();
    expect(secondCommit).not.toHaveBeenCalled();
    expect(document.querySelector('[data-save-countdown]')?.textContent?.trim()).toBe('5');
    expect(document.body.textContent).not.toContain('저장하지 않고 나가기');
    await Promise.all([firstLeave, secondLeave]);
    await expect.poll(() => document.querySelectorAll('[role="dialog"]')).toHaveLength(0);
    expect(firstCommit).toHaveBeenCalledOnce();
    expect(secondCommit).toHaveBeenCalledOnce();
    expect(success).not.toHaveBeenCalled();
  });

  it.each(['protected', 'discard'] as const)('does not claim recovery for a %s departure', async (outcome) => {
    const { editor, store, session } = await create('종료할 문서');
    const success = vi.spyOn(Toast, 'success');
    if (outcome === 'discard') {
      vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
      editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    }
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const leaving = runNavigation({ reason: 'leave', paneIds: [session].map((session) => session.paneId) }, () => true);
    if (outcome === 'discard') {
      await expect.poll(() => document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
      const discard = [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((button) =>
        button.textContent?.includes('저장하지 않고 나가기'),
      );
      if (!discard) throw new Error('Discard button missing');
      discard.click();
    }
    expect(await leaving).toBe(true);
    expect(success).not.toHaveBeenCalled();
  });

  it.each(['cancel', 'replace', 'continue'] as const)('revalidates a completion countdown when the user chooses %s', async (choice) => {
    const success = vi.spyOn(Toast, 'success');
    const { editor, store, session } = await create('완료 대기 중인 문서');
    const write = Promise.withResolvers<undefined>();
    vi.spyOn(store, 'put').mockImplementation(() => write.promise);
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const commit = vi.fn(() => true);
    const leaving = runNavigation({ reason: 'leave', paneIds: [session].map((session) => session.paneId) }, commit);
    await expect.poll(() => document.querySelectorAll('[role="dialog"]'), { timeout: 5000 }).toHaveLength(1);
    write.resolve(undefined);
    await expect.poll(() => document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBe('5');
    expect(editor.editable).toBe(false);
    if (choice === 'cancel') documentEditing.cancel();
    else if (choice === 'replace') session.dispose();
    else documentEditing.completeRecovery();
    expect(await leaving).toBe(choice === 'continue');
    expect(commit.mock.calls).toHaveLength(choice === 'continue' ? 1 : 0);
    await expect.poll(() => document.querySelector('[data-save-countdown]')).toBeNull();
    expect(success).not.toHaveBeenCalled();
  });

  it('does not let another fast departure dismiss a visible completion countdown or show a duplicate toast', async () => {
    const success = vi.spyOn(Toast, 'success');
    const recovering = await create('완료 대기 중인 문서');
    const other = await create('이미 보존된 문서');
    vi.spyOn(recovering.store, 'put').mockRejectedValue(new Error('storage unavailable'));
    recovering.editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const commit = vi.fn(() => true);
    const leaving = runNavigation({ reason: 'leave', paneIds: [recovering.session].map((session) => session.paneId) }, commit);
    await expect.poll(() => document.querySelectorAll('[role="dialog"]')).toHaveLength(1);
    recovering.push.mockResolvedValue({ heads: recovering.editor.currentHeads(), durableHeads: recovering.editor.currentHeads() });
    await recovering.pusher.pushNow();
    await expect.poll(() => document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBe('5');

    expect(await runNavigation({ reason: 'reload', paneIds: [other.session].map((session) => session.paneId) }, () => true)).toBe(true);
    await tick();
    expect(commit).not.toHaveBeenCalled();
    expect(document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBe('5');
    expect(success).not.toHaveBeenCalled();
    documentEditing.cancel();
    expect(await leaving).toBe(false);
  });

  it('updates a protected document icon while another pane in the same operation is still saving', async () => {
    const first = await create('먼저 저장되는 문서');
    const second = await create('아직 저장 중인 문서');
    const firstWrite = Promise.withResolvers<undefined>();
    const secondWrite = Promise.withResolvers<undefined>();
    vi.spyOn(first.store, 'put').mockImplementation(() => firstWrite.promise);
    vi.spyOn(second.store, 'put').mockImplementation(() => secondWrite.promise);
    for (const { editor } of [first, second])
      editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const leaving = runNavigation(
      { reason: 'leave', paneIds: [first.session, second.session].map((session) => session.paneId) },
      () => true,
    );
    await expect.poll(() => document.querySelectorAll('[role="dialog"] li'), { timeout: 5000 }).toHaveLength(2);
    firstWrite.resolve(undefined);
    await expect.poll(() => document.querySelectorAll('[aria-label^="이 기기에는 안전하게 저장되었지만,"]')).toHaveLength(1);
    expect(document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBeNull();
    expect(document.querySelector('[aria-label="저장 시도 중"]')).not.toBeNull();
    documentEditing.cancel();
    secondWrite.resolve(undefined);
    expect(await leaving).toBe(false);
  });

  it('shows local preservation during the completion countdown and switches to server confirmation after acknowledgement', async () => {
    const { editor, store, session, push } = await create('로컬에 저장된 문서');
    const write = Promise.withResolvers<undefined>();
    const server = Promise.withResolvers<Awaited<ReturnType<PusherOpts['pushFn']>>>();
    vi.spyOn(store, 'put').mockImplementation(() => write.promise);
    push.mockImplementation(() => server.promise);
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'pending' } }));
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(DocumentSaveTestHost, { target });
    const commit = vi.fn(() => true);
    const leaving = runNavigation({ reason: 'leave', paneIds: [session].map((session) => session.paneId) }, commit);
    const spinner = () => document.querySelector('[role="dialog"] [aria-label="저장 시도 중"]');
    const locallySaved = () => document.querySelector('[role="dialog"] [aria-label^="이 기기에는 안전하게 저장되었지만,"]');
    const check = () => document.querySelector('[role="dialog"] [aria-label="서버에 저장했어요"]');
    await expect.poll(spinner, { timeout: 5000 }).not.toBeNull();
    write.resolve(undefined);
    await expect.poll(() => document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null).toBe('5');
    await expect.poll(locallySaved).not.toBeNull();
    expect(check()).toBeNull();
    await expect.poll(spinner).toBeNull();
    expect(session.isSynced()).toBe(false);
    expect(commit).not.toHaveBeenCalled();
    server.resolve({ heads: editor.currentHeads(), durableHeads: editor.currentHeads() });
    await expect.poll(check).not.toBeNull();
    await expect.poll(locallySaved).toBeNull();
    await expect.poll(() => document.querySelector('[data-save-countdown]')?.textContent?.trim() ?? null, { timeout: 4000 }).toBe('2');
    expect(check()).not.toBeNull();
    expect(commit).not.toHaveBeenCalled();
    expect(await leaving).toBe(true);
    expect(commit).toHaveBeenCalledOnce();
  });

  it('captures root formatting and layout through the same editing session', async () => {
    const { editor, pusher } = await create('본문 설정');
    cleanup.push(guardBrowserUnload());
    setRootLayoutMode(editor, { type: 'continuous', max_width: 480 });
    expect(departure()).toBe(true);
    editor.settlePendingEdits();
    await pusher.captureNow();
    expect(departure()).toBe(false);
    expect(editor.appliedSnapshot.rootAttrs?.layout_mode).toEqual({ type: 'continuous', max_width: 480 });
    const revision = editor.documentRevision;
    setRootModifier(editor, { type: 'font_size', value: 2200 });
    expect(departure()).toBe(true);
    editor.settlePendingEdits();
    expect(editor.documentRevision).toBeGreaterThan(revision);
    await pusher.captureNow();
    expect(departure()).toBe(false);
    expect(editor.appliedSnapshot.rootModifiers).toContainEqual({ type: 'font_size', value: 2200 });
  });

  it('opens a real native beforeunload prompt and keeps editing after staying', async () => {
    const { editor, store, pusher } = await create('브라우저 이탈');
    vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
    cleanup.push(guardBrowserUnload());
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'retained' } }));
    await expect.poll(() => pusher.captureFailures).toBeGreaterThan(0);
    target = document.createElement('button');
    target.textContent = 'Continue';
    document.body.append(target);
    await userEvent.click(target); // Trusted user activation is required by Chromium.
    expect(await commands.dismissDocumentReload()).toBe('beforeunload');
    expect(editor.editable).toBe(true);
    expect(editor.proseText()).toContain('retained');
    await userEvent.click(target);
    await expect.poll(() => documentEditing.operations[0]?.phase).toBe('blocked');
    expect(documentEditing.operations[0]?.reason).toBe('save');
    documentEditing.cancel();
    await expect.poll(() => editor.editable).toBe(true);
  });

  it('does not replace a pane or switch sites before the grouped checkpoint is approved', async () => {
    const ready = Promise.withResolvers<PaneGroup>();
    target = document.createElement('div');
    document.body.append(target);
    const siteId = crypto.randomUUID();
    mounted = mount(PaneTestHost, { target, props: { siteId, onReady: ready.resolve } });
    const group = await ready.promise;
    const original = group.state.current.root;
    const paneId = group.state.current.focusedPaneId;
    if (!paneId) throw new Error('Pane did not initialize');
    const { editor, store } = await create('unsaved', paneId);
    vi.spyOn(store, 'put').mockRejectedValue(new Error('storage unavailable'));
    editor.updateNow((request) => request.enqueue({ type: 'insertion', op: { type: 'text', text: 'kept' } }));
    const replace = group.replacePane(paneId, { kind: 'home' });
    await expect.poll(() => documentEditing.operations[0]?.phase).toBe('blocked');
    expect(group.state.current.root).toBe(original);
    documentEditing.cancel();
    expect(await replace).toBe(false);
    group.switchToSite(crypto.randomUUID());
    await expect.poll(() => documentEditing.operations[0]?.phase).toBe('blocked');
    expect(group.currentSiteId).toBe(siteId);
    documentEditing.cancel();
    await tick();
    expect(group.state.current.root).toBe(original);
    expect(editor.proseText()).toContain('kept');
  });
});
