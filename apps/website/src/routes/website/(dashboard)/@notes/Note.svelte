<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu, TimeAgo } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { onDestroy } from 'svelte';
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
  import type { EntityIcon_entity$key } from '$mearie';

  type NoteEntity = EntityIcon_entity$key & {
    id: string;
    slug: string;
    node: { __typename: string; id?: string; title?: string; name?: string };
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
    oncollapse: () => void;
    ondelete: (id: string) => void;
    ontogglestatus: (id: string) => void;
    onchangecolor: (id: string, color: string) => void;
    onupdatecontent: (id: string, content: string) => void;
    ondragstart: () => void;
    ondragend: () => void;
    ondragcancel: () => void;
    ondragmove: (noteId: string) => void;
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
  let cancelDrag: (() => void) | null = null;

  onDestroy(() => cancelDrag?.());

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
    if (!contentUpdateTimeout) return;
    clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = null;
    onupdatecontent(note.id, content);
  }

  function handleContentChanged() {
    dirty = true;
    if (contentUpdateTimeout) clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = setTimeout(flushContentUpdate, 300);
  }

  $effect(() => {
    if (!(expanded && textareaEl)) {
      return;
    }

    textareaEl.focus();
    textareaEl.setSelectionRange(content.length, content.length);
  });
</script>

<div
  style:--resolve-duration="500ms"
  style:opacity={isDragging ? '0.5' : resolving && !cancelling ? '0' : '1'}
  style:transition={cancelling
    ? 'none'
    : 'opacity var(--resolve-duration) ease, background-color 150ms, box-shadow 150ms, border-color 150ms'}
  class={cx(
    'group',
    flex({
      flexDirection: 'column',
      position: 'relative',
      borderRadius: '10px',
      cursor: expanded ? 'default' : 'grab',
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
    if (expanded || !e.isPrimary || e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, textarea, a')) return;

    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const pointerId = e.pointerId;

    const state = {
      startX: e.clientX,
      startY: e.clientY,
      offsetX: e.clientX - rect.left,
      offsetY: e.clientY - rect.top,
      started: false,
      ghost: null as HTMLElement | null,
      cursorStyle: null as HTMLStyleElement | null,
      active: true,
    };

    const cleanup = (cancelled = false) => {
      if (!state.active) return;
      const wasStarted = state.started;
      state.active = false;
      state.ghost?.remove();
      state.cursorStyle?.remove();
      document.removeEventListener('pointermove', handleMove);
      document.removeEventListener('pointerup', handleUp);
      document.removeEventListener('pointercancel', handleCancel);
      cancelDrag = null;
      if (cancelled && wasStarted) {
        ondragcancel();
      }
    };

    const handleMove = (ev: PointerEvent) => {
      if (ev.pointerId !== pointerId) return;

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
        ondragstart();
      }

      if (state.started && state.ghost) {
        state.ghost.style.left = `${ev.clientX - state.offsetX}px`;
        state.ghost.style.top = `${ev.clientY - state.offsetY}px`;

        const elemBelow = document.elementFromPoint(ev.clientX, ev.clientY);
        const noteBelow = elemBelow?.closest('[data-note-id]') as HTMLElement | null;
        if (noteBelow) {
          const noteId = noteBelow.dataset.noteId;
          if (noteId && noteId !== note.id) {
            ondragmove(noteId);
          }
        }
      }
    };

    const handleUp = (ev: PointerEvent) => {
      if (ev.pointerId !== pointerId) return;

      const wasStarted = state.started;
      cleanup();
      if (wasStarted) {
        ondragend();
      } else {
        onexpand(note.id);
      }
    };

    const handleCancel = (ev: PointerEvent) => {
      if (ev.pointerId !== pointerId) return;
      cleanup(true);
    };

    cancelDrag?.();
    document.addEventListener('pointermove', handleMove);
    document.addEventListener('pointerup', handleUp);
    document.addEventListener('pointercancel', handleCancel);
    cancelDrag = () => cleanup(true);
  }}
  ontransitionend={(e) => {
    if (e.target === e.currentTarget && e.propertyName === 'opacity' && resolving && !cancelling) {
      onendresolve(note.id);
    }
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
        marginTop: '2px',
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
          class={css({
            width: 'full',
            fontSize: '13px',
            paddingRight: '22px',
            color: 'text.default',
            backgroundColor: 'transparent',
            resize: 'none',
            lineHeight: '[1.55]',
            whiteSpace: 'pre-wrap',
          })}
          onblur={() => {
            focused = false;
            flushContentUpdate();
          }}
          onfocus={() => {
            focused = true;
          }}
          oninput={() => handleContentChanged()}
          onkeydown={(e) => {
            if (!(e.key === 'Enter' && (e.metaKey || e.ctrlKey))) {
              return;
            }

            e.preventDefault();
            e.stopPropagation();
            flushContentUpdate();
            oncollapse();
          }}
          rows={1}
          bind:value={content}
          use:autosize={{ cacheKey: `note-${note.id}` }}></textarea>
      {:else}
        <p
          style:transition={cancelling
            ? 'none'
            : 'text-decoration-color var(--resolve-duration) ease, opacity var(--resolve-duration) ease'}
          class={css({
            fontSize: '13px',
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
          Dialog.confirm({
            title: '노트를 삭제하시겠어요?',
            message: '삭제된 노트는 복구할 수 없어요.',
            action: 'danger',
            actionLabel: '삭제',
            actionHandler: () => ondelete(note.id),
          });
        }}
        variant="danger"
      >
        삭제
      </MenuItem>
    {/snippet}
  </Menu>
</div>
