<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { createDragScroll, elementScrollViewport } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, tick } from 'svelte';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import ExpandIcon from '~icons/lucide/expand';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { NoteActions } from '$lib/note/note-actions.svelte';
  import { NoteListState } from '$lib/note/note-list-state.svelte';
  import { getNoteOperationsContext } from '$lib/note/note-mutation';
  import { getNoteSyncContext } from '$lib/note/note-sync.svelte';
  import NoteList from '$lib/note/NoteList.svelte';
  import RelatedNoteItem from '$lib/note/RelatedNoteItem.svelte';
  import { graphql } from '$mearie';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';
  import type { NoteListDragPosition, NoteListItemReorder } from '$lib/note/NoteList.svelte';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const noteOperations = getNoteOperationsContext();
  const noteSync = getNoteSyncContext();
  const widgetContext = getWidgetContext();
  const { palette, document$key } = $derived(widgetContext.env);
  const hasActiveDocument = $derived(document$key != null);
  const relatedDocument = createFragment(
    graphql(`
      fragment Editor_Widget_DocumentRelatedNoteWidget_document on Document {
        id
        entity {
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
      }
    `),
    () => document$key,
  );

  const currentDocument = $derived(hasActiveDocument ? relatedDocument.data : undefined);
  const siteId = $derived(currentDocument?.entity.site.id ?? null);
  const entityId = $derived(currentDocument?.entity.id ?? null);
  const notes = $derived(currentDocument?.entity.notes ?? []);
  type RelatedNote = (typeof notes)[number];

  const noteActions = new NoteActions<RelatedNote>();
  const listState = new NoteListState<RelatedNote>({
    isTerminallyDeleted: (targetSiteId, noteId) => noteSync.isTerminallyDeleted(targetSiteId, noteId),
    isPendingDeletion: (noteId) => noteActions.isPendingDeletion(noteId),
  });
  const openNotes = $derived(notes.filter((note) => note.status === 'OPEN' && noteActions.isStatusAdmitted(note.id, 'OPEN')));
  const visibleCount = $derived(listState.visibleNotes().length);

  let isExpanded = $state((data.isExpanded as boolean) ?? false);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  $effect.pre(() => {
    noteActions.syncStatus({
      siteId,
      entityId: entityId ?? undefined,
      notes,
      visibleStatuses: siteId && entityId && !isCollapsed ? ['OPEN'] : [],
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

  const toggleExpanded = () => {
    isExpanded = !isExpanded;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded, isCollapsed });
  };

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded, isCollapsed });
  };

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
    if (!scrollContainer || palette) return;
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
    const identity = targetSiteId && targetEntityId ? `${targetSiteId}:${targetEntityId}` : null;
    if (activeIdentity !== null && activeIdentity !== identity) {
      if (dragging) handleDragCancel(dragging.noteId);
      activeCreateRequest = null;
      lastAddedNoteId = undefined;
      if (identity === null) listState.reset();
    }
    activeIdentity = identity;
    if (!targetSiteId || !targetEntityId) return;

    return noteActions.activate({
      siteId: targetSiteId,
      entityId: targetEntityId,
      onTerminal: (noteId) => {
        if (lastAddedNoteId === noteId) lastAddedNoteId = undefined;
      },
    });
  });

  const handleAddNote = async (via: string) => {
    const targetSiteId = siteId;
    const targetEntityId = entityId;
    if (createInFlight || !targetSiteId || !targetEntityId) return;

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
      if (siteId !== targetSiteId || entityId !== targetEntityId || activeCreateRequest?.id !== requestId) return;
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
    if (!targetSiteId || !targetEntityId) return;

    await noteActions.delete({
      noteId,
      siteId: targetSiteId,
      state: listState,
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
      states: { OPEN: listState },
      siteId: targetSiteId,
      onSuccess: (status) => mixpanel.track('toggle_widget_note_status', { status }),
    });
  };

  const handleEndResolve = (noteId: string) => {
    noteActions.finishStatusTransfer(noteId, 'OPEN');
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

<Widget collapsed={isCollapsed} icon={StickyNoteIcon} noPadding title="노트">
  {#snippet headerActions()}
    {#if !palette && !isCollapsed}
      {#if hasActiveDocument}
        <button
          class={center({
            height: '26px',
            borderRadius: '6px',
            paddingX: '6px',
            color: 'text.muted',
            transition: 'common',
            _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
            cursor: 'pointer',
          })}
          onclick={(event) => {
            event.stopPropagation();
            void handleAddNote('button');
          }}
          onpointerdown={(event) => event.stopPropagation()}
          type="button"
          use:tooltip={{ message: '노트 추가', placement: 'top' }}
        >
          <Icon icon={PlusIcon} size={14} />
        </button>
      {/if}
      <button
        class={center({
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          color: 'text.muted',
          transition: 'common',
          _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
          cursor: 'pointer',
        })}
        onclick={(event) => {
          event.stopPropagation();
          toggleExpanded();
        }}
        onpointerdown={(event) => event.stopPropagation()}
        type="button"
        use:tooltip={{ message: isExpanded ? '크기 제한' : '크기 제한 해제', placement: 'top' }}
      >
        <Icon icon={isExpanded ? Minimize2Icon : ExpandIcon} size={14} />
      </button>
    {/if}
    <button
      class={cx(
        'group',
        flex({
          alignItems: 'center',
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          gap: '2px',
          color: 'text.muted',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
        }),
      )}
      onclick={toggleCollapse}
      type="button"
    >
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div
    bind:this={scrollContainer}
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      maxHeight: isExpanded ? undefined : '400px',
      overflowY: 'auto',
      padding: '8px',
      paddingRight: '4px',
    })}
  >
    {#if !hasActiveDocument}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          paddingY: '24px',
        })}
      >
        <div
          class={center({
            size: '48px',
            borderRadius: '12px',
            backgroundColor: 'surface.canvas',
            color: 'text.hint',
          })}
        >
          <Icon icon={StickyNoteIcon} size={20} />
        </div>
        <p class={css({ fontSize: '12px', color: 'text.hint', textAlign: 'center' })}>문서를 열면 연결된 노트를 볼 수 있어요</p>
      </div>
    {:else if siteId && entityId}
      <NoteList
        class={flex({ flexDirection: 'column' })}
        authoritativeNotes={openNotes}
        gap="6px"
        identity={{ siteId, entityId, status: 'OPEN' }}
        onMoveSuccess={() => mixpanel.track('move_related_note')}
        onexitcomplete={(note) => handleEndResolve(note.id)}
        reorderEnabled={!palette}
        state={listState}
      >
        {#snippet children({ item, reorder })}
          <RelatedNoteItem
            anyDragging={dragging !== null}
            cancelling={noteActions.isCancelling(item.note.id)}
            compact
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
            {palette}
            reorderEnabled={reorder.enabled}
            resolving={noteActions.isResolving(item.note.id)}
            {siteId}
          />
        {/snippet}
      </NoteList>

      {#if visibleCount === 0}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '12px',
            paddingY: '24px',
          })}
        >
          <div
            class={center({
              size: '48px',
              borderRadius: '12px',
              backgroundColor: 'surface.canvas',
              color: 'text.hint',
            })}
          >
            <Icon icon={StickyNoteIcon} size={20} />
          </div>
          <p class={css({ fontSize: '12px', color: 'text.hint', textAlign: 'center' })}>
            떠오르는 생각이나 아이디어를
            <br />
            자유롭게 기록해보세요
          </p>
        </div>
      {/if}
    {/if}
  </div>
</Widget>
