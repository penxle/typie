<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { animateFlip, createDragScroll, elementScrollViewport } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick, untrack } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import ExpandIcon from '~icons/lucide/expand';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { cache } from '$lib/graphql';
  import { reorderedNoteIdsForDrag } from '$lib/note-reorder';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../../@subscription/subscribe-modal.svelte';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';
  import DocumentRelatedNoteWidgetItem from './DocumentRelatedNoteWidgetItem.svelte';
  import type { NoteReorderDirection, NoteReorderGeometry } from '$lib/note-reorder';

  type NoteDragPosition = {
    clientX: number;
    clientY: number;
    direction: NoteReorderDirection;
    ghost: NoteReorderGeometry;
  };

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const widgetContext = getWidgetContext();
  const { palette, document$key } = $derived(widgetContext.env);

  const relatedDocument = createFragment(
    graphql(`
      fragment Editor_Widget_DocumentRelatedNoteWidget_document on Document {
        id

        entity {
          id
          notes {
            id
            order
            status
            ...DocumentRelatedNoteWidgetItem_note
          }
        }
      }
    `),
    () => document$key,
  );

  const [createNote] = createMutation(
    graphql(`
      mutation Editor_Widget_DocumentRelatedNoteWidget_CreateNote_Mutation($input: CreateNoteInput!) {
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
      mutation Editor_Widget_DocumentRelatedNoteWidget_MoveNote_Mutation($input: MoveNoteInput!) {
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
  let isExpanded = $state((data.isExpanded as boolean) ?? false);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  const toggleExpanded = () => {
    isExpanded = !isExpanded;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded, isCollapsed });
  };

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded, isCollapsed });
  };

  const sortedNotes = $derived.by(() => {
    if (!relatedDocument.data) return [];
    const notes = relatedDocument.data.entity.notes;
    if (localNoteOrder.length === 0) {
      return notes.toSorted((a, b) => a.order.localeCompare(b.order));
    }
    return [...notes].toSorted((a, b) => {
      const indexA = localNoteOrder.indexOf(a.id);
      const indexB = localNoteOrder.indexOf(b.id);
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  });

  const notes = $derived(sortedNotes);
  const resolvingNoteIds = new SvelteSet<string>();
  const openNotes = $derived(notes.filter((n) => n.status === 'OPEN' || resolvingNoteIds.has(n.id)));

  const handleBeginResolve = (noteId: string) => {
    resolvingNoteIds.add(noteId);
  };

  const handleEndResolve = (noteId: string) => {
    resolvingNoteIds.delete(noteId);
    if (relatedDocument.data?.entity.id) {
      cache.invalidate({ __typename: 'Entity', id: relatedDocument.data.entity.id, $field: 'notes' });
    }
  };

  let lastAddedNoteId = $state<string>();

  const handleAddNote = async (via: string) => {
    if (!relatedDocument.data?.entity.id) return;

    if (!SubscribeModal.gate('notes_create')) {
      return;
    }

    const result = await createNote({
      input: {
        content: '',
        color: 'gray',
        entityId: relatedDocument.data.entity.id,
      },
    });

    if (result?.createNote?.id) {
      lastAddedNoteId = result.createNote.id;
      mixpanel.track('create_related_note', {
        via,
      });
      cache.invalidate({ __typename: 'Entity', id: relatedDocument.data.entity.id, $field: 'notes' });
    }
  };

  const handleDragStart = (noteId: string) => {
    const originalOrder = sortedNotes.map((note) => note.id);
    localNoteOrder = [...originalOrder];
    dragging = {
      noteId,
      originalOrder,
      position: null,
      sectionNoteIds: openNotes.map((note) => note.id),
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

    for (const noteElement of scrollContainer.querySelectorAll<HTMLElement>('[data-widget-note-id]')) {
      const noteId = noteElement.dataset.widgetNoteId;
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

    const currentDocument = relatedDocument.data;
    if (!currentDocument) return;

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
        cache.invalidate({ __typename: 'Entity', id: currentDocument.entity.id, $field: 'notes' });
      } catch {
        localNoteOrder = currentDocument.entity.notes.map((note) => note.id);
        Toast.error('노트 순서 변경에 실패했습니다. 잠시 후 다시 시도해주세요.');
      }
    }
  };

  let prevNoteIds = $state<string[]>([]);
  $effect(() => {
    const noteIds = relatedDocument.data?.entity.notes.map((n) => n.id) ?? [];
    const noteIdsStr = noteIds.join(',');
    const prevNoteIdsStr = prevNoteIds.join(',');

    if (!dragging && noteIdsStr !== prevNoteIdsStr) {
      prevNoteIds = noteIds;
      localNoteOrder = noteIds;
    }

    if (lastAddedNoteId && noteIds.includes(lastAddedNoteId)) {
      const targetId = lastAddedNoteId;
      lastAddedNoteId = undefined;
      tick().then(() => {
        const noteElement = document.querySelector(`[data-widget-note-id="${targetId}"] textarea`) as HTMLTextAreaElement;
        noteElement?.focus();
      });
    }
  });

  $effect(() => {
    const current = dragging;
    if (palette || !scrollContainer || !current) return;

    const initialPointer = untrack(() => {
      const position = current.position;
      return position ? { clientX: position.clientX, clientY: position.clientY } : undefined;
    });
    const activeDragScroll = createDragScroll(elementScrollViewport(scrollContainer), {
      initialPointer,
      onScroll: () => {
        if (dragging?.noteId === current.noteId && current.position) {
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
    if (!scrollContainer || palette) return;

    animateFlip('[data-widget-note-id]', 'widgetNoteId', scrollContainer);
  });
</script>

<Widget collapsed={isCollapsed} icon={StickyNoteIcon} noPadding title="노트">
  {#snippet headerActions()}
    {#if !palette && !isCollapsed}
      <button
        class={center({
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
          cursor: 'pointer',
        })}
        onclick={(e) => {
          e.stopPropagation();
          handleAddNote('button');
        }}
        onpointerdown={(e) => {
          e.stopPropagation();
        }}
        type="button"
      >
        <Icon icon={PlusIcon} size={14} />
      </button>
      <button
        class={center({
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
          cursor: 'pointer',
        })}
        onclick={(e) => {
          e.stopPropagation();
          toggleExpanded();
        }}
        onpointerdown={(e) => {
          e.stopPropagation();
        }}
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
          color: 'text.subtle',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
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
    {#if openNotes.length === 0}
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
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={StickyNoteIcon} size={20} />
        </div>

        <p class={css({ fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>
          떠오르는 생각이나 아이디어를
          <br />
          자유롭게 기록해보세요
        </p>
      </div>
    {:else}
      {#each openNotes as note (note.id)}
        <DocumentRelatedNoteWidgetItem
          draggingNoteId={dragging?.noteId ?? null}
          note$key={note}
          onAddNote={() => handleAddNote('shortcut')}
          onBeginResolve={() => handleBeginResolve(note.id)}
          onDragCancel={() => handleDragCancel(note.id)}
          onDragEnd={handleDragEnd}
          onDragMove={updateDraggingPosition}
          onDragStart={() => handleDragStart(note.id)}
          onEndResolve={() => handleEndResolve(note.id)}
          {palette}
          resolving={resolvingNoteIds.has(note.id)}
        />
      {/each}
    {/if}
  </div>
</Widget>
