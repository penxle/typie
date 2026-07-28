<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { animateFlip, createDragScroll, elementScrollViewport } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick, untrack } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { cache } from '$lib/graphql';
  import { reorderedNoteIdsForDrag } from '$lib/note-reorder';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../../../@subscription/subscribe-modal.svelte';
  import DocumentPanelNoteItem from './DocumentPanelNoteItem.svelte';
  import type { NoteReorderDirection, NoteReorderGeometry } from '$lib/note-reorder';
  import type { DocumentPanelV2_Note_entity$key } from '$mearie';

  type NoteDragPosition = {
    clientX: number;
    clientY: number;
    direction: NoteReorderDirection;
    ghost: NoteReorderGeometry;
  };

  type Props = {
    entity$key: DocumentPanelV2_Note_entity$key;
  };

  let { entity$key }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment DocumentPanelV2_Note_entity on Entity {
        id
        notes {
          id
          order
          status
          ...DocumentPanelV2NoteItem_note
        }
      }
    `),
    () => entity$key,
  );

  const [createNote] = createMutation(
    graphql(`
      mutation DocumentPanelV2Note_CreateNote_Mutation($input: CreateNoteInput!) {
        createNote(input: $input) {
          id
          content
          color
          order
          entity {
            id
          }
        }
      }
    `),
  );

  const [moveNote] = createMutation(
    graphql(`
      mutation DocumentPanelV2Note_MoveNote_Mutation($input: MoveNoteInput!) {
        moveNote(input: $input) {
          id
          order
        }
      }
    `),
  );

  let dragging = $state<{
    noteId: string;
    originalOrder: string[];
    position: NoteDragPosition | null;
    sectionNoteIds: string[];
  } | null>(null);
  let localNoteOrder = $state<string[]>([]);
  let scrollContainer = $state<HTMLElement | null>(null);
  let dragScroll: ReturnType<typeof createDragScroll> | null = null;
  let createInFlight = $state(false);

  const sortedNotes = $derived.by(() => {
    if (localNoteOrder.length === 0) {
      return entity.data.notes.toSorted((a, b) => a.order.localeCompare(b.order));
    }
    return [...entity.data.notes].toSorted((a, b) => {
      const indexA = localNoteOrder.indexOf(a.id);
      const indexB = localNoteOrder.indexOf(b.id);
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  });

  const notes = $derived(sortedNotes || []);
  const resolvingNoteIds = new SvelteSet<string>();
  const openNotes = $derived(notes.filter((n) => n.status === 'OPEN' || resolvingNoteIds.has(n.id)));
  const resolvedNotes = $derived(notes.filter((n) => n.status === 'RESOLVED' && !resolvingNoteIds.has(n.id)));
  let resolvedExpanded = $state(false);

  const handleBeginResolve = (noteId: string) => {
    resolvingNoteIds.add(noteId);
  };

  const handleEndResolve = (noteId: string) => {
    resolvingNoteIds.delete(noteId);
    cache.invalidate({ __typename: 'Entity', id: entity.data.id, $field: 'notes' });
  };

  let lastAddedNoteId = $state<string>();

  const handleAddNote = async (via: string) => {
    if (createInFlight) return;

    if (!SubscribeModal.gate('notes_create')) {
      return;
    }

    createInFlight = true;
    try {
      const result = await createNote({
        input: {
          content: '',
          color: 'gray',
          entityId: entity.data.id,
        },
      });

      if (result?.createNote?.id) {
        lastAddedNoteId = result.createNote.id;
        mixpanel.track('create_related_note', {
          via,
        });
        cache.invalidate({ __typename: 'Entity', id: entity.data.id, $field: 'notes' });
      }
    } finally {
      createInFlight = false;
    }
  };

  const handleDragStart = (noteId: string) => {
    const sectionNotes = openNotes.some((note) => note.id === noteId) ? openNotes : resolvedNotes;
    const originalOrder = sortedNotes.map((note) => note.id);
    localNoteOrder = [...originalOrder];
    dragging = {
      noteId,
      originalOrder,
      position: null,
      sectionNoteIds: sectionNotes.map((note) => note.id),
    };
    return true;
  };

  const handleDragCancel = (noteId: string) => {
    const currentDragging = dragging;
    if (currentDragging?.noteId !== noteId) return;

    dragging = null;
    localNoteOrder = [...currentDragging.originalOrder];
  };

  const resolveDraggingPosition = (position: NoteDragPosition) => {
    const currentDragging = dragging;
    if (!currentDragging || !scrollContainer) return;

    const sectionNoteIds = new Set(currentDragging.sectionNoteIds);
    const currentSectionOrder = localNoteOrder.filter((noteId) => sectionNoteIds.has(noteId));
    const noteGeometries = new SvelteMap<string, NoteReorderGeometry>();

    for (const noteElement of scrollContainer.querySelectorAll<HTMLElement>('[data-related-note-id]')) {
      const noteId = noteElement.dataset.relatedNoteId;
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

  const handleDragEnd = async () => {
    const currentDragging = dragging;
    if (!currentDragging) return;

    const currentIndex = localNoteOrder.indexOf(currentDragging.noteId);
    const originalIndex = currentDragging.originalOrder.indexOf(currentDragging.noteId);
    dragging = null;

    if (currentIndex !== -1 && originalIndex !== -1 && currentIndex !== originalIndex && sortedNotes.length > 1) {
      if (!SubscribeModal.gate('notes_move')) {
        localNoteOrder = [...currentDragging.originalOrder];
        return;
      }

      const lowerNote = sortedNotes[currentIndex - 1] ?? null;
      const upperNote = sortedNotes[currentIndex + 1] ?? null;

      try {
        await moveNote({
          input: {
            noteId: currentDragging.noteId,
            lowerOrder: lowerNote?.order,
            upperOrder: upperNote?.order,
          },
        });
        mixpanel.track('move_related_note');
        cache.invalidate({ __typename: 'Entity', id: entity.data.id, $field: 'notes' });
      } catch {
        localNoteOrder = [...currentDragging.originalOrder];
        Toast.error('노트 순서 변경에 실패했습니다. 잠시 후 다시 시도해주세요.');
      }
    }
  };

  let prevNoteIds = $state<string[]>([]);
  $effect(() => {
    const noteIds = entity.data.notes.map((note) => note.id);
    if (!dragging) {
      const noteIdsStr = noteIds.join(',');
      const prevNoteIdsStr = prevNoteIds.join(',');

      if (noteIdsStr !== prevNoteIdsStr) {
        prevNoteIds = noteIds;
        localNoteOrder = noteIds;
      }
    }

    if (lastAddedNoteId && noteIds.includes(lastAddedNoteId)) {
      const targetId = lastAddedNoteId;
      lastAddedNoteId = undefined;
      tick().then(() => {
        const noteElement = document.querySelector(`[data-related-note-id="${targetId}"] textarea`) as HTMLTextAreaElement;
        noteElement?.focus();
      });
    }
  });

  $effect(() => {
    const current = dragging;
    if (!scrollContainer || !current) return;

    const initialPointer = untrack(() => {
      const position = current.position;
      return position ? { clientX: position.clientX, clientY: position.clientY } : undefined;
    });
    const activeDragScroll = createDragScroll(elementScrollViewport(scrollContainer), {
      initialPointer,
      onScroll: () => {
        if (dragging === current && current.position) {
          resolveDraggingPosition(current.position);
        }
      },
    });
    dragScroll = activeDragScroll;
    return () => {
      activeDragScroll.destroy();
      if (dragScroll === activeDragScroll) dragScroll = null;
    };
  });

  $effect.pre(() => {
    void localNoteOrder;
    animateFlip('[data-related-note-id]', 'relatedNoteId');
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
      {#if openNotes.length > 0}
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
          {openNotes.length}
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
    {#if openNotes.length === 0}
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

        <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
          <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
            떠오르는 생각이나 아이디어를
            <br />
            자유롭게 기록해보세요
          </p>
        </div>

        <Button loading={createInFlight} onclick={() => handleAddNote('button')} size="sm" variant="secondary">노트 추가</Button>
      </div>
    {:else}
      {#each openNotes as note (note.id)}
        <DocumentPanelNoteItem
          draggingNoteId={dragging?.noteId ?? null}
          note$key={note}
          onAddNote={() => handleAddNote('shortcut')}
          onBeginResolve={() => handleBeginResolve(note.id)}
          onDragCancel={() => handleDragCancel(note.id)}
          onDragEnd={handleDragEnd}
          onDragMove={updateDraggingPosition}
          onDragStart={() => handleDragStart(note.id)}
          onEndResolve={() => handleEndResolve(note.id)}
          resolving={resolvingNoteIds.has(note.id)}
        />
      {/each}
    {/if}

    <!-- 완료됨 섹션 (빈 상태와 무관하게 항상 표시) -->
    {#if resolvedNotes.length > 0}
      <div
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
            {resolvedNotes.length}
          </div>
        </button>
        {#if resolvedExpanded}
          {#each resolvedNotes as note (note.id)}
            <DocumentPanelNoteItem
              draggingNoteId={dragging?.noteId ?? null}
              note$key={note}
              onAddNote={() => handleAddNote('shortcut')}
              onBeginResolve={() => {
                /* noop: resolved notes */
              }}
              onDragCancel={() => handleDragCancel(note.id)}
              onDragEnd={handleDragEnd}
              onDragMove={updateDraggingPosition}
              onDragStart={() => handleDragStart(note.id)}
              onEndResolve={() => {
                /* noop: resolved notes */
              }}
              resolving={false}
            />
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</div>
