<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Icon, Popover } from '@typie/ui/components';
  import { getNoteColors } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import type { DocumentPanelNoteItem_note$key } from '$mearie';

  type Props = {
    note$key: DocumentPanelNoteItem_note$key;
    draggingNoteId: string | null;
    onDragStart: () => void;
    onDragEnter: () => void;
    onDragEnd: () => void;
    onAddNote: () => void;
  };

  let { note$key, draggingNoteId, onDragStart, onDragEnter, onDragEnd, onAddNote }: Props = $props();

  const note = createFragment(
    graphql(`
      fragment DocumentPanelNoteItem_note on Note {
        id
        content
        color
        entity {
          id
        }
      }
    `),
    () => note$key,
  );

  const [updateNote] = createMutation(
    graphql(`
      mutation DocumentPanelNoteItem_UpdateNote_Mutation($input: UpdateNoteInput!) {
        updateNote(input: $input) {
          id
          content
          updatedAt
        }
      }
    `),
  );

  const [deleteNote] = createMutation(
    graphql(`
      mutation DocumentPanelNoteItem_DeleteNote_Mutation($input: DeleteNoteInput!) {
        deleteNote(input: $input) {
          id
        }
      }
    `),
  );

  let content = $state(note.data.content);
  let focused = $state(false);
  let dirty = $state(false);
  let contentUpdateTimeout: ReturnType<typeof setTimeout> | null = null;

  const isDragging = $derived(draggingNoteId === note.data.id);
  const anyDragging = $derived(draggingNoteId !== null);
  const color = $derived(getNoteColors().find((c) => c.value === note.data.color)?.color ?? token('colors.prosemirror.white'));

  $effect(() => {
    const serverContent = note.data.content;

    if (dirty && serverContent === content) {
      dirty = false;
    }

    if (!dirty && !focused) {
      content = serverContent;
    }
  });

  function flushContentUpdate() {
    if (!contentUpdateTimeout) return;
    clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = null;
    updateNote({
      input: {
        noteId: note.data.id,
        content,
      },
    });
  }

  function handleContentChanged() {
    dirty = true;
    if (contentUpdateTimeout) clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = setTimeout(flushContentUpdate, 300);
  }

  const handleDeleteNote = async () => {
    const entityId = note.data.entity?.id;
    await deleteNote({ input: { noteId: note.data.id } });
    mixpanel.track('delete_related_note');
    if (entityId) {
      cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'notes' });
    }
  };
</script>

<div
  style:background-color={`color-mix(in srgb, ${token('colors.prosemirror.white')}, ${color} 75%)`}
  style:opacity={isDragging ? '0.5' : '1'}
  class={cx(
    'group',
    flex({
      flexDirection: 'column',
      gap: '8px',
      position: 'relative',
      clipPath: 'polygon(0 0, 100% 0, 100% calc(100% - 12px), calc(100% - 12px) 100%, 0 100%)',
      transition: 'common',
      _after: {
        content: '""',
        position: 'absolute',
        bottom: '0',
        right: '0',
        width: '12px',
        height: '12px',
        background: '[linear-gradient(315deg, rgba(255, 255, 255, 0.3) 50%, rgba(0, 0, 0, 0.08) 50%)]',
        boxShadow: '[1px 1px 2px rgba(0, 0, 0, 0.1)]',
      },
    }),
  )}
  data-related-note-id={note.data.id}
  ondragend={onDragEnd}
  ondragenter={onDragEnter}
  ondragover={(e) => {
    e.preventDefault();
  }}
  role="listitem"
>
  <button
    class={center({
      position: 'absolute',
      top: '8px',
      right: '8px',
      size: '24px',
      borderRadius: '4px',
      color: 'text.faint',
      cursor: 'grab',
      transition: 'common',
      opacity: '0',
      _groupHover: {
        opacity: anyDragging ? '0' : '100',
      },
      _hover: {
        color: 'text.default',
        backgroundColor: 'surface.dark/10',
      },
      _active: {
        cursor: 'grabbing',
      },
    })}
    draggable="true"
    ondragend={onDragEnd}
    ondragstart={(e) => {
      if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text', content || '');

        const noteElement = e.currentTarget.closest('[data-related-note-id]') as HTMLElement;
        if (noteElement) {
          const rect = noteElement.getBoundingClientRect();
          const ghost = document.createElement('div');

          const cloned = noteElement.cloneNode(true) as HTMLElement;
          cloned.style.pointerEvents = 'none';
          cloned.style.transform = 'rotate(1.5deg) scale(1.05)';
          cloned.style.opacity = '0.8';
          cloned.style.width = '100%';
          cloned.style.height = '100%';
          ghost.append(cloned);

          ghost.style.position = 'absolute';
          ghost.style.width = `${rect.width}px`;
          ghost.style.height = `${rect.height}px`;
          ghost.style.minHeight = `${rect.height}px`;
          ghost.style.top = '-1000px';
          ghost.style.left = '-1000px';

          document.body.append(ghost);

          const offsetX = e.clientX - rect.left;
          const offsetY = e.clientY - rect.top;

          e.dataTransfer.setDragImage(ghost, offsetX, offsetY);

          setTimeout(() => {
            ghost.remove();
          });
        }
      }

      onDragStart();
    }}
    type="button"
    use:tooltip={{ message: '드래그해서 순서 변경', placement: 'top', force: anyDragging ? false : undefined }}
  >
    <Icon icon={GripVerticalIcon} size={16} />
  </button>

  <textarea
    class={css({
      width: 'full',
      fontSize: '13px',
      padding: '12px',
      color: 'text.default',
      backgroundColor: 'transparent',
      resize: 'none',
    })}
    onblur={() => {
      focused = false;
      flushContentUpdate();
    }}
    onfocus={() => {
      focused = true;
    }}
    oninput={() => {
      handleContentChanged();
    }}
    onkeydown={(e) => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !e.isComposing) {
        e.preventDefault();
        onAddNote();
      }
    }}
    placeholder="기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요."
    rows={3}
    bind:value={content}
    use:autosize={{ cacheKey: `document-panel-note-${note.data.id}` }}
  ></textarea>

  <Popover
    style={center.raw({
      position: 'absolute',
      bottom: '8px',
      right: '8px',
      size: '24px',
      borderRadius: '4px',
      color: 'text.faint',
      cursor: 'pointer',
      transition: 'common',
      opacity: '0',
      _groupHover: {
        opacity: anyDragging ? '0' : '100',
      },
      _hover: {
        color: 'text.default',
        backgroundColor: 'surface.dark/10',
      },
      _focus: {
        opacity: '100',
        color: 'text.default',
        backgroundColor: 'surface.dark/10',
      },
    })}
    contentStyle={css.raw({ paddingX: '0', paddingY: '0' })}
  >
    {#snippet trigger()}
      <Icon icon={Trash2Icon} size={14} />
    {/snippet}
    {#snippet children({ close })}
      <button
        class={flex({
          alignItems: 'center',
          gap: '8px',
          paddingX: '12px',
          paddingY: '8px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          borderRadius: '6px',
          cursor: 'pointer',
          transition: 'common',
          _hover: {
            backgroundColor: 'accent.danger.subtle',
            color: 'accent.danger.default',
          },
        })}
        onclick={() => {
          handleDeleteNote();
          close();
        }}
        type="button"
      >
        <Icon icon={Trash2Icon} size={14} />
        노트 삭제
      </button>
    {/snippet}
  </Popover>
</div>
