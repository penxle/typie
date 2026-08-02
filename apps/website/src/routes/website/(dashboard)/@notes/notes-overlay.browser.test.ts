import '../../../../app.css';

import { createQuery } from '@mearie/svelte';
import { getAppContext } from '@typie/ui/context';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getNoteEditsContext } from '$lib/note/note-edit-state.svelte';
import { getNoteOperationsContext } from '$lib/note/note-mutation';
import { getNoteSyncContext } from '$lib/note/note-sync.svelte';
import Notes from './Notes.svelte';

vi.mock(import('@mearie/svelte'), async (importOriginal) => ({
  ...(await importOriginal()),
  createQuery: vi.fn(),
}));
vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('@typie/ui/context', () => ({ getAppContext: vi.fn() }));
vi.mock('mixpanel-browser', () => ({ default: { track: vi.fn() } }));
vi.mock('$app/navigation', () => ({ afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$lib/graphql', () => ({ cache: { invalidate: vi.fn() } }));
vi.mock(import('$lib/note/note-mutation'), { spy: true });
vi.mock(import('$lib/note/note-edit-state.svelte'), { spy: true });
vi.mock(import('$lib/note/note-sync.svelte'), { spy: true });

const app = {
  preference: { current: { currentSiteId: 'site-1' } },
  state: { notesOpen: true },
};
const noteEdits = {
  get: vi.fn(),
  remove: vi.fn(),
  sync: vi.fn(),
};
const noteSync = {
  isTerminallyDeleted: vi.fn(() => false),
  onTerminalDelete: vi.fn(() => vi.fn()),
  retainRelatedEntity: vi.fn(),
};
const noteOperations = {
  addEntity: vi.fn(),
  create: vi.fn(),
  delete: vi.fn(),
  move: vi.fn(),
  removeEntity: vi.fn(),
  update: vi.fn(),
};

const notes = Array.from({ length: 20 }, (_, index) => ({
  id: `note-${index}`,
  content: `note ${index}`,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:00.000Z',
  order: String(index).padStart(3, '0'),
  color: 'gray',
  status: 'OPEN' as const,
  site: { id: 'site-1' },
  entities: [],
}));

let component: Record<string, unknown> | undefined;

beforeEach(() => {
  app.state.notesOpen = true;
  vi.mocked(getAppContext).mockReturnValue(app as never);
  vi.mocked(getNoteEditsContext).mockReturnValue(noteEdits as never);
  vi.mocked(getNoteOperationsContext).mockReturnValue(noteOperations as never);
  vi.mocked(getNoteSyncContext).mockReturnValue(noteSync as never);
  vi.mocked(createQuery)
    .mockReset()
    .mockImplementationOnce(((_document: unknown, getVariables: () => unknown) => {
      getVariables();
      return { data: { notes }, loading: false, error: undefined, refetch: vi.fn() };
    }) as never)
    .mockReturnValue({ data: undefined, loading: false, error: undefined, refetch: vi.fn() } as never);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

describe('global notes overlay', () => {
  it('closes when the notes-list wrapper fills the scrolled viewport', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    component = mount(Notes, { target, intro: false });
    await tick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await vi.waitFor(() => expect(document.querySelector('[data-note-list]')).not.toBeNull());

    const scrollSurface = document.querySelector<HTMLElement>('[role="presentation"]');
    const noteList = document.querySelector<HTMLElement>('[data-note-list]');
    const notesArea = noteList?.parentElement;
    expect(scrollSurface).not.toBeNull();
    expect(notesArea).toBeInstanceOf(HTMLElement);
    if (!scrollSurface || !notesArea) throw new Error('Expected the notes overlay scroll surface and list area');
    expect(notesArea.contains(noteList)).toBe(true);
    expect(scrollSurface.scrollHeight).toBeGreaterThan(scrollSurface.clientHeight);

    scrollSurface.scrollTop = scrollSurface.scrollHeight;
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

    const scrollRect = scrollSurface.getBoundingClientRect();
    const notesRect = notesArea.getBoundingClientRect();
    const clientX = notesRect.left + notesRect.width / 2;
    const clientY = Math.min(notesRect.bottom, scrollRect.bottom) - 8;
    const hit = document.elementFromPoint(clientX, clientY);
    expect(hit).not.toBeNull();
    expect(hit === scrollSurface || hit?.closest('[data-notes-backdrop]') !== null).toBe(true);
    expect(app.state.notesOpen).toBe(true);
    hit?.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX, clientY }));

    expect(app.state.notesOpen).toBe(false);
  });
});
