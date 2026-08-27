/* eslint-disable svelte/prefer-svelte-reactivity -- Subscription registries must not retrigger the effects that register them. */
import { createStableContext } from '@typie/ui/context/stable';
import { untrack } from 'svelte';

export type NoteUpdate = {
  kind: 'CREATED' | 'UPDATED' | 'DELETED';
  noteId: string;
  siteId: string;
};

type NoteSyncOptions = {
  invalidateGlobal: (siteId: string) => void;
  invalidateEntity: (siteId: string, entityId: string) => void;
};

type RelatedEntity = {
  siteId: string;
  entityId: string;
};

type TerminalDeleteRegistration = {
  siteId: string;
  listener: (noteId: string) => void;
};

export class NoteSync {
  readonly #invalidateGlobal: NoteSyncOptions['invalidateGlobal'];
  readonly #invalidateEntity: NoteSyncOptions['invalidateEntity'];
  readonly #relatedEntitiesBySite = new Map<string, Set<string>>();
  readonly #terminalNoteIdsBySite = new Map<string, Set<string>>();
  readonly #terminalListenersBySite = new Map<string, Set<TerminalDeleteRegistration['listener']>>();
  constructor({ invalidateGlobal, invalidateEntity }: NoteSyncOptions) {
    this.#invalidateGlobal = invalidateGlobal;
    this.#invalidateEntity = invalidateEntity;
  }

  #receive(update: NoteUpdate): void {
    if (update.kind === 'CREATED') {
      this.#terminalNoteIds(update.siteId).delete(update.noteId);
    } else if (update.kind === 'DELETED') {
      this.#markTerminallyDeleted(update.siteId, update.noteId);
    }

    this.#invalidate(update);
  }

  #invalidate(update: NoteUpdate): void {
    this.#invalidateGlobal(update.siteId);

    // The stream does not include related entity IDs, so every observed projection must refetch to converge.
    for (const entityId of this.#relatedEntities(update.siteId)) {
      this.#invalidateEntity(update.siteId, entityId);
    }
  }

  #markTerminallyDeleted(siteId: string, noteId: string): boolean {
    const noteIds = this.#terminalNoteIds(siteId);
    if (noteIds.has(noteId)) {
      return false;
    }

    noteIds.add(noteId);
    for (const listener of this.#terminalListeners(siteId)) {
      this.#notifyTerminalListener(listener, noteId);
    }
    return true;
  }

  #notifyTerminalListener(listener: TerminalDeleteRegistration['listener'], noteId: string): void {
    try {
      untrack(() => listener(noteId));
    } catch {
      // Terminal observers are best-effort and must not interrupt state propagation.
    }
  }

  #relatedEntities(siteId: string): Set<string> {
    let entityIds = this.#relatedEntitiesBySite.get(siteId);
    if (!entityIds) {
      entityIds = new Set();
      this.#relatedEntitiesBySite.set(siteId, entityIds);
    }
    return entityIds;
  }

  #terminalNoteIds(siteId: string): Set<string> {
    let noteIds = this.#terminalNoteIdsBySite.get(siteId);
    if (!noteIds) {
      noteIds = new Set();
      this.#terminalNoteIdsBySite.set(siteId, noteIds);
    }
    return noteIds;
  }

  #terminalListeners(siteId: string): Set<TerminalDeleteRegistration['listener']> {
    let listeners = this.#terminalListenersBySite.get(siteId);
    if (!listeners) {
      listeners = new Set();
      this.#terminalListenersBySite.set(siteId, listeners);
    }
    return listeners;
  }

  receiveRemote(update: NoteUpdate): void {
    this.#receive(update);
  }

  publishLocal(update: NoteUpdate): void {
    this.#receive(update);
  }

  markNotFound({ siteId, noteId }: { siteId: string; noteId: string }): boolean {
    if (!this.#markTerminallyDeleted(siteId, noteId)) {
      return false;
    }

    this.#invalidate({ kind: 'DELETED', siteId, noteId });
    return true;
  }

  retainRelatedEntity({ siteId, entityId }: RelatedEntity): void {
    this.#relatedEntities(siteId).add(entityId);
  }

  onTerminalDelete({ siteId, listener }: TerminalDeleteRegistration): () => void {
    const listeners = this.#terminalListeners(siteId);
    listeners.add(listener);
    for (const noteId of this.#terminalNoteIds(siteId)) {
      this.#notifyTerminalListener(listener, noteId);
    }

    let disposed = false;
    return () => {
      if (disposed) return;
      disposed = true;
      listeners.delete(listener);
    };
  }

  isTerminallyDeleted(siteId: string, noteId: string): boolean {
    return this.#terminalNoteIdsBySite.get(siteId)?.has(noteId) ?? false;
  }
}

const [getNoteSyncContext, setNoteSyncContext] = createStableContext<NoteSync>('note.NoteSync');

export { getNoteSyncContext, setNoteSyncContext };
