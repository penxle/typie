<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { createDragScroll, elementScrollViewport, pushEscapeHandler } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, tick } from 'svelte';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CommandIcon from '~icons/lucide/command';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import { beforeNavigate } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { NoteActions } from '$lib/note/note-actions.svelte';
  import { NoteListState } from '$lib/note/note-list-state.svelte';
  import { getNoteOperationsContext } from '$lib/note/note-mutation';
  import { getNoteSyncContext } from '$lib/note/note-sync.svelte';
  import NoteColorPalette from '$lib/note/NoteColorPalette.svelte';
  import NoteList from '$lib/note/NoteList.svelte';
  import { graphql } from '$mearie';
  import NoteComponent from './Note.svelte';
  import NoteEntitySearchModal from './NoteEntitySearchModal.svelte';
  import type { NoteListDragPosition, NoteListItemReorder } from '$lib/note/NoteList.svelte';

  type NoteDragging = {
    noteId: string;
    position: NoteListDragPosition | null;
    move: NoteListItemReorder['ondragmove'];
    end: NoteListItemReorder['ondragend'];
    cancel: NoteListItemReorder['ondragcancel'];
  };

  const app = getAppContext();
  const noteOperations = getNoteOperationsContext();
  const noteSync = getNoteSyncContext();
  const currentSiteId = $derived(app.preference.current.currentSiteId ?? null);
  let requestedQuerySiteId: string | null = null;

  const siteQuery = createQuery(
    graphql(`
      query DashboardLayout_Notes_Site_Query($siteId: ID) {
        notes(siteId: $siteId) {
          id
          content
          createdAt
          updatedAt
          order
          color
          status
          site {
            id
          }
          entities {
            id
            slug

            ...EntityIcon_entity

            node {
              __typename

              ... on Document {
                id
                title
              }

              ... on Folder {
                id
                name
              }
            }
          }
        }
      }
    `),
    () => {
      requestedQuerySiteId = currentSiteId;
      return { siteId: currentSiteId };
    },
  );

  let successfulDataSiteId = $state<string | null>(null);
  $effect(() => {
    const siteId = currentSiteId;
    const data = siteQuery.data;
    const loading = siteQuery.loading;
    const error = siteQuery.error;

    // Mearie retains data across variable changes. It unsubscribes the previous operation before evaluating the next variables.
    if (requestedQuerySiteId === siteId && data !== undefined && !loading && !error) {
      successfulDataSiteId = requestedQuerySiteId;
    }
  });

  const hasCurrentSiteData = $derived(siteQuery.data !== undefined && successfulDataSiteId === currentSiteId);
  const notes = $derived(hasCurrentSiteData ? (siteQuery.data?.notes.filter((note) => note.site.id === currentSiteId) ?? []) : []);
  type SiteNote = (typeof notes)[number];

  let inputValue = $state('');
  let inputEl = $state<HTMLTextAreaElement>();
  let selectedColor = $state('gray');
  let createRequestId = 0;
  let activeCreateRequest = $state<{ id: number; siteId: string } | null>(null);
  const createInFlight = $derived(activeCreateRequest?.siteId === currentSiteId);
  let expandedNoteId = $state<string | null>(null);
  let entitySearchNoteId = $state<string | null>(null);
  let resolvedOpen = $state(false);
  const noteActions = new NoteActions<SiteNote>();

  let dragging = $state<NoteDragging | null>(null);
  let scrollContainer = $state<HTMLElement | null>(null);
  let composer = $state<HTMLElement | null>(null);
  let dragScroll: ReturnType<typeof createDragScroll> | null = null;

  const openListState = new NoteListState<SiteNote>({
    isTerminallyDeleted: (siteId, noteId) => noteSync.isTerminallyDeleted(siteId, noteId),
    isPendingDeletion: (noteId) => noteActions.isPendingDeletion(noteId),
  });
  const resolvedListState = new NoteListState<SiteNote>({
    isTerminallyDeleted: (siteId, noteId) => noteSync.isTerminallyDeleted(siteId, noteId),
    isPendingDeletion: (noteId) => noteActions.isPendingDeletion(noteId),
  });

  const openNotes = $derived(notes.filter((note) => note.status === 'OPEN' && noteActions.isStatusAdmitted(note.id, 'OPEN')));
  const resolvedNotes = $derived(notes.filter((note) => note.status === 'RESOLVED' && noteActions.isStatusAdmitted(note.id, 'RESOLVED')));
  const openVisibleCount = $derived(openListState.visibleNotes().length);
  const resolvedVisibleCount = $derived(resolvedListState.visibleNotes().length);

  $effect.pre(() => {
    noteActions.syncStatus({
      siteId: currentSiteId,
      notes,
      visibleStatuses: app.state.notesOpen ? (resolvedOpen ? ['OPEN', 'RESOLVED'] : ['OPEN']) : [],
    });
  });

  const entitySearchExistingIds = $derived.by(() => {
    if (!entitySearchNoteId) return [];
    const n = notes.find((n) => n.id === entitySearchNoteId);
    return n?.entities?.map((e) => e.id) ?? [];
  });

  const replayDraggingPosition = (position: NoteListDragPosition) => {
    const currentDragging = dragging;
    if (!currentDragging) return;
    currentDragging.move(position);
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

  const startDragScroll = (draggingNoteId: string, initialPointer: { clientX: number; clientY: number }) => {
    stopDragScroll();
    if (!scrollContainer || !composer) return;

    const baseViewport = elementScrollViewport(scrollContainer);
    dragScroll = createDragScroll(
      {
        ...baseViewport,
        getRect: () => {
          const rect = baseViewport.getRect();
          const bottom = Math.min(rect.bottom, window.innerHeight);
          return {
            ...rect,
            top: Math.min(Math.max(rect.top, composer?.getBoundingClientRect().bottom ?? rect.top), bottom),
            bottom,
          };
        },
      },
      {
        initialPointer,
        stickyCandidates: [],
        onScroll: () => {
          if (dragging?.noteId === draggingNoteId && dragging.position) {
            replayDraggingPosition(dragging.position);
          }
        },
      },
    );
  };

  const handleDragEnd = (noteId: string) => {
    const currentDragging = dragging;
    if (currentDragging?.noteId !== noteId) return;
    dragging = null;
    stopDragScroll();
    void currentDragging.end();
  };

  const handleDragStart = (noteId: string, initialPointer: { clientX: number; clientY: number }, reorder: NoteListItemReorder): boolean => {
    if (!reorder.ondragstart()) return false;

    dragging = {
      noteId,
      position: null,
      move: reorder.ondragmove,
      end: reorder.ondragend,
      cancel: reorder.ondragcancel,
    };
    startDragScroll(noteId, initialPointer);
    return true;
  };

  const handleDragCancel = (noteId: string) => {
    const currentDragging = dragging;
    if (currentDragging?.noteId !== noteId) return;

    dragging = null;
    stopDragScroll();
    currentDragging.cancel();
  };

  let activeSiteId = $state<string | null>(null);
  let siteInitialized = $state(false);
  $effect(() => {
    const siteId = currentSiteId;
    if (siteInitialized && activeSiteId !== siteId) {
      if (dragging) handleDragCancel(dragging.noteId);
      activeCreateRequest = null;
      if (siteId === null) {
        openListState.reset();
        resolvedListState.reset();
      }
      expandedNoteId = null;
      entitySearchNoteId = null;
      resolvedOpen = false;
    }
    siteInitialized = true;
    activeSiteId = siteId;
    if (!siteId) return;

    return noteActions.activate({
      siteId,
      onTerminal: (noteId) => {
        if (expandedNoteId === noteId) expandedNoteId = null;
        if (entitySearchNoteId === noteId) entitySearchNoteId = null;
      },
    });
  });

  const handleExpand = (noteId: string) => {
    expandedNoteId = noteId;
  };

  const handleCollapse = (options: { focusComposer?: boolean } = {}) => {
    expandedNoteId = null;
    if (options.focusComposer) {
      void tick().then(() => {
        if (expandedNoteId === null && app.state.notesOpen) {
          inputEl?.focus();
        }
      });
    }
  };

  $effect(() => {
    if (expandedNoteId) {
      return pushEscapeHandler(() => {
        handleCollapse();
        return true;
      });
    }
  });

  const handleKeyDown = (event: KeyboardEvent) => {
    const metaOrCtrlKeyOnly = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey);

    if (metaOrCtrlKeyOnly && event.key === 'j') {
      event.preventDefault();
      app.state.notesOpen = !app.state.notesOpen;
    }
  };

  const handleAddNote = async (via: string) => {
    if (createInFlight) return;
    if (!inputValue.trim()) return;

    const siteId = app.preference.current.currentSiteId;
    if (!siteId) return;

    const submittedContent = inputValue;
    const submittedColor = selectedColor;
    const requestId = ++createRequestId;
    activeCreateRequest = { id: requestId, siteId };
    try {
      const outcome = await noteOperations.create(
        {
          siteId,
          color: submittedColor,
          content: submittedContent,
        },
        {
          analytics: {
            onSuccess: () => mixpanel.track('create_note', { via }),
          },
        },
      );

      if (activeCreateRequest?.id !== requestId || app.preference.current.currentSiteId !== siteId) return;
      if (outcome.status === 'success' && inputValue === submittedContent && selectedColor === submittedColor) {
        inputValue = '';
        selectedColor = 'gray';
      }
      if (outcome.status === 'success') {
        inputEl?.focus();
      } else if (outcome.status === 'failure') {
        Toast.error('노트를 만들지 못했어요.');
      }
    } finally {
      if (activeCreateRequest?.id === requestId) activeCreateRequest = null;
    }
  };

  const handleDeleteNote = async (noteId: string) => {
    const siteId = currentSiteId;
    if (!siteId) return;

    const state = openListState.visibleNotes().some(({ note }) => note.id === noteId) ? openListState : resolvedListState;
    await noteActions.delete({
      noteId,
      siteId,
      state,
      onSuccess: () => mixpanel.track('delete_note'),
    });
  };

  const handleToggleStatus = async (noteId: string) => {
    const note = notes.find((n) => n.id === noteId);
    if (!note) return;

    const siteId = app.preference.current.currentSiteId;
    if (!siteId) return;

    await noteActions.toggleStatus({
      note,
      states: { OPEN: openListState, RESOLVED: resolvedListState },
      siteId,
      onSuccess: (status) => mixpanel.track('toggle_note_status', { status }),
    });
  };

  const handleStatusExitComplete = (noteId: string, sourceStatus: 'OPEN' | 'RESOLVED') => {
    if (!noteActions.finishStatusTransfer(noteId, sourceStatus)) return;
    if (expandedNoteId === noteId) expandedNoteId = null;
  };

  const handleAddEntity = (noteId: string) => {
    entitySearchNoteId = noteId;
  };

  const handleRemoveEntity = async (noteId: string, entityId: string) => {
    const siteId = app.preference.current.currentSiteId;
    if (!siteId) return;

    const outcome = await noteOperations.removeEntity(
      { noteId, entityId },
      {
        lastKnown: { siteId, noteId },
      },
    );
    if (currentSiteId !== siteId || notes.every((note) => note.id !== noteId)) return;
    if (outcome.status === 'failure') {
      Toast.error('연결을 해제하지 못했어요.');
    }
  };

  const close = () => {
    app.state.notesOpen = false;
  };

  beforeNavigate(() => {
    close();
  });

  $effect(() => {
    if (!app.state.notesOpen) {
      return;
    }

    cache.invalidate({ __typename: 'Query', $field: 'notes' });

    if (inputEl) {
      inputEl.focus();
    }
  });

  onDestroy(() => {
    if (dragging) handleDragCancel(dragging.noteId);
    else stopDragScroll();
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<Modal
  style={css.raw({
    backgroundColor: 'transparent',
    maxWidth: 'full',
    height: 'full',
    border: 'none',
    boxShadow: '[none]',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '0',
  })}
  onclose={close}
  open={app.state.notesOpen}
  overlayPadding={0}
>
  <div
    bind:this={scrollContainer}
    class={flex({
      position: 'relative',
      paddingTop: '[15dvh]',
      flexDirection: 'column',
      gap: '20px',
      width: 'full',
      height: 'full',
      overflowY: 'auto',
      scrollbarGutter: 'stable',
      alignItems: 'center',
    })}
    onclick={(e) => {
      const target = e.target as HTMLElement;
      if (expandedNoteId && !target.closest(`[data-note-id="${expandedNoteId}"]`)) {
        handleCollapse();
        return;
      }
      if (target.closest('[data-notes-backdrop]')) {
        close();
      }
    }}
    role="presentation"
  >
    <div
      class={css({
        position: 'absolute',
        inset: '0',
      })}
      data-notes-backdrop
      role="none"
    ></div>

    <!-- Input Area -->
    <div
      bind:this={composer}
      class={flex({
        position: 'sticky',
        top: '[calc(16px - 15dvh)]',
        zIndex: '2',
        flexDirection: 'column',
        width: 'full',
        maxWidth: '560px',
        flexShrink: '0',
        backgroundColor: 'surface.default',
        borderRadius: '14px',
        overflow: 'hidden',
        boxShadow: 'large',
      })}
    >
      <textarea
        bind:this={inputEl}
        class={css({
          width: 'full',
          minHeight: '120px',
          padding: '16px',
          fontSize: '16px',
          fontWeight: 'medium',
          color: 'text.default',
          borderRadius: '8px',
          resize: 'none',
        })}
        onkeydown={(e) => {
          if (e.key !== 'Enter' || !(e.metaKey || e.ctrlKey)) return;

          e.stopPropagation();
          if (e.isComposing) {
            setTimeout(() => void handleAddNote('shortcut'), 0);
            return;
          }

          e.preventDefault();
          void handleAddNote('shortcut');
        }}
        placeholder="떠오르는 생각을 적어보세요"
        bind:value={inputValue}></textarea>

      <div class={flex({ alignItems: 'center', gap: '8px', paddingX: '12px', paddingY: '6px' })}>
        <!-- Color dots -->
        <NoteColorPalette onchange={(color) => (selectedColor = color)} {selectedColor} size="12px" />

        <Button
          style={css.raw({ marginLeft: 'auto', gap: '4px' })}
          loading={createInFlight}
          onclick={() => handleAddNote('button')}
          size="sm"
          variant="primary"
        >
          추가
          <div class={flex({ alignItems: 'center', opacity: '70' })}>
            {#if navigator.platform.includes('Mac')}
              <Icon icon={CommandIcon} size={12} />
            {:else}
              <span class={css({ fontSize: '12px' })}>Ctrl+</span>
            {/if}
            <Icon icon={CornerDownLeftIcon} size={12} />
          </div>
        </Button>
      </div>
    </div>

    <!-- Notes List -->
    <div
      class={css({
        paddingBottom: '50px',
        maxWidth: '480px',
        flexGrow: '1',
        width: 'full',
      })}
    >
      {#if !hasCurrentSiteData && !siteQuery.error}
        <p
          class={css({
            paddingY: '32px',
            textAlign: 'center',
            fontSize: '14px',
            color: 'text.faint',
          })}
          role="status"
        >
          노트를 불러오는 중...
        </p>
      {:else if !hasCurrentSiteData && siteQuery.error}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            gap: '12px',
            paddingY: '32px',
            color: 'text.faint',
            fontSize: '14px',
          })}
        >
          <span>노트를 불러오지 못했어요.</span>
          <Button onclick={() => siteQuery.refetch()} size="sm" variant="secondary">다시 시도</Button>
        </div>
      {:else}
        {#if currentSiteId}
          <NoteList
            class={flex({ flexDirection: 'column' })}
            authoritativeNotes={openNotes}
            gap="8px"
            identity={{ siteId: currentSiteId, status: 'OPEN' }}
            onMoveSuccess={() => mixpanel.track('move_note')}
            onexitcomplete={(note) => handleStatusExitComplete(note.id, 'OPEN')}
            presentationActive={app.state.notesOpen}
            state={openListState}
          >
            {#snippet children({ item, reorder })}
              <NoteComponent
                anyDragging={dragging !== null}
                cancelling={noteActions.isCancelling(item.note.id)}
                dragging={reorder.dragging}
                expanded={expandedNoteId === item.note.id}
                note={item.note}
                onColorSaved={(color) => mixpanel.track('change_note_color', { color })}
                onaddentity={handleAddEntity}
                oncollapse={handleCollapse}
                ondelete={handleDeleteNote}
                ondragcancel={() => handleDragCancel(item.note.id)}
                ondragend={() => handleDragEnd(item.note.id)}
                ondragmove={(position) => updateDraggingPosition(item.note.id, position)}
                ondragstart={(pointer) => handleDragStart(item.note.id, pointer, reorder)}
                onexpand={handleExpand}
                onremoveentity={handleRemoveEntity}
                ontogglestatus={handleToggleStatus}
                reorderEnabled={reorder.enabled}
                resolving={noteActions.isResolving(item.note.id)}
              />
            {/snippet}
          </NoteList>
        {/if}

        {#if resolvedVisibleCount > 0}
          <button
            class={flex({
              position: 'relative',
              zIndex: '1',
              alignItems: 'center',
              gap: '6px',
              marginTop: '16px',
              paddingX: '8px',
              paddingY: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.subtle',
              cursor: 'pointer',
              borderRadius: '6px',
              transitionProperty: 'common!',
              backgroundColor: 'surface.dark/10',
              _hover: { color: 'text.default', backgroundColor: 'surface.dark/15' },
            })}
            onclick={() => {
              handleCollapse();
              resolvedOpen = !resolvedOpen;
            }}
            type="button"
          >
            <Icon icon={resolvedOpen ? ChevronDownIcon : ChevronRightIcon} size={14} />
            완료됨 ({resolvedVisibleCount})
          </button>
        {/if}

        {#if currentSiteId}
          <div
            class={css({
              display: 'grid',
              gridTemplateRows: resolvedOpen ? '1fr' : '0fr',
              transitionProperty: '[grid-template-rows]',
              transitionDuration: '150ms',
            })}
            aria-hidden={!resolvedOpen}
            aria-label="완료된 노트"
            inert={!resolvedOpen}
          >
            <div class={css({ minHeight: '0', overflow: 'hidden' })}>
              <NoteList
                class={flex({ flexDirection: 'column', marginTop: '4px' })}
                authoritativeNotes={resolvedNotes}
                gap="8px"
                identity={{ siteId: currentSiteId, status: 'RESOLVED' }}
                onMoveSuccess={() => mixpanel.track('move_note')}
                onexitcomplete={(note) => handleStatusExitComplete(note.id, 'RESOLVED')}
                presentationActive={app.state.notesOpen && resolvedOpen}
                state={resolvedListState}
              >
                {#snippet children({ item, reorder })}
                  <NoteComponent
                    anyDragging={dragging !== null}
                    cancelling={noteActions.isCancelling(item.note.id)}
                    dragging={reorder.dragging}
                    expanded={expandedNoteId === item.note.id}
                    note={item.note}
                    onColorSaved={(color) => mixpanel.track('change_note_color', { color })}
                    onaddentity={handleAddEntity}
                    oncollapse={handleCollapse}
                    ondelete={handleDeleteNote}
                    ondragcancel={() => handleDragCancel(item.note.id)}
                    ondragend={() => handleDragEnd(item.note.id)}
                    ondragmove={(position) => updateDraggingPosition(item.note.id, position)}
                    ondragstart={(pointer) => handleDragStart(item.note.id, pointer, reorder)}
                    onexpand={handleExpand}
                    onremoveentity={handleRemoveEntity}
                    ontogglestatus={handleToggleStatus}
                    reorderEnabled={reorder.enabled}
                    resolving={noteActions.isResolving(item.note.id)}
                  />
                {/snippet}
              </NoteList>
            </div>
          </div>
        {/if}

        {#if openVisibleCount + resolvedVisibleCount === 0}
          <p
            class={css({
              paddingY: '32px',
              textAlign: 'center',
              fontSize: '14px',
              color: 'text.faint',
            })}
          >
            떠오르는 생각이나 아이디어를 자유롭게 기록해보세요
          </p>
        {/if}
      {/if}
    </div>
  </div>
</Modal>

<NoteEntitySearchModal
  existingEntityIds={entitySearchExistingIds}
  noteId={entitySearchNoteId ?? ''}
  onclose={() => (entitySearchNoteId = null)}
  open={entitySearchNoteId !== null}
/>
