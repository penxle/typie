import { SvelteSet } from 'svelte/reactivity';
import { describe, expect, it, vi } from 'vitest';
import { createTrackedEffect } from '../editor-ffi/editor-effect-harness.svelte';
import { NoteSync } from './note-sync.svelte';
import type { NoteUpdate } from './note-sync.svelte';

const update = (overrides: Partial<NoteUpdate> = {}): NoteUpdate => ({
  kind: 'UPDATED',
  noteId: 'note-1',
  siteId: 'site-1',
  ...overrides,
});

function createHarness() {
  const invalidateGlobal = vi.fn();
  const invalidateEntity = vi.fn();
  const sync = new NoteSync({
    invalidateGlobal,
    invalidateEntity,
  });

  return { sync, invalidateGlobal, invalidateEntity };
}

describe('NoteSync invalidation routing', () => {
  it('does not make a registration effect depend on the registries it mutates', async () => {
    const { sync } = createHarness();
    let effectRuns = 0;
    const tracked = createTrackedEffect(() => {
      effectRuns += 1;
      if (effectRuns > 1) return;

      sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
      const unregisterDelete = sync.onTerminalDelete({ siteId: 'site-1', listener: vi.fn() });
      return () => {
        unregisterDelete();
      };
    });

    try {
      await tracked.flush();
      await tracked.flush();

      expect(effectRuns).toBe(1);
    } finally {
      tracked.destroy();
    }
  });

  it('does not make a registration effect depend on state read by a replayed terminal listener', async () => {
    const { sync } = createHarness();
    sync.receiveRemote(update({ kind: 'DELETED' }));
    const selectedNoteIds = new SvelteSet<string>();
    let effectRuns = 0;
    const tracked = createTrackedEffect(() => {
      effectRuns += 1;
      return sync.onTerminalDelete({
        siteId: 'site-1',
        listener: (noteId) => {
          selectedNoteIds.has(noteId);
        },
      });
    });

    try {
      await tracked.flush();
      selectedNoteIds.add('note-1');
      await tracked.flush();

      expect(effectRuns).toBe(1);
    } finally {
      tracked.destroy();
    }
  });

  it('routes a remote update to the site global list and every retained related entity for that site', () => {
    const { sync, invalidateGlobal, invalidateEntity } = createHarness();
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-2' });
    sync.retainRelatedEntity({ siteId: 'site-2', entityId: 'other-site' });

    sync.receiveRemote(update());

    expect(invalidateGlobal).toHaveBeenCalledOnce();
    expect(invalidateGlobal).toHaveBeenCalledWith('site-1');
    expect(invalidateEntity.mock.calls).toEqual([
      ['site-1', 'entity-1'],
      ['site-1', 'entity-2'],
    ]);
  });

  it('routes a local update to retained entities for that site only', () => {
    const { sync, invalidateGlobal, invalidateEntity } = createHarness();
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-2' });
    sync.retainRelatedEntity({ siteId: 'site-2', entityId: 'other-site' });

    sync.publishLocal(update());

    expect(invalidateGlobal.mock.calls).toEqual([['site-1']]);
    expect(invalidateEntity.mock.calls).toEqual([
      ['site-1', 'entity-1'],
      ['site-1', 'entity-2'],
    ]);
  });

  it('retains an observed entity for later wildcard fanout', () => {
    const { sync, invalidateEntity } = createHarness();
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
    sync.publishLocal(update());

    expect(invalidateEntity.mock.calls).toEqual([['site-1', 'entity-1']]);
  });

  it('retains the same observed entity idempotently', () => {
    const { sync, invalidateEntity } = createHarness();
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });

    sync.publishLocal(update());

    expect(invalidateEntity.mock.calls).toEqual([['site-1', 'entity-1']]);
  });
});

