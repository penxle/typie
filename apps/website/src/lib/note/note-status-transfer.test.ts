import { mount, tick, unmount } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NoteActions } from './note-actions.svelte';
import NoteStatusTransferTestHost from './note-status-transfer-test-host.svelte';

vi.mock('@typie/ui/notification', () => ({ Toast: { error: vi.fn() } }));
vi.mock('./note-mutation', () => ({
  getNoteOperationsContext: () => ({
    delete: vi.fn(),
    update: vi.fn(),
  }),
}));
vi.mock('./note-sync.svelte', () => ({
  getNoteSyncContext: () => ({
    isTerminallyDeleted: () => false,
    onTerminalDelete: () => vi.fn(),
  }),
}));

type Note = {
  id: string;
  order: string;
  status: 'OPEN' | 'RESOLVED';
  updatedAt: string;
};

const note = (status: Note['status']): Note => ({
  id: 'note-1',
  order: '100',
  status,
  updatedAt: status === 'OPEN' ? '2026-07-29T00:00:00.000Z' : '2026-07-29T00:00:01.000Z',
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe('note status transfer render admission', () => {
  it('keeps the source exit rendered without admitting the destination early', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const actions = new NoteActions<Note>();
    const identity = new SvelteMap([['entityId', 'entity-1']]);
    const notes = new SvelteMap([['note-1', note('OPEN')]]);
    const component = mount(NoteStatusTransferTestHost, { target, props: { actions, identity, notes } });

    try {
      await tick();
      notes.set('note-1', note('RESOLVED'));
      await tick();

      expect(target.querySelector<HTMLElement>(':scope [data-list="open"] [data-note-id="note-1"]')?.dataset.presence).toBe('exiting');
      expect(target.querySelector(':scope [data-list="resolved"] [data-note-id="note-1"]')).toBeNull();

      target.querySelector<HTMLButtonElement>('button')?.click();
      await tick();

      expect(target.querySelector(':scope [data-list="open"] [data-note-id="note-1"]')).toBeNull();
      expect(target.querySelector(':scope [data-list="resolved"] [data-note-id="note-1"]')).not.toBeNull();
    } finally {
      await unmount(component);
    }
  });

  it('baselines the first snapshot for a replacement identity without carrying the previous transfer', async () => {
    const target = document.createElement('div');
    document.body.append(target);
    const actions = new NoteActions<Note>();
    const identity = new SvelteMap([['entityId', 'entity-1']]);
    const notes = new SvelteMap([['note-1', note('OPEN')]]);
    const component = mount(NoteStatusTransferTestHost, { target, props: { actions, identity, notes } });

    try {
      await tick();
      notes.set('note-1', note('RESOLVED'));
      await tick();
      expect(target.querySelector(':scope [data-list="resolved"] [data-note-id="note-1"]')).toBeNull();

      identity.set('entityId', 'entity-2');
      await tick();

      expect(target.querySelector(':scope [data-list="open"] [data-note-id="note-1"]')).toBeNull();
      expect(target.querySelector(':scope [data-list="resolved"] [data-note-id="note-1"]')).not.toBeNull();
    } finally {
      await unmount(component);
    }
  });
});
