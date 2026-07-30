/* eslint-disable svelte/prefer-svelte-reactivity -- Authoritative observations are updated inside pre-render sync and must not retrigger it. */
import { Toast } from '@typie/ui/notification';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { getNoteOperationsContext } from './note-mutation';
import { getNoteSyncContext } from './note-sync.svelte';
import type { NoteListState } from './note-list-state.svelte';

type NoteStatus = 'OPEN' | 'RESOLVED';

type ActionNote = {
  id: string;
  order: string;
  status: NoteStatus;
  updatedAt: string;
};

type StatusRequest = {
  owner: number;
  source: NoteStatus;
  target: NoteStatus;
  successfulUpdatedAt: string | null;
  restoredSource: boolean;
};

type StatusObservation = {
  status: NoteStatus;
  updatedAt: string;
};

type StatusTransfer = {
  source: NoteStatus;
  target: NoteStatus;
};

export class NoteActions<T extends ActionNote> {
  readonly #operations = getNoteOperationsContext();
  readonly #sync = getNoteSyncContext();
  readonly #pendingDeletionNoteIds = new SvelteSet<string>();
  readonly #statusRequests = new SvelteMap<string, StatusRequest>();
  readonly #statusObservations = new Map<string, StatusObservation>();
  readonly #statusTransfers = new SvelteMap<string, StatusTransfer>();
  #operationOwner = 0;
  #requestOwner = 0;
  #statusIdentityKey: string | null = null;
  #visibleStatuses = new Set<NoteStatus>();