describe('NoteSync terminal deletion state', () => {
  it.each([
    ['remote delete', (sync: NoteSync) => sync.receiveRemote(update({ kind: 'DELETED' }))],
    ['local delete', (sync: NoteSync) => sync.publishLocal(update({ kind: 'DELETED' }))],
    ['local not_found', (sync: NoteSync) => sync.markNotFound({ siteId: 'site-1', noteId: 'note-1' })],
  ])('records and notifies one site-scoped terminal deletion for %s', (_name, markDeleted) => {
    const { sync } = createHarness();
    const siteListener = vi.fn();
    const otherSiteListener = vi.fn();
    sync.onTerminalDelete({ siteId: 'site-1', listener: siteListener });
    sync.onTerminalDelete({ siteId: 'site-2', listener: otherSiteListener });

    markDeleted(sync);
    markDeleted(sync);

    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);
    expect(sync.isTerminallyDeleted('site-2', 'note-1')).toBe(false);
    expect(siteListener).toHaveBeenCalledOnce();
    expect(siteListener).toHaveBeenCalledWith('note-1');
    expect(otherSiteListener).not.toHaveBeenCalled();
  });

  it('stops terminal notifications after the observer is disposed', () => {
    const { sync } = createHarness();
    const listener = vi.fn();
    const dispose = sync.onTerminalDelete({ siteId: 'site-1', listener });

    dispose();
    dispose();
    sync.receiveRemote(update({ kind: 'DELETED' }));

    expect(listener).not.toHaveBeenCalled();
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);
  });

  it('replays an existing site tombstone once to a newly registered listener and keeps later duplicates deduped', () => {
    const { sync } = createHarness();
    const siteListener = vi.fn();
    const otherSiteListener = vi.fn();
    sync.receiveRemote(update({ kind: 'DELETED' }));

    sync.onTerminalDelete({ siteId: 'site-1', listener: siteListener });
    sync.onTerminalDelete({ siteId: 'site-2', listener: otherSiteListener });
    sync.receiveRemote(update({ kind: 'DELETED' }));

    expect(siteListener.mock.calls).toEqual([['note-1']]);
    expect(otherSiteListener).not.toHaveBeenCalled();
  });

  it('isolates an exception while replaying an existing tombstone to a new listener', () => {
    const { sync } = createHarness();
    const throwingListener = vi.fn(() => {
      throw new Error('listener failed');
    });
    const laterListener = vi.fn();
    sync.receiveRemote(update({ kind: 'DELETED' }));

    expect(() => sync.onTerminalDelete({ siteId: 'site-1', listener: throwingListener })).not.toThrow();
    sync.onTerminalDelete({ siteId: 'site-1', listener: laterListener });

    expect(throwingListener).toHaveBeenCalledOnce();
    expect(laterListener.mock.calls).toEqual([['note-1']]);
  });

  it('isolates a throwing terminal listener and completes deletion dispatch and invalidation', () => {
    const { sync, invalidateGlobal, invalidateEntity } = createHarness();
    const firstListener = vi.fn(() => {
      throw new Error('listener failed');
    });
    const secondListener = vi.fn();
    sync.onTerminalDelete({ siteId: 'site-1', listener: firstListener });
    sync.onTerminalDelete({ siteId: 'site-1', listener: secondListener });
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });

    expect(() => sync.receiveRemote(update({ kind: 'DELETED' }))).not.toThrow();

    expect(firstListener).toHaveBeenCalledOnce();
    expect(secondListener).toHaveBeenCalledOnce();
    expect(secondListener).toHaveBeenCalledWith('note-1');
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);
    expect(invalidateGlobal.mock.calls).toEqual([['site-1']]);
    expect(invalidateEntity.mock.calls).toEqual([['site-1', 'entity-1']]);
  });

  it('invalidates a local not_found through the wildcard path exactly once', () => {
    const { sync, invalidateGlobal, invalidateEntity } = createHarness();
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
    sync.retainRelatedEntity({ siteId: 'site-2', entityId: 'other-site' });

    expect(sync.markNotFound({ siteId: 'site-1', noteId: 'note-1' })).toBe(true);
    expect(sync.markNotFound({ siteId: 'site-1', noteId: 'note-1' })).toBe(false);

    expect(invalidateGlobal.mock.calls).toEqual([['site-1']]);
    expect(invalidateEntity.mock.calls).toEqual([['site-1', 'entity-1']]);
  });

  it('retains tombstones across registrations and site activity until an explicit create clears the same site and id', () => {
    const { sync } = createHarness();
    sync.markNotFound({ siteId: 'site-1', noteId: 'note-1' });

    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });
    sync.receiveRemote(update({ siteId: 'site-2', kind: 'CREATED' }));
    sync.receiveRemote(update({ siteId: 'site-1', noteId: 'different-note', kind: 'CREATED' }));

    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);

    sync.publishLocal(update({ kind: 'CREATED' }));

    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(false);
  });
});
