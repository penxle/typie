<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal, Select } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { values } from '@typie/ui/tiptap/values-base';
  import { clamp, debounce } from '@typie/ui/utils';
  import { tick, untrack } from 'svelte';
  import { fly } from 'svelte/transition';
  import { match } from 'ts-pattern';
  import { PostType } from '@/enums';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import FileIcon from '~icons/lucide/file';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import ShapesIcon from '~icons/lucide/shapes';
  import { beforeNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { cache, fragment, graphql } from '$graphql';
  import Masonry from './Masonry.svelte';
  import NoteComponent from './Note.svelte';
  import type { DashboardLayout_Notes_query } from '$graphql';

  type Props = {
    $query: DashboardLayout_Notes_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment DashboardLayout_Notes_query on Query {
        me @required {
          id

          recentlyViewedEntities {
            id
            slug

            node {
              __typename

              ... on Post {
                id
                title
                type
              }
              ... on Canvas {
                id
                title
              }
            }
          }
        }

        notes {
          id
          content
          createdAt
          updatedAt
          order
          color
          entity {
            id
            slug

            node {
              __typename

              ... on Post {
                id
                title
                type
              }
              ... on Canvas {
                id
                title
              }
            }
          }
        }
      }
    `),
  );

  const currentEntityQuery = graphql(`
    query DashboardLayout_Notes_CurrentEntity_Query($slug: String!) @client {
      entity(slug: $slug) {
        id

        node {
          __typename

          ... on Post {
            id
            title
            type
          }
          ... on Canvas {
            id
            title
          }
        }
      }
    }
  `);

  const createNote = graphql(`
    mutation DashboardLayout_Notes_CreateNote_Mutation($input: CreateNoteInput!) {
      createNote(input: $input) {
        id
        content
        createdAt
        color
        entity {
          id
          slug

          node {
            __typename

            ... on Post {
              id
              title
              type
            }
            ... on Canvas {
              id
              title
            }
          }
        }
      }
    }
  `);

  const updateNote = graphql(`
    mutation DashboardLayout_Notes_UpdateNote_Mutation($input: UpdateNoteInput!) {
      updateNote(input: $input) {
        id
        content
        updatedAt
        entity {
          id
          slug

          node {
            __typename

            ... on Post {
              id
              title
              type
            }
            ... on Canvas {
              id
              title
            }
          }
        }
      }
    }
  `);

  const deleteNote = graphql(`
    mutation DashboardLayout_Notes_DeleteNote_Mutation($input: DeleteNoteInput!) {
      deleteNote(input: $input) {
        id
      }
    }
  `);

  const moveNote = graphql(`
    mutation DashboardLayout_Notes_MoveNote_Mutation($input: MoveNoteInput!) {
      moveNote(input: $input) {
        id
        order
      }
    }
  `);

  const app = getAppContext();

  const colors = values.textBackgroundColor.filter((color) => color.value !== 'none').map((color) => color.hex);
  const getRandomColor = () => colors[Math.floor(Math.random() * colors.length)];

  let inputValue = $state('');
  let inputEl = $state<HTMLTextAreaElement>();
  let selectedEntityId = $state<string | null>(null);
  const selectedEntityTitle = $derived.by(() => {
    const note = $query.notes.find((note) => note.entity?.id === selectedEntityId);
    if (!note) return null;

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    return match(note.entity!.node)
      .with({ __typename: 'Post' }, (node) => node.title)
      .with({ __typename: 'Canvas' }, (node) => node.title)
      .with({ __typename: 'Folder' }, () => null)
      .exhaustive();
  });

  let editingNoteId = $state<string | null>(null);
  let editingValue = $state('');
  let editInputEl = $state<HTMLTextAreaElement>();
  let editSelectedEntityId = $state<string | null>(null);

  const currentEntity = $derived($currentEntityQuery && page.params.slug ? $currentEntityQuery.entity : null);
  const recentlyViewedEntities = $derived(
    $query.me.recentlyViewedEntities
      .slice(0, 10)
      .map((entity) =>
        match(entity.node)
          .with({ __typename: 'Post' }, (node) => ({
            entity,
            title: node.title,
            icon: node.type === PostType.TEMPLATE ? ShapesIcon : FileIcon,
          }))
          .with({ __typename: 'Canvas' }, (node) => ({
            entity,
            title: node.title,
            icon: LineSquiggleIcon,
          }))
          .with({ __typename: 'Folder' }, () => null)
          .exhaustive(),
      )
      .filter((hit): hit is NonNullable<typeof hit> => hit !== null),
  );

  const notesRelatedToEntity = $derived($query.notes.filter((note) => selectedEntityId && note.entity?.id === selectedEntityId));
  const notesNotRelatedToEntity = $derived($query.notes.filter((note) => note.entity?.id !== selectedEntityId));
  const restNotes = $derived(notesRelatedToEntity.length > 0 ? notesNotRelatedToEntity : $query.notes);

  let draggedNoteId = $state<string | null>(null);
  let dragOverNoteId = $state<string | null>(null);
  let prevNotePositions = $state<Record<string, DOMRect>>({});

  const handleDragEnd = () => {
    if (draggedNoteId) {
      draggedNoteId = null;
      dragOverNoteId = null;
    }
  };

  const handleNoteDragEnter = debounce(async (noteId: string) => {
    if (draggedNoteId && draggedNoteId !== noteId) {
      dragOverNoteId = noteId;
      const draggedIndex = $query.notes.findIndex((n) => n.id === draggedNoteId);
      const dropIndex = $query.notes.findIndex((n) => n.id === noteId);
      if (draggedIndex !== -1 && dropIndex !== -1) {
        const positions: Record<string, DOMRect> = {};
        const noteElements = document.querySelectorAll('[data-note-id]');
        noteElements.forEach((el) => {
          const id = (el as HTMLElement).dataset.noteId;
          if (id) {
            positions[id] = el.getBoundingClientRect();
          }
        });
        prevNotePositions = positions;

        let lowerNote, upperNote;

        if (draggedIndex < dropIndex) {
          lowerNote = $query.notes[dropIndex];
          upperNote = $query.notes[dropIndex + 1] || null;
        } else if (draggedIndex > dropIndex) {
          lowerNote = dropIndex > 0 ? $query.notes[dropIndex - 1] : null;
          upperNote = $query.notes[dropIndex];
        } else {
          return;
        }

        await moveNote({
          noteId: draggedNoteId,
          lowerOrder: lowerNote?.order,
          upperOrder: upperNote?.order,
        });
        cache.invalidate({ __typename: 'Query', field: 'notes' });
      }
    }
  }, 50);

  $effect(() => {
    void $query.notes;

    untrack(() => {
      tick().then(() => {
        const noteElements = document.querySelectorAll('[data-note-id]');

        if (Object.keys(prevNotePositions).length === 0) return;

        for (const el of noteElements) {
          const id = (el as HTMLElement).dataset.noteId;
          if (!id || !prevNotePositions[id]) continue;

          const prevPos = prevNotePositions[id];
          const lastPos = el.getBoundingClientRect();
          const deltaX = prevPos.left - lastPos.left;
          const deltaY = prevPos.top - lastPos.top;

          if (Math.abs(deltaX) === 0 && Math.abs(deltaY) === 0) continue;

          const htmlEl = el as HTMLElement;
          htmlEl.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
          htmlEl.style.transition = 'none';

          requestAnimationFrame(() => {
            htmlEl.style.transition = 'transform 300ms cubic-bezier(0.4, 0, 0.2, 1)';
            htmlEl.style.transform = '';
            htmlEl.style.pointerEvents = 'none';
            setTimeout(() => {
              htmlEl.style.transition = 'none';
              htmlEl.style.pointerEvents = 'auto';
              prevNotePositions = {};
            }, 300);
          });
        }
      });
    });
  });

  const handleKeyDown = (event: KeyboardEvent) => {
    const metaOrCtrlKeyOnly = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey);

    if (metaOrCtrlKeyOnly && event.key === 'j') {
      event.preventDefault();
      app.state.notesOpen = !app.state.notesOpen;
    } else if (app.state.notesOpen && event.key === 'Escape') {
      event.stopPropagation();

      if (editingNoteId) {
        closeEditModal();
        return;
      }

      close();
      return;
    }
  };

  const handleAddNote = async () => {
    if (!inputValue.trim()) return;

    await createNote({
      color: getRandomColor(),
      content: inputValue,
      entityId: selectedEntityId,
    });
    cache.invalidate({ __typename: 'Query', field: 'notes' });

    inputValue = '';
    inputEl?.focus();
  };

  const handleDeleteNote = async (noteId: string) => {
    await deleteNote({ noteId });
    cache.invalidate({ __typename: 'Query', field: 'notes' });
  };

  const editNote = (id: string) => {
    const note = $query.notes.find((n) => n.id === id);
    if (note) {
      editingNoteId = note.id;
      editingValue = note.content;
      editSelectedEntityId = note.entity?.id || null;
    }
  };

  const handleSaveEdit = async () => {
    if (editingNoteId && editingValue.trim()) {
      await updateNote({
        noteId: editingNoteId,
        content: editingValue.trim(),
        entityId: editSelectedEntityId,
      });

      closeEditModal();
    }
  };

  const closeEditModal = () => {
    editingNoteId = null;
    editingValue = '';
    editSelectedEntityId = null;
    inputEl?.focus();
  };

  const close = () => {
    app.state.notesOpen = false;
  };

  beforeNavigate(() => {
    close();
  });

  $effect(() => {
    if (app.state.notesOpen) {
      cache.invalidate({ __typename: 'Query', field: 'notes' });

      if (inputEl) {
        inputEl.focus();
      }
    }
  });

  $effect(() => {
    void currentEntity;

    untrack(() => {
      if (currentEntity?.id && recentlyViewedEntities.some((entity) => entity.entity.id === currentEntity.id)) {
        selectedEntityId = currentEntity.id;
      } else {
        selectedEntityId = null;
      }
    });
  });

  $effect(() => {
    if (editingNoteId && editInputEl) {
      editInputEl.focus();
      editInputEl.setSelectionRange(editingValue.length, editingValue.length);
    }
  });

  $effect(() => {
    if (page.params.slug) {
      currentEntityQuery.load({ slug: page.params.slug });
    }
  });
</script>

<svelte:window ondragend={handleDragEnd} onkeydown={handleKeyDown} />

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
  >
    <div
      class={css({
        position: 'absolute',
        inset: '0',
      })}
      onclick={close}
      role="none"
    ></div>

    <div
      class={flex({
        position: 'sticky',
        top: '[calc(16px - 15dvh)]',
        zIndex: '1',
        flexDirection: 'column',
        width: 'full',
        maxWidth: '450px',
        flexShrink: '0',
        backgroundColor: 'surface.default',
        borderRadius: '12px',
        overflow: 'hidden',
        boxShadow: 'large',
      })}
    >
      <textarea
        bind:this={inputEl}
        class={css({
          width: 'full',
          minHeight: '80px',
          padding: '16px',
          fontSize: '16px',
          fontWeight: 'medium',
          color: 'text.default',
          borderRadius: '8px',
          resize: 'none',
        })}
        onkeydown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !e.isComposing) {
            e.preventDefault();
            handleAddNote();
          }
        }}
        placeholder="기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요."
        bind:value={inputValue}
      ></textarea>

      <div class={flex({ gap: '4px', alignItems: 'center', paddingX: '12px', paddingY: '6px' })}>
        <span class={css({ flexShrink: '0', fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>관련 항목:</span>
        <Select
          items={[
            { label: '없음', value: null },
            ...recentlyViewedEntities.map((entity) => ({ label: entity.title, value: entity.entity.id, icon: entity.icon })),
          ]}
          onselect={(value) => {
            selectedEntityId = value;
          }}
          value={selectedEntityId}
        />
        <kbd
          class={center({
            marginLeft: 'auto',
            flexShrink: '0',
            gap: '2px',
            borderRadius: '4px',
            paddingX: '6px',
            paddingY: '2px',
            fontFamily: 'mono',
            fontSize: '13px',
            fontWeight: 'medium',
            color: 'text.faint',
            backgroundColor: 'surface.muted',
          })}
        >
          <span>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}</span>
          {#if !navigator.platform.includes('Mac')}
            <span>+</span>
          {/if}
          <span>J</span>
        </kbd>
      </div>

      <div class={css({ height: '1px', backgroundColor: 'interactive.hover' })}></div>

      <div
        class={flex({
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingX: '12px',
          paddingY: '6px',
          backgroundColor: 'surface.muted',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '16px', color: 'text.faint' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', paddingX: '4px', height: '22px' })}>
              <div class={css({ fontSize: '10px', fontWeight: 'bold' })}>ESC</div>
            </div>

            <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>닫기</span>
          </div>

          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', paddingX: '4px', height: '22px' })}>
              <div class={flex({ fontSize: '10px', fontWeight: 'bold', alignItems: 'center', gap: '2px' })}>
                {#if !navigator.platform.includes('Mac')}
                  <span>Ctrl</span>
                {:else}
                  <span class={css({ fontSize: '14px' })}>⌘</span>
                {/if}
                <Icon icon={CornerDownLeftIcon} size={12} />
              </div>
            </div>

            <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>추가</span>
          </div>
        </div>

        <Button style={css.raw({ flexShrink: '0' })} disabled={!inputValue.trim()} onclick={handleAddNote} size="sm" variant="secondary">
          추가
        </Button>
      </div>
    </div>

    <div
      class={css({
        paddingBottom: '50px',
        maxWidth: '8/12',
        flexGrow: '1',
        width: 'full',
      })}
    >
      {#if notesRelatedToEntity.length > 0}
        <h1
          class={css({
            position: 'relative',
            width: 'fit',
            zIndex: '1',
            fontSize: '16px',
            fontWeight: 'semibold',
            color: 'text.subtle',
            borderRadius: '8px',
            paddingX: '10px',
            paddingY: '4px',
            backgroundColor: 'surface.default/70',
          })}
          in:fly={{ y: 10, duration: 150 }}
        >
          <span class={css({ fontWeight: 'bold', color: 'text.default' })}>{selectedEntityTitle || '현재 항목'}</span>
          관련 노트
        </h1>
        <Masonry
          style={css.raw({ height: 'fit' })}
          ondrop={(e) => {
            e.preventDefault();
            draggedNoteId = null;
            dragOverNoteId = null;
          }}
        >
          {#each notesRelatedToEntity as note (note.id)}
            <NoteComponent
              dropTargetNoteId={dragOverNoteId}
              isDragging={draggedNoteId === note.id}
              {note}
              ondelete={handleDeleteNote}
              ondragenter={() => {
                if (draggedNoteId !== note.id) {
                  handleNoteDragEnter(note.id);
                }
              }}
              ondragstart={() => {
                draggedNoteId = note.id;
              }}
              onedit={editNote}
            />
          {/each}
        </Masonry>
      {/if}
      {#if notesRelatedToEntity.length > 0 && restNotes.length > 0}
        <h1
          class={css({
            marginTop: '16px',
            position: 'relative',
            width: 'fit',
            zIndex: '1',
            fontSize: '16px',
            fontWeight: 'semibold',
            color: 'text.subtle',
            borderRadius: '8px',
            paddingX: '10px',
            paddingY: '4px',
            backgroundColor: 'surface.default/70',
          })}
          in:fly={{ y: 10, duration: 150 }}
        >
          모든 노트
        </h1>
      {/if}
      <Masonry
        ondrop={(e) => {
          e.preventDefault();
          draggedNoteId = null;
          dragOverNoteId = null;
        }}
      >
        {#each restNotes as note (note.id)}
          <NoteComponent
            dropTargetNoteId={dragOverNoteId}
            isDragging={draggedNoteId === note.id}
            {note}
            ondelete={handleDeleteNote}
            ondragenter={() => {
              if (draggedNoteId !== note.id) {
                handleNoteDragEnter(note.id);
              }
            }}
            ondragstart={() => {
              draggedNoteId = note.id;
            }}
            onedit={editNote}
          />
        {/each}
      </Masonry>
    </div>
  </div>
</Modal>

<Modal
  style={css.raw({
    backgroundColor: 'transparent',
    maxWidth: '450px',
    border: 'none',
  })}
  focusTrapOptions={{
    returnFocusOnDeactivate: false,
  }}
  onclose={closeEditModal}
  open={editingNoteId !== null}
>
  <div
    class={flex({
      flexDirection: 'column',
      width: 'full',
      backgroundColor: 'surface.default',
      borderRadius: '12px',
      overflow: 'hidden',
    })}
  >
    <textarea
      bind:this={editInputEl}
      class={css({
        width: 'full',
        padding: '16px',
        fontSize: '16px',
        fontWeight: 'medium',
        color: 'text.default',
        borderRadius: '8px',
        resize: 'none',
      })}
      onkeydown={(e) => {
        if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !e.isComposing) {
          e.preventDefault();
          handleSaveEdit();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          closeEditModal();
        }
      }}
      placeholder="기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요."
      rows={clamp(editingValue.split('\n').length, 5, 15)}
      bind:value={editingValue}
    ></textarea>

    <div class={flex({ gap: '4px', alignItems: 'center', paddingX: '12px', paddingY: '6px' })}>
      <span class={css({ flexShrink: '0', fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>관련 항목:</span>
      <Select
        items={[
          { label: '없음', value: null },
          ...recentlyViewedEntities.map((entity) => ({ label: entity.title, value: entity.entity.id, icon: entity.icon })),
        ]}
        onselect={(value) => {
          editSelectedEntityId = value;
        }}
        value={editSelectedEntityId}
      />
    </div>

    <div class={css({ height: '1px', backgroundColor: 'interactive.hover' })}></div>

    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '12px',
        paddingY: '6px',
        backgroundColor: 'surface.muted',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '16px', color: 'text.faint' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', paddingX: '4px', height: '22px' })}>
            <div class={css({ fontSize: '10px', fontWeight: 'bold' })}>ESC</div>
          </div>

          <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>취소</span>
        </div>

        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <div class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: '6px', paddingX: '4px', height: '22px' })}>
            <div class={flex({ fontSize: '10px', fontWeight: 'bold', alignItems: 'center', gap: '2px' })}>
              {#if !navigator.platform.includes('Mac')}
                <span>Ctrl</span>
              {:else}
                <span class={css({ fontSize: '14px' })}>⌘</span>
              {/if}
              <Icon icon={CornerDownLeftIcon} size={12} />
            </div>
          </div>

          <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>저장</span>
        </div>
      </div>

      <div class={flex({ gap: '8px' })}>
        <Button onclick={closeEditModal} size="sm" variant="secondary">취소</Button>
        <Button disabled={!editingValue.trim()} onclick={handleSaveEdit} size="sm">저장</Button>
      </div>
    </div>
  </div>
</Modal>
