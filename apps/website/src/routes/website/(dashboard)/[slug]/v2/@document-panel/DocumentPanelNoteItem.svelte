<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CircleIcon from '~icons/lucide/circle';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getNoteColor, noteColors } from '../../../@notes/colors';
  import { SubscribeModal } from '../../../@subscription/subscribe-modal.svelte';
  import type { NoteReorderDirection, NoteReorderGeometry } from '$lib/note-reorder';
  import type { DocumentPanelV2NoteItem_note$key } from '$mearie';

  type NoteDragPosition = {
    clientX: number;
    clientY: number;
    direction: NoteReorderDirection;
    ghost: NoteReorderGeometry;
  };

  type Props = {
    note$key: DocumentPanelV2NoteItem_note$key;
    draggingNoteId: string | null;
    resolving: boolean;
    onAddNote: () => void;
    onBeginResolve: () => void;
    onDragCancel: () => void;
    onDragEnd: () => void;
    onDragMove: (position: NoteDragPosition) => void;
    onDragStart: () => boolean;
    onEndResolve: () => void;
  };

  let {
    note$key,
    draggingNoteId,
    resolving,
    onAddNote,
    onBeginResolve,
    onDragCancel,
    onDragEnd,
    onDragMove,
    onDragStart,
    onEndResolve,
  }: Props = $props();

  const note = createFragment(
    graphql(`
      fragment DocumentPanelV2NoteItem_note on Note {
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
      mutation DocumentPanelV2NoteItem_UpdateNote_Mutation($input: UpdateNoteInput!) {
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
      mutation DocumentPanelV2NoteItem_DeleteNote_Mutation($input: DeleteNoteInput!) {
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
  const DRAG_DIRECTION_EPSILON = 0.5;
  const RESOLVE_PAUSE_DURATION = 250;

  let cancelDrag: (() => void) | null = null;
  let resolveCollapsing = $state(false);
  let collapseFinished = $state(false);

  onDestroy(() => {
    cancelDrag?.();
  });

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

    if (!SubscribeModal.gate('notes_update')) {
      return;
    }

    if (contentUpdateTimeout) clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = setTimeout(flushContentUpdate, 300);
  }

  let cancelling = $state(false);
  const displayStatus = $derived(cancelling ? 'OPEN' : resolving ? 'RESOLVED' : note.data.status);

  const handleChangeColor = async (color: string) => {
    if (!SubscribeModal.gate('notes_update')) {
      return;
    }

    await updateNote({ input: { noteId: note.data.id, color } });
    mixpanel.track('change_related_note_color', { color });
  };

  const handleToggleStatus = async () => {
    if (cancelling) return;

    if (!SubscribeModal.gate('notes_update')) {
      return;
    }

    if (resolving) {
      cancelling = true;
      try {
        await updateNote({ input: { noteId: note.data.id, status: 'OPEN' } });
        mixpanel.track('toggle_related_note_status', { status: 'OPEN' });
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
      mixpanel.track('toggle_related_note_status', { status: newStatus });
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

  const executeDeleteNote = async () => {
    const entityId = note.data.entity?.id;
    await deleteNote({ input: { noteId: note.data.id } });
    mixpanel.track('delete_related_note');
    if (entityId) {
      cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'notes' });
    }
  };

  const handleDeleteNote = () => {
    if (content.trim() === '') {
      void executeDeleteNote();
      return;
    }

    Dialog.confirm({
      title: '노트를 삭제하시겠어요?',
      message: '삭제된 노트는 복구할 수 없어요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: executeDeleteNote,
    });
  };

  function handleResolveTransitionEnd(event: TransitionEvent) {
    if (
      event.target !== event.currentTarget ||
      event.propertyName !== 'grid-template-rows' ||
      !resolveCollapsing ||
      !resolving ||
      cancelling
    ) {
      return;
    }

    collapseFinished = true;
  }

  $effect(() => {
    collapseFinished = false;
    if (!resolving || cancelling) {
      resolveCollapsing = false;
      return;
    }

    const timeout = setTimeout(() => {
      resolveCollapsing = true;
    }, RESOLVE_PAUSE_DURATION);

    return () => clearTimeout(timeout);
  });

  $effect(() => {
    if (!collapseFinished || note.data.status !== 'RESOLVED' || !resolving || cancelling) {
      return;
    }

    collapseFinished = false;
    onEndResolve();
  });
</script>

<div
  style:grid-template-rows={resolveCollapsing ? '0fr' : '1fr'}
  style:margin-bottom={resolveCollapsing ? '-6px' : '0px'}
  style:opacity={resolveCollapsing ? '0' : '1'}
  style:transition={cancelling ? 'none' : 'grid-template-rows 180ms ease, margin-bottom 180ms ease, opacity 180ms ease'}
  class={css({ display: 'grid', minWidth: '0', width: 'full' })}
  ontransitionend={handleResolveTransitionEnd}
>
  <div
    style:overflow={resolveCollapsing ? 'hidden' : 'visible'}
    style:opacity={isDragging ? '0.5' : '1'}
    style:transition="opacity 180ms ease"
    class={cx(
      'group',
      flex({
        flexDirection: 'column',
        position: 'relative',
        minHeight: '0',
        backgroundColor: 'surface.subtle',
        borderRadius: '8px',
        cursor: 'grab',
      }),
    )}
    data-related-note-id={note.data.id}
    onpointerdown={(e) => {
      if (!e.isPrimary || e.button !== 0) return;
      const target = e.target as HTMLElement;
      if (target.closest('button, textarea, a')) return;

      e.preventDefault();
      const el = e.currentTarget as HTMLElement;
      const rect = el.getBoundingClientRect();
      const pointerId = e.pointerId;
      const pointerCaptureTarget = document.documentElement;

      const state = {
        startX: e.clientX,
        startY: e.clientY,
        offsetX: e.clientX - rect.left,
        offsetY: e.clientY - rect.top,
        started: false,
        ghost: null as HTMLElement | null,
        cursorStyle: null as HTMLStyleElement | null,
        removeEscapeHandler: null as (() => void) | null,
        moveFrame: null as number | null,
        pendingMove: null as PointerEvent | null,
        active: true,
        previousCenterY: rect.top + rect.height / 2,
        direction: 0 as NoteReorderDirection,
      };

      const cleanup = (cancelled = false) => {
        if (!state.active) return;
        const wasStarted = state.started;
        state.active = false;
        if (state.moveFrame !== null) {
          cancelAnimationFrame(state.moveFrame);
          state.moveFrame = null;
        }
        state.pendingMove = null;
        state.ghost?.remove();
        state.cursorStyle?.remove();
        state.removeEscapeHandler?.();
        state.removeEscapeHandler = null;
        document.removeEventListener('pointermove', handleMove);
        document.removeEventListener('pointerup', handleUp);
        document.removeEventListener('pointercancel', handleCancel);
        pointerCaptureTarget.removeEventListener('lostpointercapture', handleLostPointerCapture);
        if (pointerCaptureTarget.hasPointerCapture(pointerId)) {
          pointerCaptureTarget.releasePointerCapture(pointerId);
        }
        cancelDrag = null;
        if (cancelled && wasStarted) {
          onDragCancel();
        }
      };

      const updateGhost = (ev: PointerEvent) => {
        if (!state.ghost) return;

        const top = ev.clientY - state.offsetY;
        const centerY = top + rect.height / 2;
        const centerDeltaY = centerY - state.previousCenterY;

        state.ghost.style.left = `${ev.clientX - state.offsetX}px`;
        state.ghost.style.top = `${top}px`;
        state.previousCenterY = centerY;

        if (Math.abs(centerDeltaY) > DRAG_DIRECTION_EPSILON) {
          state.direction = centerDeltaY < 0 ? -1 : 1;
        }

        onDragMove({
          clientX: ev.clientX,
          clientY: ev.clientY,
          direction: state.direction,
          ghost: {
            top,
            bottom: top + rect.height,
          },
        });
      };

      const processMove = (ev: PointerEvent) => {
        const dist = Math.hypot(ev.clientX - state.startX, ev.clientY - state.startY);
        if (!state.started && dist > DRAG_THRESHOLD) {
          if (!onDragStart()) {
            cleanup();
            return;
          }
          state.started = true;

          const ghost = document.createElement('div');
          const cloned = el.cloneNode(true) as HTMLElement;
          (cloned as unknown as { inert: boolean }).inert = true;
          cloned.setAttribute('aria-hidden', 'true');
          cloned.style.pointerEvents = 'none';
          cloned.style.transform = 'rotate(1.5deg) scale(1.05)';
          cloned.style.opacity = '0.8';
          cloned.style.width = '100%';
          cloned.style.height = '100%';
          ghost.append(cloned);

          ghost.style.position = 'fixed';
          ghost.style.pointerEvents = 'none';
          ghost.style.zIndex = token('zIndex.ghost');
          ghost.style.width = `${rect.width}px`;
          ghost.style.height = `${rect.height}px`;
          ghost.style.left = `${ev.clientX - state.offsetX}px`;
          ghost.style.top = `${ev.clientY - state.offsetY}px`;
          document.body.append(ghost);

          state.ghost = ghost;
          state.cursorStyle = document.createElement('style');
          state.cursorStyle.textContent = '* { cursor: grabbing !important; }';
          document.head.append(state.cursorStyle);
          state.removeEscapeHandler = pushEscapeHandler(() => {
            cleanup(true);
            return true;
          });
        }

        if (state.started && state.ghost) {
          updateGhost(ev);
        }
      };

      const handleMove = (ev: PointerEvent) => {
        if (ev.pointerId !== pointerId) return;

        state.pendingMove = ev;
        if (state.moveFrame !== null) return;

        state.moveFrame = requestAnimationFrame(() => {
          state.moveFrame = null;
          const pendingMove = state.pendingMove;
          state.pendingMove = null;
          if (pendingMove && state.active) {
            processMove(pendingMove);
          }
        });
      };

      const flushPendingMove = () => {
        if (state.moveFrame !== null) {
          cancelAnimationFrame(state.moveFrame);
          state.moveFrame = null;
        }
        const pendingMove = state.pendingMove;
        state.pendingMove = null;
        if (pendingMove && state.active) {
          processMove(pendingMove);
        }
      };

      const handleUp = (ev: PointerEvent) => {
        if (ev.pointerId !== pointerId) return;

        flushPendingMove();
        if (!state.active) return;
        const wasStarted = state.started;
        if (wasStarted) {
          updateGhost(ev);
        }
        cleanup();
        if (wasStarted) {
          onDragEnd();
        }
      };

      const handleCancel = (ev: PointerEvent) => {
        if (ev.pointerId !== pointerId) return;
        cleanup(true);
      };

      const handleLostPointerCapture = (ev: PointerEvent) => {
        if (ev.pointerId !== pointerId) return;
        cleanup(true);
      };

      cancelDrag?.();
      document.addEventListener('pointermove', handleMove);
      document.addEventListener('pointerup', handleUp);
      document.addEventListener('pointercancel', handleCancel);
      pointerCaptureTarget.addEventListener('lostpointercapture', handleLostPointerCapture);
      try {
        pointerCaptureTarget.setPointerCapture(pointerId);
      } catch {
        cleanup();
        return;
      }
      cancelDrag = () => cleanup(true);
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
          marginTop: '4px',
          cursor: 'pointer',
          transition: 'common',
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
        style:transition={resolving || cancelling ? 'none' : 'text-decoration-color 150ms ease, opacity 150ms ease'}
        class={css({
          width: 'full',
          fontSize: '14px',
          paddingRight: '22px',
          color: 'text.default',
          backgroundColor: 'transparent',
          resize: 'none',
          lineHeight: '[1.65]',
          textDecorationLine: displayStatus === 'RESOLVED' ? 'line-through' : 'none',
          textDecorationColor: displayStatus === 'RESOLVED' ? 'text.faint' : 'transparent',
          opacity: displayStatus === 'RESOLVED' && !resolving ? '55' : '100',
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
          if (!(e.key === 'Enter' && (e.metaKey || e.ctrlKey))) return;

          e.stopPropagation();
          if (e.isComposing) {
            setTimeout(onAddNote, 0);
            return;
          }
          e.preventDefault();
          onAddNote();
        }}
        placeholder={displayStatus === 'RESOLVED' ? '(내용 없음)' : '떠오르는 생각을 적어보세요'}
        rows={1}
        bind:value={content}
        use:autosize={{ cacheKey: `document-panel-note-${note.data.id}`, value: content }}></textarea>
    </div>

    <!-- ⋯ More button with Menu -->
    <Menu
      style={center.raw({
        position: 'absolute',
        top: '11px',
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
  </div>
</div>
