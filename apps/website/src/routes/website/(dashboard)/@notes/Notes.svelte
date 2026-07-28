<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { animateFlip, createDragScroll, elementScrollViewport, pushEscapeHandler } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, tick } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CommandIcon from '~icons/lucide/command';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import { beforeNavigate } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { reorderedNoteIdsForDrag } from '$lib/note-reorder';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { noteColors } from './colors';
  import NoteComponent from './Note.svelte';
  import NoteEntitySearchModal from './NoteEntitySearchModal.svelte';
  import type { NoteReorderDirection, NoteReorderGeometry } from '$lib/note-reorder';

  type NoteDragPosition = {
    clientX: number;
    clientY: number;
    direction: NoteReorderDirection;
    ghost: NoteReorderGeometry;
  };

  type NoteDragging = {
    noteId: string;
    originalOrder: string[];
    position: NoteDragPosition | null;
    sectionNoteIds: string[];
  };

  const [createNote] = createMutation(
    graphql(`
      mutation DashboardLayout_Notes_CreateNote_Mutation($input: CreateNoteInput!) {
        createNote(input: $input) {
          id
          content
          createdAt
          color
          status
          entities {
            id
            slug

            ...EntityIcon_entity

            node {
              __typename
            }
          }
        }
      }
    `),
  );

  const [updateNote] = createMutation(
    graphql(`
      mutation DashboardLayout_Notes_UpdateNote_Mutation($input: UpdateNoteInput!) {
        updateNote(input: $input) {
          id
          content
          updatedAt
          status
          entities {
            id
            slug

            ...EntityIcon_entity

            node {
              __typename
            }
          }
        }
      }
    `),
  );

  const [deleteNote] = createMutation(
    graphql(`
      mutation DashboardLayout_Notes_DeleteNote_Mutation($input: DeleteNoteInput!) {
        deleteNote(input: $input) {
          id
        }
      }
    `),
  );

  const [moveNote] = createMutation(
    graphql(`
      mutation DashboardLayout_Notes_MoveNote_Mutation($input: MoveNoteInput!) {
        moveNote(input: $input) {
          id
          order
        }
      }
    `),
  );

  const [removeNoteEntity] = createMutation(
    graphql(`
      mutation DashboardLayout_Notes_RemoveNoteEntity_Mutation($input: RemoveNoteEntityInput!) {
        removeNoteEntity(input: $input) {
          id
          entities {
            id
            slug

            ...EntityIcon_entity

            node {
              __typename
            }
          }
        }
      }
    `),
  );

  const app = getAppContext();

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
    () => ({ siteId: app.preference.current.currentSiteId }),
  );

  const notes = $derived(siteQuery.data?.notes ?? []);

  let inputValue = $state('');
  let inputEl = $state<HTMLTextAreaElement>();
  let selectedColor = $state('gray');
  let createInFlight = $state(false);
  let expandedNoteId = $state<string | null>(null);
  let entitySearchNoteId = $state<string | null>(null);
  let resolvedOpen = $state(false);
  const resolvingNoteIds = new SvelteSet<string>();

  let dragging = $state<NoteDragging | null>(null);
  let localNoteOrder = $state<string[]>([]);
  let scrollContainer = $state<HTMLElement | null>(null);
  let composer = $state<HTMLElement | null>(null);
  let dragScroll: ReturnType<typeof createDragScroll> | null = null;

  const sortedNotes = $derived.by(() => {
    if (localNoteOrder.length === 0) return notes;
    return [...notes].toSorted((a, b) => {
      const indexA = localNoteOrder.indexOf(a.id);
      const indexB = localNoteOrder.indexOf(b.id);
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  });

  const openNotes = $derived(sortedNotes.filter((n) => n.status === 'OPEN' || resolvingNoteIds.has(n.id)));
  const resolvedNotes = $derived(sortedNotes.filter((n) => n.status === 'RESOLVED' && !resolvingNoteIds.has(n.id)));

  const entitySearchExistingIds = $derived.by(() => {
    if (!entitySearchNoteId) return [];
    const n = notes.find((n) => n.id === entitySearchNoteId);
    return n?.entities?.map((e) => e.id) ?? [];
  });

  const resolveDraggingPosition = (position: NoteDragPosition) => {
    const currentDragging = dragging;
    if (!currentDragging || !scrollContainer) return;

    const sectionNoteIds = new Set(currentDragging.sectionNoteIds);
    const currentSectionOrder = localNoteOrder.filter((noteId) => sectionNoteIds.has(noteId));
    const noteGeometries = new SvelteMap<string, NoteReorderGeometry>();

    for (const noteElement of scrollContainer.querySelectorAll<HTMLElement>('[data-note-id]')) {
      const noteId = noteElement.dataset.noteId;
      if (!noteId || !sectionNoteIds.has(noteId)) continue;

      const { top, bottom } = noteElement.getBoundingClientRect();
      noteGeometries.set(noteId, { top, bottom });
    }
    noteGeometries.set(currentDragging.noteId, position.ghost);

    const reorderedSection = reorderedNoteIdsForDrag(currentSectionOrder, currentDragging.noteId, position.direction, noteGeometries);
    if (!reorderedSection || reorderedSection.every((noteId, index) => noteId === currentSectionOrder[index])) return;

    let sectionIndex = 0;
    const reorderedNotes = localNoteOrder.map((noteId) => {
      if (!sectionNoteIds.has(noteId)) return noteId;
      return reorderedSection[sectionIndex++] ?? noteId;
    });
    if (reorderedNotes.some((noteId, index) => noteId !== localNoteOrder[index])) {
      localNoteOrder = reorderedNotes;
    }
  };

  const updateDraggingPosition = (position: NoteDragPosition) => {
    if (!dragging) return;

    dragging.position = position;
    dragScroll?.updatePointer(position.clientX, position.clientY);
    resolveDraggingPosition(position);
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
            resolveDraggingPosition(dragging.position);
          }
        },
      },
    );
  };

  const handleDragEnd = async () => {
    const currentDragging = dragging;
    if (!currentDragging) return;

    const currentIndex = localNoteOrder.indexOf(currentDragging.noteId);
    const originalIndex = currentDragging.originalOrder.indexOf(currentDragging.noteId);
    dragging = null;
    stopDragScroll();

    if (currentIndex !== -1 && originalIndex !== -1 && currentIndex !== originalIndex && sortedNotes.length > 1) {
      if (!SubscribeModal.gate('notes_move')) {
        localNoteOrder = [...currentDragging.originalOrder];
        return;
      }

      const lowerNote = sortedNotes[currentIndex - 1] ?? null;
      const upperNote = sortedNotes[currentIndex + 1] ?? null;

      try {
        const { noteId } = currentDragging;
        await moveNote({
          input: {
            noteId,
            lowerOrder: lowerNote?.order,
            upperOrder: upperNote?.order,
          },
        });
        mixpanel.track('move_note');
        cache.invalidate({ __typename: 'Query', $field: 'notes' });

        const movedNote = sortedNotes.find((n) => n.id === noteId);
        for (const entity of movedNote?.entities ?? []) {
          cache.invalidate({ __typename: 'Entity', id: entity.id, $field: 'notes' });
        }
      } catch {
        localNoteOrder = [...currentDragging.originalOrder];
        Toast.error('노트 순서 변경에 실패했습니다. 잠시 후 다시 시도해주세요.');
      }
    }
  };

  let prevNoteIds = $state<string[]>([]);
  $effect(() => {
    const noteIds = notes.map((note) => note.id);
    if (dragging) return;

    const noteIdsStr = noteIds.join(',');
    const prevNoteIdsStr = prevNoteIds.join(',');

    if (noteIdsStr !== prevNoteIdsStr) {
      prevNoteIds = noteIds;
      localNoteOrder = noteIds;
    }
  });

  const handleDragStart = (noteId: string, initialPointer: { clientX: number; clientY: number }) => {
    const sectionNotes = openNotes.some((note) => note.id === noteId) ? openNotes : resolvedNotes;
    const originalOrder = sortedNotes.map((note) => note.id);
    localNoteOrder = [...originalOrder];
    const currentDragging: NoteDragging = {
      noteId,
      originalOrder,
      position: null,
      sectionNoteIds: sectionNotes.map((note) => note.id),
    };
    dragging = currentDragging;
    startDragScroll(noteId, initialPointer);
    return true;
  };

  const handleDragCancel = (noteId: string) => {
    const currentDragging = dragging;
    if (currentDragging?.noteId !== noteId) return;

    dragging = null;
    stopDragScroll();
    localNoteOrder = [...currentDragging.originalOrder];
  };

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

    if (!SubscribeModal.gate('notes_create')) {
      return;
    }

    const submittedContent = inputValue;
    const submittedColor = selectedColor;
    createInFlight = true;
    try {
      await createNote({
        input: {
          siteId: app.preference.current.currentSiteId,
          color: submittedColor,
          content: submittedContent,
        },
      });
      mixpanel.track('create_note', { via });
      cache.invalidate({ __typename: 'Query', $field: 'notes' });

      if (inputValue === submittedContent && selectedColor === submittedColor) {
        inputValue = '';
        selectedColor = 'gray';
      }
      inputEl?.focus();
    } finally {
      createInFlight = false;
    }
  };

  const handleDeleteNote = async (noteId: string) => {
    await deleteNote({ input: { noteId } });
    mixpanel.track('delete_note');
    cache.invalidate({ __typename: 'Query', $field: 'notes' });

    const note = notes.find((n) => n.id === noteId);
    for (const entity of note?.entities ?? []) {
      cache.invalidate({ __typename: 'Entity', id: entity.id, $field: 'notes' });
    }
  };

  const cancellingNoteIds = new SvelteSet<string>();

  const handleToggleStatus = async (noteId: string) => {
    const note = notes.find((n) => n.id === noteId);
    if (!note) return;

    if (cancellingNoteIds.has(noteId)) return;

    if (!SubscribeModal.gate('notes_update')) {
      return;
    }

    // Cancel resolving animation
    if (resolvingNoteIds.has(noteId)) {
      cancellingNoteIds.add(noteId);
      try {
        await updateNote({ input: { noteId, status: 'OPEN' } });
        mixpanel.track('toggle_note_status', { status: 'OPEN' });
        cache.invalidate({ __typename: 'Query', $field: 'notes' });
      } catch {
        cancellingNoteIds.delete(noteId);
        return;
      }
      cancellingNoteIds.delete(noteId);
      resolvingNoteIds.delete(noteId);
      return;
    }

    const newStatus = note.status === 'OPEN' ? 'RESOLVED' : 'OPEN';

    if (newStatus === 'RESOLVED') {
      resolvingNoteIds.add(noteId);
    }

    try {
      await updateNote({ input: { noteId, status: newStatus } });
      mixpanel.track('toggle_note_status', { status: newStatus });
      if (newStatus === 'OPEN') {
        cache.invalidate({ __typename: 'Query', $field: 'notes' });
        for (const entity of note.entities ?? []) {
          cache.invalidate({ __typename: 'Entity', id: entity.id, $field: 'notes' });
        }
      }
    } catch {
      resolvingNoteIds.delete(noteId);
    }
  };

  const handleEndResolve = (noteId: string) => {
    const note = notes.find((n) => n.id === noteId);
    resolvingNoteIds.delete(noteId);
    cache.invalidate({ __typename: 'Query', $field: 'notes' });
    for (const entity of note?.entities ?? []) {
      cache.invalidate({ __typename: 'Entity', id: entity.id, $field: 'notes' });
    }
  };

  const handleUpdateContent = async (noteId: string, content: string) => {
    if (!SubscribeModal.gate('notes_update')) {
      return;
    }

    await updateNote({ input: { noteId, content } });
    cache.invalidate({ __typename: 'Query', $field: 'notes' });
  };

  const handleChangeColor = async (noteId: string, color: string) => {
    if (!SubscribeModal.gate('notes_update')) {
      return;
    }

    await updateNote({ input: { noteId, color } });
    cache.invalidate({ __typename: 'Query', $field: 'notes' });
    mixpanel.track('change_note_color', { color });
  };

  const handleAddEntity = (noteId: string) => {
    entitySearchNoteId = noteId;
  };

  const handleRemoveEntity = async (noteId: string, entityId: string) => {
    if (!SubscribeModal.gate('notes_remove_entity')) {
      return;
    }

    await removeNoteEntity({ input: { noteId, entityId } });
    cache.invalidate({ __typename: 'Query', $field: 'notes' });
    cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'notes' });
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
    stopDragScroll();
  });

  $effect.pre(() => {
    void localNoteOrder;
    animateFlip('[data-note-id]', 'noteId');
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
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          {#each noteColors as c (c.value)}
            <button
              style:background-color={selectedColor === c.value ? c.color : 'transparent'}
              style:border={selectedColor === c.value ? 'none' : `1.5px solid ${c.color}`}
              class={center({
                width: '12px',
                height: '12px',
                borderRadius: 'full',
                cursor: 'pointer',
                padding: '0',
              })}
              aria-label={c.label}
              onclick={() => (selectedColor = c.value)}
              type="button"
              use:tooltip={{ message: c.label, placement: 'top' }}
            ></button>
          {/each}
        </div>

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
      {#if openNotes.length > 0}
        <div class={flex({ flexDirection: 'column', gap: '8px' })} role="list">
          {#each openNotes as note (note.id)}
            <NoteComponent
              cancelling={cancellingNoteIds.has(note.id)}
              draggingNoteId={dragging?.noteId ?? null}
              expanded={expandedNoteId === note.id}
              {note}
              onaddentity={handleAddEntity}
              onchangecolor={handleChangeColor}
              oncollapse={handleCollapse}
              ondelete={handleDeleteNote}
              ondragcancel={() => handleDragCancel(note.id)}
              ondragend={handleDragEnd}
              ondragmove={updateDraggingPosition}
              ondragstart={(pointer) => handleDragStart(note.id, pointer)}
              onendresolve={handleEndResolve}
              onexpand={handleExpand}
              onremoveentity={handleRemoveEntity}
              ontogglestatus={handleToggleStatus}
              onupdatecontent={handleUpdateContent}
              resolving={resolvingNoteIds.has(note.id)}
            />
          {/each}
        </div>
      {/if}

      {#if resolvedNotes.length > 0}
        <button
          class={flex({
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
          완료됨 ({resolvedNotes.length})
        </button>

        {#if resolvedOpen}
          <div class={flex({ flexDirection: 'column', gap: '8px', marginTop: '4px' })} role="list">
            {#each resolvedNotes as note (note.id)}
              <NoteComponent
                cancelling={cancellingNoteIds.has(note.id)}
                draggingNoteId={dragging?.noteId ?? null}
                expanded={expandedNoteId === note.id}
                {note}
                onaddentity={handleAddEntity}
                onchangecolor={handleChangeColor}
                oncollapse={handleCollapse}
                ondelete={handleDeleteNote}
                ondragcancel={() => handleDragCancel(note.id)}
                ondragend={handleDragEnd}
                ondragmove={updateDraggingPosition}
                ondragstart={(pointer) => handleDragStart(note.id, pointer)}
                onendresolve={handleEndResolve}
                onexpand={handleExpand}
                onremoveentity={handleRemoveEntity}
                ontogglestatus={handleToggleStatus}
                onupdatecontent={handleUpdateContent}
                resolving={resolvingNoteIds.has(note.id)}
              />
            {/each}
          </div>
        {/if}
      {/if}

      {#if sortedNotes.length === 0}
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
    </div>
  </div>
</Modal>

<NoteEntitySearchModal
  existingEntityIds={entitySearchExistingIds}
  noteId={entitySearchNoteId ?? ''}
  onclose={() => (entitySearchNoteId = null)}
  open={entitySearchNoteId !== null}
/>
