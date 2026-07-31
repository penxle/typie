import { mount, tick, unmount } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteListState } from '$lib/note/note-list-state.svelte';
import Notes from './Notes.svelte';

const { app, createNote, createQuery, noteEdits, noteSync, terminalNoteIds, terminalListeners, toastError } = vi.hoisted(() => ({
  app: {
    preference: undefined as unknown as { readonly current: { currentSiteId: string | undefined } },
    state: { notesOpen: true },
  },
  createNote: vi.fn(),
  createQuery: vi.fn(),
  noteEdits: {
    get: vi.fn(),
    remove: vi.fn(),
    sync: vi.fn(),
  },
  noteSync: {
    isTerminallyDeleted: vi.fn((_siteId: string, noteId: string) => terminalNoteIds.has(noteId)),
    onTerminalDelete: vi.fn(({ listener }: { listener: (noteId: string) => void }) => {
      terminalListeners.add(listener);
      return () => terminalListeners.delete(listener);
    }),
    retainRelatedEntity: vi.fn(),
  },
  terminalNoteIds: new Set<string>(),
  terminalListeners: new Set<(noteId: string) => void>(),
  toastError: vi.fn(),
}));

let currentSite = new SvelteMap<string, string | undefined>();

vi.mock('@mearie/svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@mearie/svelte')>()),
  createQuery,
}));
vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('@typie/ui/context', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@typie/ui/context')>()),
  getAppContext: () => app,
}));
vi.mock('@typie/ui/notification', () => ({ Toast: { error: toastError } }));
vi.mock('mixpanel-browser', () => ({ default: { track: vi.fn() } }));
vi.mock('$app/navigation', () => ({ afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$lib/graphql', () => ({ cache: { invalidate: vi.fn() } }));
vi.mock('$lib/note/note-mutation', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/note/note-mutation')>()),
  getNoteOperationsContext: () => ({
    addEntity: vi.fn(),
    create: createNote,
    delete: vi.fn(),
    move: vi.fn(),
    removeEntity: vi.fn(),
    update: vi.fn(),
  }),
}));
vi.mock('$lib/note/note-edit-state.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/note/note-edit-state.svelte')>()),
  getNoteEditsContext: () => noteEdits,
}));
vi.mock('$lib/note/note-sync.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/note/note-sync.svelte')>()),
  getNoteSyncContext: () => noteSync,
}));

type SiteNote = {
  id: string;
  content: string;
  createdAt: string;
  updatedAt: string;
  order: string;
  color: string;
  status: 'OPEN' | 'RESOLVED';
  site: { id: string };
  entities: [];
};

type SiteQuery = {
  data?: { notes: SiteNote[] };
  loading: boolean;
  error?: Error;
  refetch: ReturnType<typeof vi.fn>;
};

const siteNote = (status: SiteNote['status']): SiteNote => ({
  id: 'note-1',
  content: 'note content',
  createdAt: '2026-07-29T00:00:00.000Z',
  updatedAt: status === 'OPEN' ? '2026-07-29T00:00:01.000Z' : '2026-07-29T00:00:00.000Z',
  order: '100',
  color: 'gray',
  status,
  site: { id: 'site-1' },
  entities: [],
});

const createSiteQueryController = (initial: SiteQuery) => {
  const state = new SvelteMap([['snapshot', initial]]);
  const evaluatedSiteIds: (string | null)[] = [];
  let getVariables: (() => { siteId: string | null }) | undefined;
  const read = () => {
    if (getVariables) evaluatedSiteIds.push(getVariables().siteId);
    return state.get('snapshot') ?? initial;
  };

  return {
    connect(nextGetVariables: () => { siteId: string | null }) {
      getVariables = nextGetVariables;
    },
    evaluatedSiteIds,
    publish(snapshot: SiteQuery) {
      state.set('snapshot', snapshot);
    },
    query: {
      get data() {
        return read().data;
      },
      get loading() {
        return read().loading;
      },
      get error() {
        return read().error;
      },
      refetch: initial.refetch,
    },
  };
};

const mountNotes = async (siteQuery: SiteQuery | ReturnType<typeof createSiteQueryController>) => {
  const controller = 'query' in siteQuery ? siteQuery : createSiteQueryController(siteQuery);
  createQuery
    .mockImplementationOnce((_document, getVariables) => {
      controller.connect(getVariables as () => { siteId: string | null });
      return controller.query;
    })
    .mockReturnValue({ data: undefined, loading: false, error: undefined, refetch: vi.fn() });
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(Notes, { target });
  await tick();
  return { component, controller, target };
};

