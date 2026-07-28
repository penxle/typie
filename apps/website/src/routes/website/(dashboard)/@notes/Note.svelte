<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu, TimeAgo } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { onDestroy, untrack } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CircleIcon from '~icons/lucide/circle';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import LinkIcon from '~icons/lucide/link';
  import Trash2Icon from '~icons/lucide/trash-2';
  import UnlinkIcon from '~icons/lucide/unlink';
  import EntityIcon from '../@context-menu/EntityIcon.svelte';
  import { getNoteColor, noteColors } from './colors';
  import type { NoteReorderDirection, NoteReorderGeometry } from '$lib/note-reorder';
  import type { EntityIcon_entity$key } from '$mearie';

  type NoteDragPosition = {
    clientX: number;
    clientY: number;
    direction: NoteReorderDirection;
    ghost: NoteReorderGeometry;
  };

  type NoteEntity = EntityIcon_entity$key & {
    id: string;
    slug: string;
    node: { __typename: string; id?: string; title?: string; name?: string };
  };

  type CollapseOptions = {
    focusComposer?: boolean;
  };

  type Props = {
    note: {
      id: string;
      content: string;
      color: string;
      status: string;
      updatedAt: string;
      entities: ArrayLike<NoteEntity> & Iterable<NoteEntity>;
    };
    cancelling: boolean;
    expanded: boolean;
    resolving: boolean;
    draggingNoteId: string | null;
    onexpand: (id: string) => void;
    oncollapse: (options?: CollapseOptions) => void;
    ondelete: (id: string) => void;
    ontogglestatus: (id: string) => void;
    onchangecolor: (id: string, color: string) => void;
    onupdatecontent: (id: string, content: string) => void;
    ondragstart: (pointer: { clientX: number; clientY: number }) => boolean;
    ondragend: () => void;
    ondragcancel: () => void;
    ondragmove: (position: NoteDragPosition) => void;
    onaddentity: (noteId: string) => void;
    onremoveentity: (noteId: string, entityId: string) => void;
    onendresolve: (noteId: string) => void;
  };

  let {
    note,
    cancelling,
    expanded,
    resolving,
    draggingNoteId,
    onexpand,
    oncollapse,
    ondelete,
    ontogglestatus,
    onchangecolor,
    onupdatecontent,
    ondragstart,
    ondragend,
    ondragcancel,
    ondragmove,
    onaddentity,
    onremoveentity,
    onendresolve,
  }: Props = $props();

  // Inline editing — DocumentPanelNoteItem pattern
  let content = $state(note.content);
  let focused = $state(false);
  let dirty = $state(false);
  let contentUpdatePending = false;
  let contentUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let textareaEl = $state<HTMLTextAreaElement>();

  const isDragging = $derived(draggingNoteId === note.id);
  const anyDragging = $derived(draggingNoteId !== null);
  const displayStatus = $derived(cancelling ? 'OPEN' : resolving ? 'RESOLVED' : note.status);
  const isResolved = $derived(displayStatus === 'RESOLVED');
  const colorHex = $derived(getNoteColor(note.color) ?? token('colors.surface.default'));

  function getEntityTitle(entity: NoteEntity): string {
    if (entity.node.__typename === 'Document') return entity.node.title || '(제목 없음)';
    if (entity.node.__typename === 'Folder') return entity.node.name || '(이름 없음)';
    return '(제목 없음)';
  }

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
    const serverContent = note.content;
    if (dirty && serverContent === content) {
      dirty = false;
    }
    if (!dirty && !focused) {
      content = serverContent;
    }
  });

  function flushContentUpdate() {
    if (contentUpdateTimeout) {
      clearTimeout(contentUpdateTimeout);
      contentUpdateTimeout = null;
    }
    if (!contentUpdatePending) return;

    contentUpdatePending = false;
    onupdatecontent(note.id, content);
  }

  function handleContentChanged() {
    dirty = true;
    contentUpdatePending = true;
    if (contentUpdateTimeout) clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = setTimeout(flushContentUpdate, 300);
  }

  function handleDeleteNote() {
    if (content.trim() === '') {
      ondelete(note.id);
      return;
    }

    Dialog.confirm({
      title: '노트를 삭제하시겠어요?',
      message: '삭제된 노트는 복구할 수 없어요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: () => ondelete(note.id),
    });
  }

  function completeShortcut() {
    textareaEl?.blur();
    focused = false;
    flushContentUpdate();
    oncollapse({ focusComposer: true });
  }

  function handleEditorShortcut(event: KeyboardEvent) {
    if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;

    event.stopPropagation();
    if (event.isComposing) {
      setTimeout(completeShortcut, 0);
      return;
    }

    event.preventDefault();
    completeShortcut();
  }

  $effect(() => {
    if (!(expanded && textareaEl)) {
      return;
    }

    textareaEl.focus();
    const initialCaretPosition = untrack(() => content.length);
    textareaEl.setSelectionRange(initialCaretPosition, initialCaretPosition);
  });

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
    if (!collapseFinished || note.status !== 'RESOLVED' || !resolving || cancelling) {
      return;
    }

    collapseFinished = false;
    onendresolve(note.id);
  });
