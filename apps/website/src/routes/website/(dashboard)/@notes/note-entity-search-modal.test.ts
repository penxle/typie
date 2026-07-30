import { mount, tick, unmount } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import NoteEntitySearchModal from './NoteEntitySearchModal.svelte';

const { addEntity, createQuery } = vi.hoisted(() => ({
  addEntity: vi.fn(),
  createQuery: vi.fn(),
}));

vi.mock('@mearie/svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@mearie/svelte')>()),
  createQuery,
}));
vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('@typie/ui/context', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@typie/ui/context')>()),
  getAppContext: () => ({
    preference: { current: { currentSiteId: 'site-1' } },
  }),
}));
vi.mock('$lib/note/note-mutation', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/note/note-mutation')>()),
  getNoteOperationsContext: () => ({ addEntity }),
}));
vi.mock('../@context-menu/EntityIcon.svelte', () => ({
  default: () => {
    // This test exercises modal selection, not entity icon rendering.
  },
}));

describe('note entity search modal', () => {
  beforeEach(() => {
    vi.useFakeTimers();
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
    addEntity.mockReset();
    addEntity.mockResolvedValue({ status: 'success' });
    createQuery.mockReset();
    createQuery
      .mockReturnValueOnce({
        data: {
          me: {
            id: 'user-1',
            recentlyViewedEntities: [
              {
                id: 'entity-1',
                slug: 'recent-document',
                site: { id: 'site-1' },
                node: { __typename: 'Document', id: 'document-1', title: '최근 문서' },
              },
            ],
          },
        },
        loading: false,
        error: undefined,
        refetch: vi.fn(),
      })
      .mockReturnValueOnce({
        data: undefined,
        loading: false,
        error: new Error('offline'),
        refetch: vi.fn(),
      });
  });

  afterEach(() => {
    vi.useRealTimers();
    Reflect.deleteProperty(Element.prototype, 'animate');
    document.body.replaceChildren();
  });

  it('does not select a hidden recent item from the search error state', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteEntitySearchModal, {
      target,
      props: {
        noteId: 'note-1',
        existingEntityIds: [],
        open: true,
        onclose: vi.fn(),
      },
    });

    try {
      await tick();
      const input = document.body.querySelector<HTMLInputElement>('input[placeholder="항목 검색..."]');
      expect(input).not.toBeNull();
      if (!input) return;

      input.value = '검색어';
      input.dispatchEvent(new InputEvent('input', { bubbles: true }));
      await vi.advanceTimersByTimeAsync(300);
      await tick();

      expect(document.body.textContent).toContain('검색 결과를 불러오지 못했어요.');

      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
      await tick();

      expect(addEntity).not.toHaveBeenCalled();
    } finally {
      await unmount(component);
    }
  });

  it('selects the first result after navigating while the list was empty', async () => {
    const recent = new SvelteMap<string, { data?: { me: { id: string; recentlyViewedEntities: ReturnType<typeof recentEntity>[] } } }>([
      ['snapshot', { data: { me: { id: 'user-1', recentlyViewedEntities: [] } } }],
    ]);
    createQuery.mockReset();
    createQuery
      .mockReturnValueOnce({
        get data() {
          return recent.get('snapshot')?.data;
        },
        loading: false,
        error: undefined,
        refetch: vi.fn(),
      })
      .mockReturnValueOnce({ data: undefined, loading: false, error: undefined, refetch: vi.fn() });
    const { component, input } = await mountModal();

    try {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
      recent.set('snapshot', { data: { me: { id: 'user-1', recentlyViewedEntities: [recentEntity('entity-1', '첫 문서')] } } });
      await tick();

      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
      await vi.waitFor(() => expect(addEntity).toHaveBeenCalledOnce());
      expect(addEntity.mock.calls[0]?.[0]).toEqual({ noteId: 'note-1', entityId: 'entity-1' });
    } finally {
      await unmount(component);
    }
  });

  it('normalizes the selection when the result list shrinks', async () => {
    const recent = new SvelteMap<string, { data?: { me: { id: string; recentlyViewedEntities: ReturnType<typeof recentEntity>[] } } }>([
      [
        'snapshot',
        {
          data: {
            me: {
              id: 'user-1',
              recentlyViewedEntities: [recentEntity('entity-1', '첫 문서'), recentEntity('entity-2', '두 번째 문서')],
            },
          },
        },
      ],
    ]);
    createQuery.mockReset();
    createQuery
      .mockReturnValueOnce({
        get data() {
          return recent.get('snapshot')?.data;
        },
        loading: false,
        error: undefined,
        refetch: vi.fn(),
      })
      .mockReturnValueOnce({ data: undefined, loading: false, error: undefined, refetch: vi.fn() });
    const { component, input } = await mountModal();

    try {
      const second = document.body.querySelector<HTMLButtonElement>('[data-note-search-index="1"]');
      expect(second).not.toBeNull();
      second?.dispatchEvent(new PointerEvent('pointermove', { bubbles: true }));
      recent.set('snapshot', { data: { me: { id: 'user-1', recentlyViewedEntities: [recentEntity('entity-1', '첫 문서')] } } });
      await tick();

      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
      await vi.waitFor(() => expect(addEntity).toHaveBeenCalledOnce());
      expect(addEntity.mock.calls[0]?.[0]).toEqual({ noteId: 'note-1', entityId: 'entity-1' });
    } finally {
      await unmount(component);
    }
  });
});

const recentEntity = (id: string, title: string) => ({
  id,
  slug: id,
  site: { id: 'site-1' },
  node: { __typename: 'Document' as const, id: `document-${id}`, title },
});

const mountModal = async () => {
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(NoteEntitySearchModal, {
    target,
    props: {
      noteId: 'note-1',
      existingEntityIds: [],
      open: true,
      onclose: vi.fn(),
    },
  });
  await tick();
  const input = document.body.querySelector<HTMLInputElement>('input[placeholder="항목 검색..."]');
  expect(input).not.toBeNull();
  if (!input) throw new Error('Missing search input');
  return { component, input };
};
