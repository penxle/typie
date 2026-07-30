<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { createDragScroll, elementScrollViewport } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, tick } from 'svelte';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { NoteActions } from '$lib/note/note-actions.svelte';
  import { NoteListState } from '$lib/note/note-list-state.svelte';
  import { getNoteOperationsContext } from '$lib/note/note-mutation';
  import { getNoteSyncContext } from '$lib/note/note-sync.svelte';
  import NoteList from '$lib/note/NoteList.svelte';
  import RelatedNoteItem from '$lib/note/RelatedNoteItem.svelte';
  import { graphql } from '$mearie';
  import type { NoteListDragPosition, NoteListItemReorder } from '$lib/note/NoteList.svelte';
  import type { DocumentPanelV2_Note_entity$key } from '$mearie';

  type Props = {
    entity$key: DocumentPanelV2_Note_entity$key;
  };

  let { entity$key }: Props = $props();

  const noteOperations = getNoteOperationsContext();
  const noteSync = getNoteSyncContext();
  const entity = createFragment(
    graphql(`
      fragment DocumentPanelV2_Note_entity on Entity {
        id
        site {
          id
        }
        notes {
          id
          order
          status
          updatedAt
          ...RelatedNoteItem_note
        }
      }
    `),
    () => entity$key,
  );

  const notes = $derived(entity.data.notes);
  type RelatedNote = (typeof notes)[number];

  const siteId = $derived(entity.data.site.id);
  const entityId = $derived(entity.data.id);
  const noteActions = new NoteActions<RelatedNote>();
  const openListState = new NoteListState<RelatedNote>({
    isTerminallyDeleted: (targetSiteId, noteId) => noteSync.isTerminallyDeleted(targetSiteId, noteId),
    isPendingDeletion: (noteId) => noteActions.isPendingDeletion(noteId),
  });
  const resolvedListState = new NoteListState<RelatedNote>({
    isTerminallyDeleted: (targetSiteId, noteId) => noteSync.isTerminallyDeleted(targetSiteId, noteId),
    isPendingDeletion: (noteId) => noteActions.isPendingDeletion(noteId),
  });

  const openNotes = $derived(notes.filter((note) => note.status === 'OPEN' && noteActions.isStatusAdmitted(note.id, 'OPEN')));
  const resolvedNotes = $derived(notes.filter((note) => note.status === 'RESOLVED' && noteActions.isStatusAdmitted(note.id, 'RESOLVED')));
  const openVisibleCount = $derived(openListState.visibleNotes().length);
  const resolvedVisibleCount = $derived(resolvedListState.visibleNotes().length);

  let resolvedExpanded = $state(false);

  $effect.pre(() => {
    noteActions.syncStatus({
      siteId,
      entityId,
      notes,
      visibleStatuses: resolvedExpanded ? ['OPEN', 'RESOLVED'] : ['OPEN'],
    });
  });

  let createRequestId = 0;
  let activeCreateRequest = $state<{ entityId: string; id: number; siteId: string } | null>(null);
  const createInFlight = $derived(activeCreateRequest?.siteId === siteId && activeCreateRequest?.entityId === entityId);
  let lastAddedNoteId = $state<string>();
  let scrollContainer = $state<HTMLElement | null>(null);
  let dragScroll: ReturnType<typeof createDragScroll> | null = null;
  let dragging = $state<{
    noteId: string;
    position: NoteListDragPosition | null;
    move: NoteListItemReorder['ondragmove'];
    end: NoteListItemReorder['ondragend'];
    cancel: NoteListItemReorder['ondragcancel'];
  } | null>(null);

  const replayDraggingPosition = (position: NoteListDragPosition) => {
    dragging?.move(position);
  };

  const updateDraggingPosition = (noteId: string, position: NoteListDragPosition) => {
    if (dragging?.noteId !== noteId) return;
    dragging.position = position;
    dragScroll?.updatePointer(position.clientX, position.clientY);
    replayDraggingPosition(position);
  };

  const stopDragScroll = () => {
    dragScroll?.destroy();
    dragScroll = null;
  };

  const startDragScroll = (noteId: string, initialPointer: { clientX: number; clientY: number }) => {
    if (!scrollContainer) return;
    stopDragScroll();
    dragScroll = createDragScroll(elementScrollViewport(scrollContainer), {
      initialPointer,
      onScroll: () => {
        if (dragging?.noteId === noteId && dragging.position) {
          replayDraggingPosition(dragging.position);
        }
      },
    });
  };

  const handleDragStart = (noteId: string, pointer: { clientX: number; clientY: number }, reorder: NoteListItemReorder): boolean => {
    if (!reorder.ondragstart()) return false;
    dragging = {
      noteId,
      position: null,
      move: reorder.ondragmove,
      end: reorder.ondragend,
      cancel: reorder.ondragcancel,
    };
    startDragScroll(noteId, pointer);
    return true;
  };

  const handleDragEnd = (noteId: string) => {
    const current = dragging;
    if (current?.noteId !== noteId) return;
    dragging = null;
    stopDragScroll();
    void current.end();
  };

  const handleDragCancel = (noteId: string) => {
    const current = dragging;
    if (current?.noteId !== noteId) return;
    dragging = null;
    stopDragScroll();
    current.cancel();
  };

  let activeIdentity = $state<string | null>(null);
  $effect(() => {
    const targetSiteId = siteId;
    const targetEntityId = entityId;
    const identity = `${targetSiteId}:${targetEntityId}`;
    if (activeIdentity !== null && activeIdentity !== identity) {
      if (dragging) handleDragCancel(dragging.noteId);
      activeCreateRequest = null;
      resolvedExpanded = false;
      lastAddedNoteId = undefined;
    }
    activeIdentity = identity;

    return noteActions.activate({
      siteId: targetSiteId,
      entityId: targetEntityId,
      onTerminal: (noteId) => {
        if (lastAddedNoteId === noteId) lastAddedNoteId = undefined;
      },
    });
  });

  const handleAddNote = async (via: string) => {
    if (createInFlight) return;

    const targetSiteId = siteId;
    const targetEntityId = entityId;
    const requestId = ++createRequestId;
    activeCreateRequest = { entityId: targetEntityId, id: requestId, siteId: targetSiteId };
    try {
      const outcome = await noteOperations.create(
        {
          content: '',
          color: 'gray',
          entityId: targetEntityId,
        },
        {
          analytics: {
            onSuccess: () => mixpanel.track('create_related_note', { via }),
          },
        },
      );
      if (activeCreateRequest?.id !== requestId || siteId !== targetSiteId || entityId !== targetEntityId) return;
      if (outcome.status === 'success') {
        lastAddedNoteId = outcome.value.id;
      } else if (outcome.status === 'failure') {
        Toast.error('노트를 만들지 못했어요.');
      }
    } finally {
      if (activeCreateRequest?.id === requestId) activeCreateRequest = null;
    }
  };

  const handleDeleteNote = async (noteId: string) => {
    const targetSiteId = siteId;
    const targetEntityId = entityId;
    const state = openListState.visibleNotes().some(({ note }) => note.id === noteId) ? openListState : resolvedListState;
    if (!targetSiteId || !targetEntityId) return;

    await noteActions.delete({
      noteId,
      siteId: targetSiteId,
      state,
      onSuccess: () => mixpanel.track('delete_related_note'),
    });
  };

  const handleToggleStatus = async (noteId: string) => {
    const note = notes.find((item) => item.id === noteId);
    const targetSiteId = siteId;
    const targetEntityId = entityId;
    if (!note || !targetSiteId || !targetEntityId) return;

    await noteActions.toggleStatus({
      note,
      states: { OPEN: openListState, RESOLVED: resolvedListState },
      siteId: targetSiteId,
      onSuccess: (status) => mixpanel.track('toggle_related_note_status', { status }),
    });
  };

  const handleStatusExitComplete = (noteId: string, sourceStatus: 'OPEN' | 'RESOLVED') => {
    noteActions.finishStatusTransfer(noteId, sourceStatus);
  };

  $effect(() => {
    const currentNotes = notes;
    const addedNoteId = lastAddedNoteId;
    if (!addedNoteId || currentNotes.every((note) => note.id !== addedNoteId)) return;
    lastAddedNoteId = undefined;
    void tick().then(() => {
      scrollContainer?.querySelector<HTMLTextAreaElement>(`[data-note-id="${addedNoteId}"] textarea`)?.focus();
    });
  });

  onDestroy(() => {
    if (dragging) handleDragCancel(dragging.noteId);
    else stopDragScroll();
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      flexShrink: '0',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '6px', fontWeight: 'semibold' })}>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>노트</div>
      {#if openVisibleCount > 0}
        <div
          class={css({
            fontSize: '11px',
            color: 'text.default',
            backgroundColor: 'surface.muted',
            paddingX: '6px',
            paddingY: '2px',
            borderRadius: '4px',
          })}
        >
          {openVisibleCount}
        </div>
      {/if}
    </div>

    <button
      class={center({
        size: '20px',
        color: 'text.faint',
        transition: 'common',
        _hover: { color: 'text.subtle' },
        cursor: 'pointer',
      })}
      onclick={() => handleAddNote('button')}
      type="button"
      use:tooltip={{ message: '노트 추가', placement: 'top' }}
    >
      <Icon icon={PlusIcon} size={14} />
    </button>
  </div>

  <div
    bind:this={scrollContainer}
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      flexGrow: '1',
      overflowY: 'auto',
      paddingX: '8px',
      paddingTop: '8px',
      paddingBottom: '20px',
    })}
  >
    <NoteList
      class={flex({ flexDirection: 'column' })}
      authoritativeNotes={openNotes}
      gap="6px"
      identity={{ siteId, entityId, status: 'OPEN' }}
      onMoveSuccess={() => mixpanel.track('move_related_note')}
      onexitcomplete={(note) => handleStatusExitComplete(note.id, 'OPEN')}
      state={openListState}
    >
      {#snippet children({ item, reorder })}
        <RelatedNoteItem
          anyDragging={dragging !== null}
          cancelling={noteActions.isCancelling(item.note.id)}
          dragging={reorder.dragging}
          note$key={item.note}
          onAddNote={() => handleAddNote('shortcut')}
          onColorSaved={(color) => mixpanel.track('change_related_note_color', { color })}
          onDelete={() => handleDeleteNote(item.note.id)}
          onDragCancel={() => handleDragCancel(item.note.id)}
          onDragEnd={() => handleDragEnd(item.note.id)}
          onDragMove={(position) => updateDraggingPosition(item.note.id, position)}
          onDragStart={(pointer) => handleDragStart(item.note.id, pointer, reorder)}
          onToggleStatus={() => handleToggleStatus(item.note.id)}
          reorderEnabled={reorder.enabled}
          resolving={noteActions.isResolving(item.note.id)}
          {siteId}
        />
      {/snippet}
    </NoteList>

    {#if openVisibleCount === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '20px',
          paddingY: '60px',
        })}
      >
        <div
          class={center({
            size: '64px',
            borderRadius: '16px',
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={StickyNoteIcon} size={28} />
        </div>
        <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
          떠오르는 생각이나 아이디어를
          <br />
          자유롭게 기록해보세요
        </p>
        <Button loading={createInFlight} onclick={() => handleAddNote('button')} size="sm" variant="secondary">노트 추가</Button>
      </div>
    {/if}

    <div
      style:display={resolvedVisibleCount > 0 ? 'flex' : 'none'}
      class={flex({
        flexDirection: 'column',
        gap: '6px',
        borderTopWidth: '1px',
        borderColor: 'surface.muted',
        paddingTop: '6px',
        marginTop: 'auto',
      })}
    >
      <button
        class={flex({
          alignItems: 'center',
          gap: '6px',
          paddingX: '12px',
          paddingY: '8px',
          fontSize: '12px',
          color: 'text.faint',
          cursor: 'pointer',
          borderRadius: '6px',
          transition: 'common',
          transitionProperty: '[color, background-color]',
          _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
        })}
        onclick={() => (resolvedExpanded = !resolvedExpanded)}
        type="button"
      >
        <Icon
          style={css.raw({ transition: 'common', transform: resolvedExpanded ? 'rotate(90deg)' : 'rotate(0deg)' })}
          icon={ChevronRightIcon}
          size={14}
        />
        완료됨
        <div
          class={css({
            fontSize: '11px',
            fontWeight: 'semibold',
            color: 'text.default',
            backgroundColor: 'surface.muted',
            paddingX: '6px',
            paddingY: '2px',
            borderRadius: '4px',
          })}
        >
          {resolvedVisibleCount}
        </div>
      </button>
      <div
        class={css({
          display: 'grid',
          gridTemplateRows: resolvedExpanded ? '1fr' : '0fr',
          transitionProperty: '[grid-template-rows]',
          transitionDuration: '150ms',
        })}
        aria-hidden={!resolvedExpanded}
        aria-label="완료된 노트"
        inert={!resolvedExpanded}
      >
        <div class={css({ minHeight: '0', overflow: 'hidden' })}>
          <NoteList
            class={flex({ flexDirection: 'column' })}
            authoritativeNotes={resolvedNotes}
            gap="6px"
            identity={{ siteId, entityId, status: 'RESOLVED' }}
            onMoveSuccess={() => mixpanel.track('move_related_note')}
            onexitcomplete={(note) => handleStatusExitComplete(note.id, 'RESOLVED')}
            state={resolvedListState}
          >
            {#snippet children({ item, reorder })}
              <RelatedNoteItem
                anyDragging={dragging !== null}
                cancelling={noteActions.isCancelling(item.note.id)}
                dragging={reorder.dragging}
                note$key={item.note}
                onAddNote={() => handleAddNote('shortcut')}
                onColorSaved={(color) => mixpanel.track('change_related_note_color', { color })}
                onDelete={() => handleDeleteNote(item.note.id)}
                onDragCancel={() => handleDragCancel(item.note.id)}
                onDragEnd={() => handleDragEnd(item.note.id)}
                onDragMove={(position) => updateDraggingPosition(item.note.id, position)}
                onDragStart={(pointer) => handleDragStart(item.note.id, pointer, reorder)}
                onToggleStatus={() => handleToggleStatus(item.note.id)}
                reorderEnabled={reorder.enabled}
                resolving={noteActions.isResolving(item.note.id)}
                {siteId}
              />
            {/snippet}
          </NoteList>
        </div>
      </div>
    </div>
  </div>
</div>