</script>

<div
  style:grid-template-rows={resolveCollapsing ? '0fr' : '1fr'}
  style:margin-bottom={resolveCollapsing ? '-8px' : '0px'}
  style:opacity={resolveCollapsing ? '0' : '1'}
  style:transition={cancelling ? 'none' : 'grid-template-rows 180ms ease, margin-bottom 180ms ease, opacity 180ms ease'}
  class={css({ display: 'grid', minWidth: '0', width: 'full' })}
  ontransitionend={handleResolveTransitionEnd}
>
  <div
    style:overflow={resolveCollapsing ? 'hidden' : 'visible'}
    style:opacity={isDragging ? '0.5' : '1'}
    style:transition="opacity 180ms ease, background-color 150ms, box-shadow 150ms, border-color 150ms"
    class={cx(
      'group',
      flex({
        flexDirection: 'column',
        position: 'relative',
        minHeight: '0',
        borderRadius: '10px',
        cursor: 'grab',
        backgroundColor: expanded ? 'surface.default' : 'surface.subtle',
        boxShadow: expanded ? 'large' : 'small',
        borderWidth: '1px',
        borderColor: expanded ? 'border.subtle' : 'transparent',
        _hover: {
          borderColor: 'border.subtle',
        },
      }),
    )}
    data-note-id={note.id}
    onkeydown={(e) => {
      if (expanded) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onexpand(note.id);
      }
    }}
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
          ondragcancel();
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

        ondragmove({
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
          if (!ondragstart({ clientX: ev.clientX, clientY: ev.clientY })) {
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
          ondragend();
        } else if (!expanded) {
          onexpand(note.id);
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
    role="button"
    tabindex="0"
  >
    <div class={flex({ gap: '10px', padding: '12px', paddingBottom: expanded ? '8px' : '12px' })}>
      <!-- Color Checkbox -->
      <button
        style:background-color={isResolved ? colorHex : 'transparent'}
        style:border={isResolved ? 'none' : `1.5px solid ${colorHex}`}
        style:transition={resolving || cancelling ? 'none' : undefined}
        class={center({
          width: '16px',
          height: '16px',
          padding: '0',
          borderRadius: 'full',
          flexShrink: '0',
          marginTop: '3px',
          cursor: 'pointer',
          transition: 'common',
          ...(isResolved && !resolving ? { _hover: { opacity: '60' } } : { _hover: { opacity: '100' } }),
        })}
        onclick={() => ontogglestatus(note.id)}
        type="button"
        use:tooltip={{ message: isResolved ? '미완료로 표시' : '완료로 표시', placement: 'top' }}
      >
        {#if isResolved}
          <Icon style={css.raw({ color: 'surface.default', '& *': { strokeWidth: '[3px]' } })} icon={CheckIcon} size={12} />
        {:else}
          <div
            style:background-color={colorHex}
            class={center({
              width: 'full',
              height: 'full',
              borderRadius: 'full',
              opacity: '0',
              transition: 'common!',
              ':hover > &': { opacity: '100' },
            })}
          >
            <Icon style={css.raw({ color: 'surface.default', '& *': { strokeWidth: '[3px]' } })} icon={CheckIcon} size={12} />
          </div>
        {/if}
      </button>

      <!-- Content -->
      <div class={flex({ flexDirection: 'column', gap: '4px', flexGrow: '1', minWidth: '0' })}>
        {#if expanded}
          <textarea
            bind:this={textareaEl}
            style:transition={resolving || cancelling ? 'none' : 'text-decoration-color 150ms ease, opacity 150ms ease'}
            class={css({
              width: 'full',
              fontSize: '14px',
              paddingRight: '22px',
              color: 'text.default',
              backgroundColor: 'transparent',
              resize: 'none',
              lineHeight: '[1.55]',
              whiteSpace: 'pre-wrap',
              textDecorationLine: isResolved ? 'line-through' : 'none',
              textDecorationColor: isResolved ? 'text.faint' : 'transparent',
              opacity: isResolved && !resolving ? '50' : '100',
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
            onkeydown={handleEditorShortcut}
            placeholder={isResolved ? '(내용 없음)' : '떠오르는 생각을 적어보세요'}
            rows={1}
            bind:value={content}
            use:autosize={{ cacheKey: `note-${note.id}`, value: content }}></textarea>
        {:else}
          <p
            style:transition={resolving || cancelling ? 'none' : 'text-decoration-color 150ms ease, opacity 150ms ease'}
            class={css({
              fontSize: '14px',
              lineHeight: '[1.55]',
              paddingRight: '22px',
              color: note.content.trim() ? 'text.default' : 'text.faint',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
              lineClamp: '3',
              opacity: isResolved && !resolving ? '50' : '100',
              textDecorationLine: isResolved ? 'line-through' : 'none',
              textDecorationColor: isResolved ? 'text.faint' : 'transparent',
            })}
          >
            {note.content.trim() || '(내용 없음)'}
          </p>
        {/if}

        <!-- Meta -->
        <div
          class={css({
            display: 'grid',
            gridTemplateRows: expanded ? '0fr' : '1fr',
            transitionProperty: '[grid-template-rows]',
            transitionDuration: '150ms',
          })}
        >
          <div class={css({ overflow: 'hidden' })}>
            <div class={flex({ alignItems: 'center', gap: '6px', flexWrap: 'wrap' })}>
              {#each note.entities as entity (entity.id)}
                {#if entity.node.__typename === 'Folder'}
                  <span class={flex({ alignItems: 'center', gap: '4px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>
                    <EntityIcon entity$key={entity} fallback={FolderIcon} size={12} />
                    <span class={css({ lineClamp: '1' })}>{getEntityTitle(entity)}</span>
                  </span>
                {:else}
                  <a
                    class={flex({
                      alignItems: 'center',
                      gap: '4px',
                      fontSize: '12px',
                      fontWeight: 'medium',
                      color: 'text.faint',
                      borderRadius: '4px',
                      paddingX: '2px',
                      _hover: { color: 'text.subtle', backgroundColor: 'surface.dark/10' },
                    })}
                    href={`/${entity.slug}`}
                    onclick={(e) => e.stopPropagation()}
                  >
                    <EntityIcon entity$key={entity} size={12} />
                    <span class={css({ lineClamp: '1' })}>{getEntityTitle(entity)}</span>
                  </a>
                {/if}
                <span class={css({ fontSize: '12px', color: 'text.faint' })}>·</span>
              {/each}
              <TimeAgo style={css.raw({ fontSize: '12px', color: 'text.faint' })} timestamp={new Date(note.updatedAt).getTime()} />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Expanded Toolbar -->
    <!-- Expanded Toolbar -->
    <div
      class={css({
        display: 'grid',
        gridTemplateRows: expanded ? '1fr' : '0fr',
        transitionProperty: '[grid-template-rows]',
        transitionDuration: '150ms',
      })}
    >
      <div class={css({ overflow: 'hidden' })}>
        <div
          class={flex({
            flexDirection: 'column',
            gap: '6px',
            paddingLeft: '38px',
            paddingRight: '12px',
            paddingBottom: '10px',
          })}
        >
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <!-- Color dots -->
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              {#each noteColors as c (c.value)}
                <button
                  style:background-color={c.value === note.color ? c.color : 'transparent'}
                  style:border={c.value === note.color ? 'none' : `1.5px solid ${c.color}`}
                  class={center({
                    width: '12px',
                    height: '12px',
                    borderRadius: 'full',
                    cursor: 'pointer',
                    padding: '0',
                  })}
                  aria-label={c.label}
                  onclick={() => onchangecolor(note.id, c.value)}
                  type="button"
                  use:tooltip={{ message: c.label, placement: 'top' }}
                ></button>
              {/each}
            </div>

            <div class={css({ width: '1px', height: '12px', backgroundColor: 'border.subtle' })}></div>

            <button
              class={flex({
                alignItems: 'center',
                gap: '4px',
                fontSize: '12px',
                fontWeight: 'medium',
                color: 'text.subtle',
                cursor: 'pointer',
                flexShrink: '0',
                _hover: { color: 'text.default' },
              })}
              onclick={() => onaddentity(note.id)}
              type="button"
            >
              <Icon icon={LinkIcon} size={12} />
              연결 추가
            </button>
          </div>

          {#if note.entities.length > 0}
            <div class={flex({ alignItems: 'center', gap: '6px', flexWrap: 'wrap' })}>
              {#each note.entities as entity (entity.id)}
                <div class={flex({ alignItems: 'center', gap: '2px', fontSize: '12px', color: 'text.faint', minWidth: '0' })}>
                  <EntityIcon
                    style={css.raw({ flexShrink: '0' })}
                    entity$key={entity}
                    fallback={entity.node.__typename === 'Folder' ? FolderIcon : undefined}
                    size={12}
                  />
                  <span class={css({ lineClamp: '1' })}>{getEntityTitle(entity)}</span>
                  <button
                    class={center({
                      size: '14px',
                      borderRadius: '2px',
                      color: 'text.faint',
                      cursor: 'pointer',
                      flexShrink: '0',
                      _hover: { color: 'text.default' },
                    })}
                    onclick={() => onremoveentity(note.id, entity.id)}
                    type="button"
                    use:tooltip={{ message: '연결 해제', placement: 'top' }}
                  >
                    <Icon icon={UnlinkIcon} size={10} />
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- ⋯ More Menu -->
    <Menu
      style={center.raw({
        position: 'absolute',
        top: '11px',
        right: '8px',
        size: '22px',
        borderRadius: '4px',
        color: 'text.faint',
        cursor: 'pointer',
        transition: 'common!',
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
          {#each noteColors as c (c.value)}
            <MenuItem onclick={() => onchangecolor(note.id, c.value)}>
              {#snippet prefix()}
                <div
                  style:background-color={c.color}
                  class={center({ width: '14px', height: '14px', borderRadius: 'full', flexShrink: '0' })}
                >
                  {#if c.value === note.color}
                    <Icon style={css.raw({ color: 'surface.default' })} icon={CheckIcon} size={10} />
                  {/if}
                </div>
              {/snippet}
              {c.label}
            </MenuItem>
          {/each}
        </Submenu>
        <MenuItem icon={isResolved ? CircleIcon : CircleCheckIcon} onclick={() => ontogglestatus(note.id)}>
          {isResolved ? '미완료로 표시' : '완료로 표시'}
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
