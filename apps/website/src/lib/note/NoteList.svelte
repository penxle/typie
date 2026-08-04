<script lang="ts" module>
  import type { NoteReorderDirection, NoteReorderGeometry } from '$lib/note-reorder';
  import type { NoteListIdentity, VisibleNote } from './note-list-state.svelte';

  type ListNote = {
    id: string;
    order: string;
    status: string;
  };

  export type NoteListDragPosition = {
    clientX: number;
    clientY: number;
    direction: NoteReorderDirection;
    ghost: NoteReorderGeometry;
  };

  export type NoteListItemReorder = {
    enabled: boolean;
    dragging: boolean;
    ondragstart: () => boolean;
    ondragmove: (position: NoteListDragPosition) => void;
    ondragend: () => Promise<void>;
    ondragcancel: () => void;
  };

  type NoteListRenderProps<T extends ListNote> = {
    item: VisibleNote<T>;
    reorder: NoteListItemReorder;
  };
</script>

<script generics="T extends { id: string; order: string; status: string }" lang="ts">
  import * as Sentry from '@sentry/sveltekit';
  import { Toast } from '@typie/ui/notification';
  import { animateFlip } from '@typie/ui/utils';
  import { onDestroy } from 'svelte';
  import { resolveNextFractionalOrderMove } from '$lib/fractional-order';
  import { reorderedNoteIdsForDrag } from '$lib/note-reorder';
  import { getNoteOperationsContext } from './note-mutation';
  import { getNoteSyncContext } from './note-sync.svelte';
  import NotePresence from './NotePresence.svelte';
  import type { Snippet } from 'svelte';
  import type { NoteListState } from './note-list-state.svelte';

  type Props = {
    state: NoteListState<T>;
    identity: NoteListIdentity;
    authoritativeNotes: readonly T[];
    children: Snippet<[NoteListRenderProps<T>]>;
    presentationActive?: boolean;
    reorderEnabled?: boolean;
    onMoveSuccess?: (note: T) => void;
    onexitcomplete?: (item: T) => void;
    gap?: string;
    class?: string;
  };

  let {
    state: listState,
    identity,
    authoritativeNotes,
    children,
    presentationActive = true,
    reorderEnabled = true,
    onMoveSuccess,
    onexitcomplete,
    gap = '0px',
    class: className,
  }: Props = $props();

  const noteOperations = getNoteOperationsContext();
  const noteSync = getNoteSyncContext();
  let listElement = $state<HTMLDivElement>();
  let dragging = $state<{
    noteId: string;
    originalOrder: string[];
    position: NoteListDragPosition | null;
  } | null>(null);
  let activeIdentityKey: string | null = null;
  let ownedListState: NoteListState<T> | null = null;
  let ownedPresentationActive: boolean | null = null;
  let latestSyncGeneration = 0;
  let visibleSyncGeneration = 0;
  let suppressedFlipGeneration: number | null = null;
  let operationOwner = 0;
  let lastVisibleNoteIds: readonly string[] | null = null;
  let suppressNextFlipForCompletedExit = false;
  let preferredMove: { noteId: string } | null = null;
  let reconcileTask: Promise<void> | null = null;
  let lastAuthoritativeMembership: readonly string[] | null = null;
  let latestAuthoritativeNotes: readonly T[] = [];

  $effect.pre(() => {
    const nextIdentityKey = JSON.stringify([identity.siteId, identity.entityId, identity.status]);
    const nextAuthoritativeMembership = authoritativeNotes.map((note) => note.id).toSorted((left, right) => left.localeCompare(right));
    const suppressFlip = activeIdentityKey !== nextIdentityKey;
    const identityChanged = activeIdentityKey !== null && activeIdentityKey !== nextIdentityKey;
    const stateChanged = ownedListState !== null && ownedListState !== listState;
    const presentationChanged = ownedPresentationActive !== null && ownedPresentationActive !== presentationActive;
    const membershipChanged =
      !identityChanged &&
      !stateChanged &&
      lastAuthoritativeMembership !== null &&
      (nextAuthoritativeMembership.length !== lastAuthoritativeMembership.length ||
        nextAuthoritativeMembership.some((noteId, index) => noteId !== lastAuthoritativeMembership?.[index]));
    if (identityChanged || stateChanged || presentationChanged || (membershipChanged && (dragging !== null || reconcileTask !== null))) {
      operationOwner += 1;
      preferredMove = null;
      reconcileTask = null;
      if (stateChanged) ownedListState?.reset();
      else if (membershipChanged) listState.restoreAuthoritativeOrder();
      dragging = null;
    }
    activeIdentityKey = nextIdentityKey;
    ownedListState = listState;
    ownedPresentationActive = presentationActive;
    latestAuthoritativeNotes = authoritativeNotes;
    lastAuthoritativeMembership = nextAuthoritativeMembership;
    visibleSyncGeneration = ++latestSyncGeneration;
    if (suppressFlip || !presentationActive) suppressedFlipGeneration = visibleSyncGeneration;
    if (presentationActive) listState.sync(identity, authoritativeNotes);
    else listState.settle(identity, authoritativeNotes);
  });

  $effect(() => {
    const activeState = listState;
    const activeIdentity = { ...identity };
    const activePresentationActive = presentationActive;
    const siteId = activeIdentity.siteId;
    const entityId = activeIdentity.entityId;
    if (entityId) noteSync.retainRelatedEntity({ siteId, entityId });
    const unregisterDelete = noteSync.onTerminalDelete({
      siteId,
      listener: (noteId) => {
        if (!activePresentationActive) {
          activeState.settle(activeIdentity, latestAuthoritativeNotes);
          return;
        }
        const item = activeState.visibleNotes().find(({ note }) => note.id === noteId);
        if (!item) return;

        operationOwner += 1;
        preferredMove = null;
        reconcileTask = null;
        activeState.restoreAuthoritativeOrder();
        dragging = null;
        activeState.markDeleted(item.note);
      },
    });
    return unregisterDelete;
  });

  onDestroy(() => {
    operationOwner += 1;
    preferredMove = null;
    reconcileTask = null;
    ownedListState?.restoreAuthoritativeOrder();
    dragging = null;
  });

  const visibleNotes = $derived(listState.visibleNotes());

  $effect.pre(() => {
    const nextVisibleNoteIds = visibleNotes.map(({ note }) => note.id);
    const previousVisibleNoteIds = lastVisibleNoteIds;
    if (
      previousVisibleNoteIds !== null &&
      nextVisibleNoteIds.length === previousVisibleNoteIds.length &&
      nextVisibleNoteIds.every((noteId, index) => noteId === previousVisibleNoteIds[index])
    ) {
      return;
    }
    lastVisibleNoteIds = nextVisibleNoteIds;
    if (suppressNextFlipForCompletedExit) {
      suppressNextFlipForCompletedExit = false;
      return;
    }
    if (visibleSyncGeneration !== latestSyncGeneration) return;
    if (visibleSyncGeneration === suppressedFlipGeneration) {
      suppressedFlipGeneration = null;
      return;
    }
    if (listElement) void animateFlip('[data-note-list-owned-item]:not([data-note-list-dragging])', 'noteId', listElement);
  });

  const beginDrag = (noteId: string): boolean => {
    const item = visibleNotes.find(({ note }) => note.id === noteId);
    if (item === undefined || dragging !== null || !presentationActive || !reorderEnabled || item.deleting || item.presence === 'exiting') {
      return false;
    }

    const originalOrder = visibleNotes.map(({ note }) => note.id);
    if (!listState.beginReorder(originalOrder)) return false;

    dragging = { noteId, originalOrder, position: null };
    return true;
  };

  const updateDrag = (noteId: string, position: NoteListDragPosition): void => {
    const current = dragging;
    if (!current || !listElement || current.noteId !== noteId) return;
    current.position = position;

    const noteIds = visibleNotes.map(({ note }) => note.id);
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- This is one pointer event's geometry snapshot.
    const noteGeometries = new Map<string, NoteReorderGeometry>();
    for (const noteElement of listElement.querySelectorAll<HTMLElement>('[data-note-list-owned-item]')) {
      const itemId = noteElement.dataset.noteId;
      if (!itemId) continue;
      const { top, bottom } = noteElement.getBoundingClientRect();
      noteGeometries.set(itemId, { top, bottom });
    }
    noteGeometries.set(noteId, position.ghost);

    const nextOrder = reorderedNoteIdsForDrag(noteIds, noteId, position.direction, noteGeometries);
    if (nextOrder && nextOrder.some((itemId, index) => itemId !== noteIds[index])) {
      listState.beginReorder(nextOrder);
    }
  };

  const cancelDrag = (noteId: string): void => {
    if (dragging?.noteId !== noteId) return;
    dragging = null;
    listState.rollbackOrder();
  };

  const startReconcile = (noteId: string): Promise<void> => {
    preferredMove = { noteId };
    if (reconcileTask) return reconcileTask;

    const commitState = listState;
    const operation = operationOwner;
    let mutationFailureReported = false;
    const task = (async () => {
      while (operation === operationOwner) {
        const desiredOrder = commitState.desiredOrder();
        if (desiredOrder === null) return;
        const authoritativeOrders = commitState.authoritativeOrders();
        const authoritativeNoteIds = [...authoritativeOrders].toSorted((left, right) => left[1].localeCompare(right[1])).map(([id]) => id);
        if (
          desiredOrder.length === authoritativeNoteIds.length &&
          desiredOrder.every((noteId, index) => noteId === authoritativeNoteIds[index])
        ) {
          return;
        }

        const currentPreferredMove: { noteId: string } | null = preferredMove;
        const move = resolveNextFractionalOrderMove(authoritativeOrders, desiredOrder, currentPreferredMove?.noteId);
        const movedNote = move && commitState.authoritativeNote(move.key);
        if (!move || !movedNote) {
          throw new Error('Cannot reconcile note order with the current authoritative snapshot');
        }

        const outcome = await noteOperations.move(
          {
            noteId: movedNote.id,
            lowerOrder: move.lowerOrder,
            upperOrder: move.upperOrder,
          },
          {
            lastKnown: { siteId: identity.siteId, noteId: movedNote.id },
            analytics: {
              onSuccess: () => onMoveSuccess?.(movedNote),
            },
          },
        );
        if (operation !== operationOwner) return;
        if (outcome.status === 'failure') {
          if (commitState.desiredOrder() === null) return;
          mutationFailureReported = true;
          throw outcome.error;
        }
        const result = outcome.status === 'success' ? outcome.value : outcome.status === 'subscription_gated' ? false : null;
        if (result === false) {
          if (operation === operationOwner) {
            operationOwner += 1;
            commitState.restoreAuthoritativeOrder();
            dragging = null;
          }
          return;
        }
        if (result === null) return;
        if (result.id !== move.key) {
          throw new Error(`Move note response ${result.id} does not match ${move.key}`);
        }
        if (preferredMove === currentPreferredMove) preferredMove = null;
        const authoritativeOrderAdvanced = commitState.advanceAuthoritativeOrder(result.id, result.order);
        if (!authoritativeOrderAdvanced && commitState.desiredOrder() === desiredOrder) {
          throw new Error('Move note response did not advance the authoritative order');
        }
      }
    })()
      .catch((err: unknown) => {
        if (operation !== operationOwner) return;
        operationOwner += 1;
        commitState.restoreAuthoritativeOrder();
        dragging = null;
        if (!mutationFailureReported) {
          try {
            Sentry.captureException(err);
          } catch {
            // Error reporting must not prevent rollback or user feedback.
          }
        }
        Toast.error('순서를 바꾸지 못했어요.');
      })
      .finally(() => {
        if (reconcileTask !== task) return;
        preferredMove = null;
        reconcileTask = null;
      });

    reconcileTask = task;
    return task;
  };

  const finishDrag = async (noteId: string): Promise<void> => {
    const current = dragging;
    if (!current || current.noteId !== noteId) return;

    const noteIds = listState.visibleNotes().map(({ note }) => note.id);
    dragging = null;

    if (noteIds.every((itemId, index) => itemId === current.originalOrder[index])) {
      listState.rollbackOrder();
      return;
    }
    if (!listState.setDesiredOrder(noteIds)) {
      listState.rollbackOrder();
      return;
    }

    await startReconcile(noteId);
  };

  const reorderFor = (item: VisibleNote<T>): NoteListItemReorder => ({
    enabled:
      presentationActive &&
      reorderEnabled &&
      item.presence !== 'exiting' &&
      !item.deleting &&
      (dragging === null || dragging.noteId === item.note.id),
    dragging: dragging?.noteId === item.note.id,
    ondragstart: () => beginDrag(item.note.id),
    ondragmove: (position) => updateDrag(item.note.id, position),
    ondragend: () => finishDrag(item.note.id),
    ondragcancel: () => cancelDrag(item.note.id),
  });
</script>

<div bind:this={listElement} style:gap class={className} data-note-list role="list">
  {#each visibleNotes as item (item.note.id)}
    <div
      data-note-id={item.note.id}
      data-note-list-dragging={dragging?.noteId === item.note.id ? '' : undefined}
      data-note-list-owned-item
      role="listitem"
    >
      <NotePresence
        gap={visibleNotes.length > 1 ? gap : '0px'}
        onentercomplete={() => listState.finishEntering(item.note.id)}
        onexitcomplete={() => {
          suppressNextFlipForCompletedExit = true;
          if (listState.finishExiting(item.note.id)) {
            onexitcomplete?.(item.note);
          } else {
            suppressNextFlipForCompletedExit = false;
          }
        }}
        presence={item.presence}
      >
        {@render children({ item, reorder: reorderFor(item) })}
      </NotePresence>
    </div>
  {/each}
</div>
