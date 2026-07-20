<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import CircleIcon from '~icons/lucide/circle';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getNoteColor, noteColors } from '../../@notes/colors';
  import type { DocumentRelatedNoteWidgetItem_note$key } from '$mearie';

  type Props = {
    note$key: DocumentRelatedNoteWidgetItem_note$key;
    draggingNoteId: string | null;
    palette?: boolean;
    resolving: boolean;
    onAddNote: () => void;
    onBeginResolve: () => void;
    onDragEnd: (clientX: number, clientY: number) => void;
    onDragEnter: () => void;
    onDragMove: (clientX: number, clientY: number) => void;
    onDragStart: () => void;
    onEndResolve: () => void;
  };

  let {
    note$key,
    draggingNoteId,
    palette = false,
    resolving,
    onAddNote,
    onBeginResolve,
    onDragEnd,
    onDragEnter,
    onDragMove,
    onDragStart,
    onEndResolve,
  }: Props = $props();

  const note = createFragment(
    graphql(`
      fragment DocumentRelatedNoteWidgetItem_note on Note {
        id
        content
        color
        status
        entity {
          id
        }
      }
    `),
    () => note$key,
  );

  const [updateNote] = createMutation(
    graphql(`
      mutation DocumentRelatedNoteWidgetItem_UpdateNote_Mutation($input: UpdateNoteInput!) {
        updateNote(input: $input) {
          id
          content
          color
          status
          updatedAt
        }
      }
    `),
  );

  const [deleteNote] = createMutation(
    graphql(`
      mutation DocumentRelatedNoteWidgetItem_DeleteNote_Mutation($input: DeleteNoteInput!) {
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
  const colorHex = $derived(getNoteColor(note.data.color) ?? token('colors.surface.default'));

  const DRAG_THRESHOLD = 5;
  let dragCleanup: (() => void) | null = null;

  let cancelling = $state(false);
  const displayStatus = $derived(cancelling ? 'OPEN' : resolving ? 'RESOLVED' : note.data.status);

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

  const handleChangeColor = async (color: string) => {
    await updateNote({ input: { noteId: note.data.id, color } });
    mixpanel.track('change_related_note_color', { color });
  };

  const handleToggleStatus = async () => {
    if (cancelling) return;

    if (resolving) {
      cancelling = true;
      try {
        await updateNote({ input: { noteId: note.data.id, status: 'OPEN' } });
        mixpanel.track('toggle_widget_note_status', { status: 'OPEN' });
      } catch {
        cancelling = false;
        return;
      }
      cancelling = false;
      onEndResolve();
      return;
    }

    const newStatus = note.data.status === 'OPEN' ? 'RESOLVED' : 'OPEN';

    if (newStatus === 'RESOLVED') {
      onBeginResolve();
    }

    try {
      await updateNote({ input: { noteId: note.data.id, status: newStatus } });
      mixpanel.track('toggle_widget_note_status', { status: newStatus });
    } catch {
      if (newStatus === 'RESOLVED') {
        onEndResolve();
      }
      return;
    }

    if (newStatus === 'OPEN') {
      const entityId = note.data.entity?.id;
      if (entityId) {
        cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'notes' });
      }
    }
  };

  const handleDeleteNote = () => {
    Dialog.confirm({
      title: '노트를 삭제하시겠어요?',
      message: '삭제된 노트는 복구할 수 없어요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        const entityId = note.data.entity?.id;
        await deleteNote({ input: { noteId: note.data.id } });
        mixpanel.track('delete_related_note');
        if (entityId) {
          cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'notes' });
        }
      },
    });
  };
</script>

<div
  style:--resolve-duration="500ms"
  style:opacity={isDragging ? '0.5' : resolving && !cancelling ? '0' : '1'}
  style:transition={cancelling ? 'none' : 'opacity var(--resolve-duration) ease'}
  class={cx(
    'group',
    flex({
      flexDirection: 'column',
      position: 'relative',
      backgroundColor: 'surface.subtle',
      borderRadius: '8px',
      cursor: 'grab',
    }),
  )}
  data-widget-note-id={note.data.id}
  ondragenter={onDragEnter}
  ondragover={(e) => {
    e.preventDefault();
  }}
  onpointerdown={(e) => {
    if (palette) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, textarea')) return;

    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();

    const state = {
      startX: e.clientX,
      startY: e.clientY,
      offsetX: e.clientX - rect.left,
      offsetY: e.clientY - rect.top,
      started: false,
      ghost: null as HTMLElement | null,
      cursorStyle: null as HTMLStyleElement | null,
    };

    const cleanup = () => {
      state.ghost?.remove();
      state.cursorStyle?.remove();
      document.removeEventListener('pointermove', handleMove);
      document.removeEventListener('pointerup', handleUp);
      dragCleanup = null;
    };

    const handleMove = (ev: PointerEvent) => {
      const dist = Math.abs(ev.clientX - state.startX) + Math.abs(ev.clientY - state.startY);

      if (!state.started && dist > DRAG_THRESHOLD) {
        state.started = true;

        const ghost = document.createElement('div');
        const cloned = el.cloneNode(true) as HTMLElement;
        cloned.style.pointerEvents = 'none';
        cloned.style.transform = 'rotate(1.5deg) scale(1.05)';
        cloned.style.opacity = '0.8';
        cloned.style.width = '100%';
        cloned.style.height = '100%';
        ghost.append(cloned);

        ghost.style.position = 'fixed';
        ghost.style.pointerEvents = 'none';
        ghost.style.zIndex = '9999';
        ghost.style.width = `${rect.width}px`;
        ghost.style.left = `${ev.clientX - state.offsetX}px`;
        ghost.style.top = `${ev.clientY - state.offsetY}px`;
        document.body.append(ghost);

        state.ghost = ghost;
        state.cursorStyle = document.createElement('style');
        state.cursorStyle.textContent = '* { cursor: grabbing !important; }';
        document.head.append(state.cursorStyle);
        onDragStart();
      }

      if (state.started && state.ghost) {
        state.ghost.style.left = `${ev.clientX - state.offsetX}px`;
        state.ghost.style.top = `${ev.clientY - state.offsetY}px`;
        onDragMove(ev.clientX, ev.clientY);
      }
    };

    const handleUp = (ev: PointerEvent) => {
      if (state.started) {
        onDragEnd(ev.clientX, ev.clientY);
      }
      cleanup();
    };

    dragCleanup?.();
    document.addEventListener('pointermove', handleMove);
    document.addEventListener('pointerup', handleUp);
    dragCleanup = cleanup;
  }}
  ontransitionend={(e) => {
    if (e.target === e.currentTarget && e.propertyName === 'opacity' && resolving && !cancelling) {
      onEndResolve();
    }
  }}
  role="listitem"
>
  <div class={flex({ gap: '10px', padding: '12px' })}>
    <!-- Color Checkbox -->
    <button
      style:background-color={displayStatus === 'RESOLVED' ? colorHex : 'transparent'}
      style:border={displayStatus === 'RESOLVED' ? 'none' : `1.5px solid ${colorHex}`}
      style:transition={resolving || cancelling ? 'none' : undefined}
      class={center({
        width: '16px',
        height: '16px',
        padding: '0',
        borderRadius: 'full',
        flexShrink: '0',
        marginTop: '2px',
        cursor: palette ? 'default' : 'pointer',
        transition: 'common',
        pointerEvents: palette ? 'none' : 'auto',
        ...(displayStatus === 'RESOLVED' && !resolving
          ? {
              _hover: { opacity: '60' },
            }
          : {
              _hover: { opacity: '100' },
            }),
      })}
      onclick={handleToggleStatus}
      type="button"
      use:tooltip={{ message: displayStatus === 'RESOLVED' ? '미완료로 표시' : '완료로 표시', placement: 'top' }}
    >
      {#if displayStatus === 'RESOLVED'}
        <Icon style={css.raw({ color: 'surface.default', '& *': { strokeWidth: '[3px]' } })} icon={CheckIcon} size={12} />
      {:else}
        <div
          style:background-color={colorHex}
          class={center({
            width: 'full',
            height: 'full',
            borderRadius: 'full',
            opacity: '0',
            transition: 'common',
            ':hover > &': { opacity: '100' },
          })}
        >
          <Icon style={css.raw({ color: 'surface.default', '& *': { strokeWidth: '[3px]' } })} icon={CheckIcon} size={12} />
        </div>
      {/if}
    </button>

    <!-- Textarea -->
    <textarea
      style:transition={cancelling ? 'none' : 'text-decoration-color var(--resolve-duration) ease, opacity var(--resolve-duration) ease'}
      class={css({
        width: 'full',
        fontSize: '13px',
        paddingRight: '22px',
        color: 'text.default',
        backgroundColor: 'transparent',
        resize: 'none',
        lineHeight: '[1.65]',
        textDecorationLine: displayStatus === 'RESOLVED' ? 'line-through' : 'none',
        textDecorationColor: displayStatus === 'RESOLVED' ? 'text.faint' : 'transparent',
        opacity: displayStatus === 'RESOLVED' && !resolving ? '55' : '100',
      })}
      disabled={palette}
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
        if (!(e.key === 'Enter' && (e.metaKey || e.ctrlKey)) || e.isComposing) {
          return;
        }

        e.preventDefault();
        onAddNote();
      }}
      placeholder="기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요."
      rows={1}
      bind:value={content}
      use:autosize={{ cacheKey: `widget-note-${note.data.id}` }}></textarea>
  </div>

  {#if !palette}
    <!-- ⋯ More button with Menu -->
    <Menu
      style={center.raw({
        position: 'absolute',
        top: '8px',
        right: '8px',
        size: '22px',
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
        _focusVisible: {
          opacity: '100',
          color: 'text.default',
          backgroundColor: 'surface.dark/10',
        },
        '&[aria-expanded="true"]': {
          opacity: '100',
          color: 'text.default',
          backgroundColor: 'surface.dark/10',
        },
      })}
      placement="bottom-end"
    >
      {#snippet button()}
        <Icon icon={EllipsisIcon} size={14} />
      {/snippet}
      {#snippet children({ close })}
        <Submenu label="색 바꾸기" listStyle={css.raw({ minWidth: '100px' })}>
          {#snippet prefix()}
            <div
              style:background-color={colorHex}
              class={css({ width: '14px', height: '14px', borderRadius: 'full', flexShrink: '0' })}
            ></div>
          {/snippet}
          {#each noteColors as noteColorOption (noteColorOption.value)}
            <MenuItem onclick={() => handleChangeColor(noteColorOption.value)}>
              {#snippet prefix()}
                <div
                  style:background-color={noteColorOption.color}
                  class={center({ width: '14px', height: '14px', borderRadius: 'full', flexShrink: '0' })}
                >
                  {#if noteColorOption.value === note.data.color}
                    <Icon style={css.raw({ color: 'surface.default' })} icon={CheckIcon} size={10} />
                  {/if}
                </div>
              {/snippet}
              {noteColorOption.label}
            </MenuItem>
          {/each}
        </Submenu>
        <MenuItem icon={displayStatus === 'RESOLVED' ? CircleIcon : CircleCheckIcon} onclick={handleToggleStatus}>
          {displayStatus === 'RESOLVED' ? '미완료로 표시' : '완료로 표시'}
        </MenuItem>
        <HorizontalDivider />
        <MenuItem
          icon={Trash2Icon}
          onclick={() => {
            close();
            handleDeleteNote();
          }}
          variant="danger"
        >
          삭제
        </MenuItem>
      {/snippet}
    </Menu>
  {/if}
</div>
