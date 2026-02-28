<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { animateFlip, elementScrollViewport, getRandomNoteColor, handleDragScroll } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import DocumentPanelNoteItem from './DocumentPanelNoteItem.svelte';
  import type { DocumentPanel_Note_entity$key } from '$mearie';

  type Props = {
    entity$key: DocumentPanel_Note_entity$key;
  };

  let { entity$key }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment DocumentPanel_Note_entity on Entity {
        id
        notes {
          id
          order
          ...DocumentPanelNoteItem_note
        }
      }
    `),
    () => entity$key,
  );

  const [createNote] = createMutation(
    graphql(`
      mutation DocumentPanelNote_CreateNote_Mutation($input: CreateNoteInput!) {
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
      mutation DocumentPanelNote_MoveNote_Mutation($input: MoveNoteInput!) {
        moveNote(input: $input) {
          id
          order
        }
      }
    `),
  );

  let dragging = $state<{
    noteId: string;
    originalIndex: number;
  } | null>(null);
  let localNoteOrder = $state<string[]>([]);
  let scrollContainer = $state<HTMLElement | null>(null);

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

  let lastAddedNoteId = $state<string>();

  const handleAddNote = async (via: string) => {
    const randomColor = getRandomNoteColor();
    const result = await createNote({
      input: {
        content: '',
        color: randomColor,
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
  };

  const handleDragStart = (noteId: string) => {
    dragging = {
      noteId,
      originalIndex: localNoteOrder.indexOf(noteId),
    };
  };

  const handleDragEnter = (noteId: string) => {
    if (dragging && dragging.noteId !== noteId) {
      const draggedIndex = localNoteOrder.indexOf(dragging.noteId);
      const dropIndex = localNoteOrder.indexOf(noteId);

      if (draggedIndex !== -1 && dropIndex !== -1 && draggedIndex !== dropIndex) {
        const newOrder = [...localNoteOrder];
        const [removed] = newOrder.splice(draggedIndex, 1);
        newOrder.splice(dropIndex, 0, removed);
        localNoteOrder = newOrder;
      }
    }
  };

  const handleDragEnd = async () => {
    if (!dragging) return;

    const currentIndex = localNoteOrder.indexOf(dragging.noteId);

    if (currentIndex !== -1 && dragging.originalIndex !== -1 && currentIndex !== dragging.originalIndex && sortedNotes.length > 1) {
      const lowerNote = sortedNotes[currentIndex - 1] ?? null;
      const upperNote = sortedNotes[currentIndex + 1] ?? null;

      try {
        await moveNote({
          input: {
            noteId: dragging.noteId,
            lowerOrder: lowerNote?.order,
            upperOrder: upperNote?.order,
          },
        });
        mixpanel.track('move_related_note');
        cache.invalidate({ __typename: 'Entity', id: entity.data.id, $field: 'notes' });
      } catch {
        localNoteOrder = entity.data.notes.map((note) => note.id);
        Toast.error('노트 순서 변경에 실패했습니다. 잠시 후 다시 시도해주세요.');
      }
    }

    dragging = null;
  };

  let prevNoteIds = $state<string[]>([]);
  $effect(() => {
    const noteIds = entity.data.notes.map((n) => n.id);
    const noteIdsStr = noteIds.join(',');
    const prevNoteIdsStr = prevNoteIds.join(',');

    if (noteIdsStr !== prevNoteIdsStr) {
      prevNoteIds = noteIds;
      localNoteOrder = noteIds;
    }

    const noteElement = document.querySelector(`[data-related-note-id="${lastAddedNoteId}"] textarea`) as HTMLTextAreaElement;
    if (noteElement) {
      noteElement.focus();
      lastAddedNoteId = undefined;
    }
  });

  $effect(() => {
    return handleDragScroll(scrollContainer ? elementScrollViewport(scrollContainer) : null, !!dragging);
  });

  $effect.pre(() => {
    void localNoteOrder;
    animateFlip('[data-related-note-id]', 'relatedNoteId');
  });
</script>

<div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflow: 'hidden' })}>
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
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>이 문서 관련 노트</div>
      {#if notes.length > 0}
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
          {notes.length}
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
    {#if notes.length === 0}
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

        <Button onclick={() => handleAddNote('button')} size="sm" variant="secondary">노트 추가</Button>
      </div>
    {:else}
      {#each notes as note (note.id)}
        <DocumentPanelNoteItem
          draggingNoteId={dragging?.noteId ?? null}
          note$key={note}
          onAddNote={() => handleAddNote('shortcut')}
          onDragEnd={handleDragEnd}
          onDragEnter={() => handleDragEnter(note.id)}
          onDragStart={() => handleDragStart(note.id)}
        />
      {/each}
    {/if}
  </div>
</div>