describe('global notes error UX', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe = vi.fn();
        unobserve = vi.fn();
        disconnect = vi.fn();
      },
    );
    Object.defineProperty(Element.prototype, 'animate', {
      configurable: true,
      value: vi.fn(() => ({
        cancel: vi.fn(),
        currentTime: 0,
        effect: null,
        onfinish: null,
        playState: 'finished',
      })),
    });
    currentSite = new SvelteMap([['id', 'site-1']]);
    app.preference = {
      get current() {
        return { currentSiteId: currentSite.get('id') };
      },
    };
    app.state.notesOpen = true;
    createNote.mockReset();
    createQuery.mockReset();
    noteSync.isTerminallyDeleted.mockClear();
    noteSync.onTerminalDelete.mockClear();
    noteSync.retainRelatedEntity.mockClear();
    terminalNoteIds.clear();
    terminalListeners.clear();
    toastError.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    Reflect.deleteProperty(Element.prototype, 'animate');
    document.body.replaceChildren();
  });

  it('shows an inline retry instead of the empty state when the first query fails without cached data', async () => {
    const refetch = vi.fn();
    const { component } = await mountNotes({ loading: false, error: new Error('offline'), refetch });

    try {
      expect(document.body.textContent).toContain('노트를 불러오지 못했어요.');
      expect(document.body.textContent).not.toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');

      const retry = [...document.body.querySelectorAll('button')].find((button) => button.textContent?.trim() === '다시 시도');
      expect(retry).toBeDefined();
      retry?.click();
      expect(refetch).toHaveBeenCalledOnce();
    } finally {
      await unmount(component);
    }
  });

  it('shows a loading message only while the first query has no cached data', async () => {
    const { component } = await mountNotes({ loading: true, refetch: vi.fn() });

    try {
      expect(document.body.textContent).toContain('노트를 불러오는 중...');
      expect(document.body.textContent).not.toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');
    } finally {
      await unmount(component);
    }
  });

  it('keeps the successful empty state when a refresh fails with cached data', async () => {
    const controller = createSiteQueryController({ data: { notes: [] }, loading: false, refetch: vi.fn() });
    const { component } = await mountNotes(controller);

    try {
      controller.publish({ data: { notes: [] }, loading: false, error: new Error('offline'), refetch: vi.fn() });
      await tick();

      expect(document.body.textContent).toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');
      expect(document.body.textContent).not.toContain('노트를 불러오지 못했어요.');
    } finally {
      await unmount(component);
    }
  });

  it('removes the collapsed completed-note region from interaction and accessibility', async () => {
    const { component } = await mountNotes({ data: { notes: [] }, loading: false, refetch: vi.fn() });

    try {
      const region = document.body.querySelector<HTMLElement>('[aria-label="완료된 노트"]');
      expect(region).not.toBeNull();
      expect(region?.inert).toBe(true);
      expect(region?.getAttribute('aria-hidden')).toBe('true');
    } finally {
      await unmount(component);
    }
  });

  it('does not retain a hidden completed exit or let its completion collapse the reopened note', async () => {
    vi.useFakeTimers();
    const controller = createSiteQueryController({
      data: { notes: [siteNote('RESOLVED')] },
      loading: false,
      refetch: vi.fn(),
    });
    const { component } = await mountNotes(controller);

    try {
      controller.publish({
        data: { notes: [siteNote('OPEN')] },
        loading: false,
        refetch: vi.fn(),
      });
      await tick();

      const ownedCards = () => [...document.body.querySelectorAll<HTMLElement>('[data-note-list-owned-item][data-note-id="note-1"]')];
      expect(ownedCards()).toHaveLength(1);

      const openCard = ownedCards()[0]?.querySelector<HTMLElement>('[role="button"][data-note-id="note-1"]');
      openCard?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Enter' }));
      await tick();
      expect(openCard?.querySelector('textarea')).not.toBeNull();

      await vi.advanceTimersByTimeAsync(500);
      await tick();

      expect(ownedCards()).toHaveLength(1);
      expect(openCard?.querySelector('textarea')).not.toBeNull();
    } finally {
      await unmount(component);
    }
  });

  it('does not retain a terminally deleted card in the hidden completed list', async () => {
    const controller = createSiteQueryController({
      data: { notes: [siteNote('RESOLVED')] },
      loading: false,
      refetch: vi.fn(),
    });
    const { component } = await mountNotes(controller);

    try {
      terminalNoteIds.add('note-1');
      for (const listener of terminalListeners) listener('note-1');
      await tick();

      expect(document.body.querySelector('[data-note-list-owned-item][data-note-id="note-1"]')).toBeNull();
    } finally {
      await unmount(component);
    }
  });

  it('does not present retained data from the previous site as the new site cache', async () => {
    const controller = createSiteQueryController({ data: { notes: [] }, loading: false, refetch: vi.fn() });
    const { component } = await mountNotes(controller);

    try {
      expect(document.body.textContent).toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');

      currentSite.set('id', 'site-2');
      controller.publish({ data: { notes: [] }, loading: true, refetch: vi.fn() });
      await tick();

      expect(controller.evaluatedSiteIds).toContain('site-2');
      expect(document.body.textContent).toContain('노트를 불러오는 중...');
      expect(document.body.textContent).not.toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');

      controller.publish({ data: { notes: [] }, loading: false, error: new Error('offline'), refetch: vi.fn() });
      await tick();

      expect(document.body.textContent).toContain('노트를 불러오지 못했어요.');
      expect(document.body.textContent).not.toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');

      controller.publish({ data: { notes: [] }, loading: false, refetch: vi.fn() });
      await tick();
      expect(document.body.textContent).toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');

      controller.publish({ data: { notes: [] }, loading: false, error: new Error('offline'), refetch: vi.fn() });
      await tick();
      expect(document.body.textContent).toContain('떠오르는 생각이나 아이디어를 자유롭게 기록해보세요');
      expect(document.body.textContent).not.toContain('노트를 불러오지 못했어요.');
    } finally {
      await unmount(component);
    }
  });

  it('resets both list states when the current site becomes unavailable', async () => {
    const reset = vi.spyOn(NoteListState.prototype, 'reset');
    const { component } = await mountNotes({ data: { notes: [] }, loading: false, refetch: vi.fn() });

    try {
      reset.mockClear();
      currentSite.set('id', undefined);
      await tick();

      expect(reset).toHaveBeenCalledTimes(2);
    } finally {
      await unmount(component);
      reset.mockRestore();
    }
  });

  it('keeps a failed create draft and reports the agreed message', async () => {
    createNote.mockResolvedValue({ status: 'failure', error: new Error('offline') });
    const { component } = await mountNotes({ data: { notes: [] }, loading: false, refetch: vi.fn() });

    try {
      const input = document.body.querySelector<HTMLTextAreaElement>('textarea');
      const add = [...document.body.querySelectorAll('button')].find((button) => button.textContent?.includes('추가'));
      expect(input).not.toBeNull();
      expect(add).toBeDefined();
      if (!input || !add) return;

      input.value = '실패해도 남아야 하는 초안';
      input.dispatchEvent(new InputEvent('input', { bubbles: true }));
      add.click();
      await Promise.resolve();
      await tick();

      expect(input.value).toBe('실패해도 남아야 하는 초안');
      expect(toastError).toHaveBeenCalledOnce();
      expect(toastError).toHaveBeenCalledWith('노트를 만들지 못했어요.');
    } finally {
      await unmount(component);
    }
  });

  it('lets the new site create while the previous site request is pending without letting the old request clear the new draft', async () => {
    const first = Promise.withResolvers<{ status: 'success'; value: { id: string } }>();
    const second = Promise.withResolvers<{ status: 'success'; value: { id: string } }>();
    createNote.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const controller = createSiteQueryController({ data: { notes: [] }, loading: false, refetch: vi.fn() });
    const { component } = await mountNotes(controller);

    try {
      const input = document.body.querySelector<HTMLTextAreaElement>('textarea');
      const add = [...document.body.querySelectorAll('button')].find((button) => button.textContent?.includes('추가'));
      expect(input).not.toBeNull();
      expect(add).toBeDefined();
      if (!input || !add) return;

      input.value = '첫 번째 사이트 초안';
      input.dispatchEvent(new InputEvent('input', { bubbles: true }));
      add.click();
      await vi.waitFor(() => expect(createNote).toHaveBeenCalledOnce());

      currentSite.set('id', 'site-2');
      controller.publish({ data: { notes: [] }, loading: false, refetch: vi.fn() });
      await tick();

      input.value = '두 번째 사이트 초안';
      input.dispatchEvent(new InputEvent('input', { bubbles: true }));
      add.click();
      await vi.waitFor(() => expect(createNote).toHaveBeenCalledTimes(2));

      first.resolve({ status: 'success', value: { id: 'note-site-1' } });
      await tick();
      expect(input.value).toBe('두 번째 사이트 초안');

      add.click();
      await tick();
      expect(createNote).toHaveBeenCalledTimes(2);

      second.resolve({ status: 'success', value: { id: 'note-site-2' } });
      await vi.waitFor(() => expect(input.value).toBe(''));
    } finally {
      await unmount(component);
    }
  });
});
