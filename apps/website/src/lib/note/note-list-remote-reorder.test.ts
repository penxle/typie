import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import NoteListRemoteReorderTestHost from './note-list-remote-reorder-test-host.svelte';
import { NoteListState } from './note-list-state.svelte';

const { onTerminalDelete, retainRelatedEntity } = vi.hoisted(() => ({
  onTerminalDelete: vi.fn(() => vi.fn()),
  retainRelatedEntity: vi.fn(),
}));

vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('./note-sync.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./note-sync.svelte')>()),
  getNoteSyncContext: () => ({ onTerminalDelete, retainRelatedEntity }),
}));

describe('NoteList remote reorder', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    document.body.replaceChildren();
  });

  it('animates directly from the current order to the remote order', async () => {
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

    const state = new NoteListState<{ id: string; order: string; status: string }>({
      isTerminallyDeleted: () => false,
    });
    const initial = [
      { id: 'a', order: '100', status: 'OPEN' },
      { id: 'b', order: '200', status: 'OPEN' },
      { id: 'c', order: '300', status: 'OPEN' },
    ];
    const target = document.createElement('div');
    document.body.append(target);
    const remote = [
      { id: 'b', order: '100', status: 'OPEN' },
      { id: 'c', order: '200', status: 'OPEN' },
      { id: 'a', order: '300', status: 'OPEN' },
    ];
    const component = mount(NoteListRemoteReorderTestHost, {
      target,
      props: { state, initialNotes: initial, remoteNotes: remote },
    });

    try {
      await tick();
      target.querySelector<HTMLButtonElement>('[data-test-apply-remote]')?.click();
      await tick();
      await tick();

      const items = [...target.querySelectorAll<HTMLElement>('[data-note-list-owned-item]')];
      expect(items.map((item) => item.dataset.noteId)).toEqual(['b', 'c', 'a']);
      expect(items.every((item) => item.style.transition.includes('transform 300ms'))).toBe(true);
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