  #clear(): void {
    this.#pendingDeletionNoteIds.clear();
    this.#statusRequests.clear();
    this.#statusObservations.clear();
    this.#statusTransfers.clear();
    this.#visibleStatuses.clear();
  }

  #setIdentity(siteId: string | null, entityId?: string): void {
    const identityKey = JSON.stringify([siteId, entityId]);
    if (this.#statusIdentityKey === identityKey) return;

    this.#operationOwner += 1;
    this.#clear();
    this.#statusIdentityKey = identityKey;
  }

  #reportStatusFailure(): void {
    Toast.error('상태를 바꾸지 못했어요.');
  }

  #finishConfirmedRequest(noteId: string): void {
    const request = this.#statusRequests.get(noteId);
    if (!request || request.successfulUpdatedAt === null) return;

    const observation = this.#statusObservations.get(noteId);
    const newerSource =
      request.successfulUpdatedAt !== null && observation?.status === request.source && observation.updatedAt > request.successfulUpdatedAt;
    if (observation?.status === request.target || newerSource) {
      this.#statusRequests.delete(noteId);
      const transfer = this.#statusTransfers.get(noteId);
      if (newerSource && transfer?.source === request.source && transfer.target === request.target) {
        this.#statusTransfers.delete(noteId);
      }
    }
  }

  isPendingDeletion(noteId: string): boolean {
    return this.#pendingDeletionNoteIds.has(noteId);
  }

  isCancelling(noteId: string): boolean {
    const request = this.#statusRequests.get(noteId);
    const transfer = this.#statusTransfers.get(noteId);
    return request?.target === 'OPEN' && transfer?.source === 'OPEN' && transfer.target === 'RESOLVED';
  }

  isResolving(noteId: string): boolean {
    const request = this.#statusRequests.get(noteId);
    const transfer = this.#statusTransfers.get(noteId);
    return request?.target === 'RESOLVED' || this.isCancelling(noteId) || transfer?.target === 'RESOLVED';
  }

  syncStatus({
    siteId,
    entityId,
    notes,
    visibleStatuses,
  }: {
    siteId: string | null;
    entityId?: string;
    notes: readonly T[];
    visibleStatuses: readonly NoteStatus[];
  }): void {
    this.#setIdentity(siteId, entityId);

    const visible = new Set(visibleStatuses);
    this.#visibleStatuses = visible;
    for (const [noteId, transfer] of this.#statusTransfers) {
      if (!visible.has(transfer.source)) this.#statusTransfers.delete(noteId);
    }

    const nextNoteIds = new Set<string>();
    for (const note of notes) {
      nextNoteIds.add(note.id);
      const previous = this.#statusObservations.get(note.id);
      if (previous && previous.status !== note.status) {
        const transfer = this.#statusTransfers.get(note.id);
        const request = this.#statusRequests.get(note.id);
        if (transfer && note.status === transfer.source) {
          this.#statusTransfers.delete(note.id);
        } else if (request?.source === previous.status && request.target === note.status) {
          // A local request already owns this transfer, which may have completed before confirmation arrived.
        } else if (visible.has(previous.status)) {
          this.#statusTransfers.set(note.id, { source: previous.status, target: note.status });
        } else {
          this.#statusTransfers.delete(note.id);
        }
      }
      this.#statusObservations.set(note.id, { status: note.status, updatedAt: note.updatedAt });
      this.#finishConfirmedRequest(note.id);
    }

    for (const noteId of this.#statusObservations.keys()) {
      if (nextNoteIds.has(noteId)) continue;
      this.#statusObservations.delete(noteId);
      this.#statusRequests.delete(noteId);
      this.#statusTransfers.delete(noteId);
    }
  }

  isStatusAdmitted(noteId: string, status: NoteStatus): boolean {
    const request = this.#statusRequests.get(noteId);
    return request?.source !== status && this.#statusTransfers.get(noteId)?.target !== status;
  }

  finishStatusTransfer(noteId: string, source: NoteStatus): void {
    if (this.#statusTransfers.get(noteId)?.source === source) {
      this.#statusTransfers.delete(noteId);
    }
  }

  activate({ siteId, entityId, onTerminal }: { siteId: string; entityId?: string; onTerminal?: (noteId: string) => void }): () => void {
    this.#setIdentity(siteId, entityId);
    const operationOwner = this.#operationOwner;
    const unregister = this.#sync.onTerminalDelete({
      siteId,
      listener: (noteId) => {
        if (operationOwner !== this.#operationOwner) return;
        this.#pendingDeletionNoteIds.delete(noteId);
        this.#statusRequests.delete(noteId);
        this.#statusObservations.delete(noteId);
        this.#statusTransfers.delete(noteId);
        try {
          onTerminal?.(noteId);
        } catch {
          // Surface cleanup is best-effort after shared state has converged.
        }
      },
    });
    return () => {
      unregister();
      if (operationOwner !== this.#operationOwner) return;
      this.#operationOwner += 1;
      this.#pendingDeletionNoteIds.clear();
      this.#statusRequests.clear();
    };
  }

  async delete({
    noteId,
    siteId,
    state,
    onSuccess,
  }: {
    noteId: string;
    siteId: string;
    state: NoteListState<T>;
    onSuccess?: () => void;
  }): Promise<void> {
    if (!state.markDeleting(noteId)) return;
    this.#pendingDeletionNoteIds.add(noteId);
    const operationOwner = this.#operationOwner;

    const outcome = await this.#operations.delete(
      { noteId },
      {
        lastKnown: { siteId, noteId },
        analytics: {
          onSuccess,
          onTerminal: onSuccess,
        },
      },
    );
    if (operationOwner !== this.#operationOwner) return;
    if (this.#sync.isTerminallyDeleted(siteId, noteId)) return;
    if (outcome.status !== 'failure') return;

    this.#pendingDeletionNoteIds.delete(noteId);
    state.clearDeleting(noteId);
    Toast.error('노트를 삭제하지 못했어요.');
  }

  async toggleStatus({
    note,
    states,
    siteId,
    onSuccess,
  }: {
    note: T;
    states: Partial<Record<'OPEN' | 'RESOLVED', NoteListState<T>>>;
    siteId: string;
    onSuccess?: (status: string) => void;
  }): Promise<void> {
    if (this.#statusRequests.has(note.id)) return;

    const transfer = this.#statusTransfers.get(note.id);
    const source = transfer?.target ?? note.status;
    const target = transfer?.source ?? (source === 'OPEN' ? 'RESOLVED' : 'OPEN');
    const restoredSource = transfer ? (states[transfer.source]?.restoreExiting(note.id) ?? false) : false;
    if (!transfer && this.#visibleStatuses.has(source)) {
      this.#statusTransfers.set(note.id, { source, target });
    }
    const requestOwner = ++this.#requestOwner;
    this.#statusRequests.set(note.id, {
      owner: requestOwner,
      source,
      target,
      successfulUpdatedAt: null,
      restoredSource,
    });
    const operationOwner = this.#operationOwner;

    const outcome = await this.#operations.update(
      { noteId: note.id, status: target },
      {
        lastKnown: { siteId, noteId: note.id },
        analytics: {
          onSuccess: () => onSuccess?.(target),
        },
      },
    );
    if (operationOwner !== this.#operationOwner) return;
    if (this.#sync.isTerminallyDeleted(siteId, note.id)) return;
    const currentRequest = this.#statusRequests.get(note.id);
    if (currentRequest?.owner !== requestOwner) return;

    if (this.#statusObservations.get(note.id)?.status === target) {
      this.#statusRequests.delete(note.id);
      return;
    }

    if (outcome.status === 'success') {
      this.#statusRequests.set(note.id, {
        ...currentRequest,
        successfulUpdatedAt: outcome.value.updatedAt,
      });
      this.#finishConfirmedRequest(note.id);
      return;
    }

    this.#statusRequests.delete(note.id);
    if (currentRequest.restoredSource && transfer) {
      states[transfer.source]?.resumeExiting(note.id);
    } else if (!transfer) {
      const activeTransfer = this.#statusTransfers.get(note.id);
      if (activeTransfer?.source === source && activeTransfer.target === target) {
        this.#statusTransfers.delete(note.id);
      }
    }

    if (outcome.status === 'failure') this.#reportStatusFailure();
  }
}
