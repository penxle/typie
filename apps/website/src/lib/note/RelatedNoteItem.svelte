<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { onDestroy } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CircleIcon from '~icons/lucide/circle';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getNoteColor } from '$lib/note/note-colors';
  import { noteDrag } from '$lib/note/note-drag.svelte';
  import { getNoteEditsContext } from '$lib/note/note-edit-state.svelte';
  import NoteColorPalette from '$lib/note/NoteColorPalette.svelte';
  import { graphql } from '$mearie';
  import type { NoteListDragPosition } from '$lib/note/NoteList.svelte';
  import type { RelatedNoteItem_note$key } from '$mearie';

  type Props = {
    note$key: RelatedNoteItem_note$key;
    siteId: string;
    cancelling: boolean;
    compact?: boolean;
    dragging: boolean;
    anyDragging: boolean;
    palette?: boolean;
    resolving: boolean;
    reorderEnabled: boolean;
    onAddNote: () => void;
    onDelete: () => void;
    onDragCancel: () => void;
    onDragEnd: () => void;
    onDragMove: (position: NoteListDragPosition) => void;
    onDragStart: (pointer: { clientX: number; clientY: number }) => boolean;
    onColorSaved: (color: string) => void;
    onToggleStatus: () => void;
  };

  let {
    note$key,
    siteId,
    cancelling,
    compact = false,
    dragging,
    anyDragging,
    palette = false,
    resolving,
    reorderEnabled,
    onAddNote,
    onDelete,
    onDragCancel,
    onDragEnd,
    onDragMove,
    onDragStart,
    onColorSaved,
    onToggleStatus,
  }: Props = $props();

  const note = createFragment(
    graphql(`
      fragment RelatedNoteItem_note on Note {
        id
        content
        color
        status
      }
    `),
    () => note$key,
  );

  const noteEdits = getNoteEditsContext();
  const editState = noteEdits.get(note.data.id, { content: note.data.content, color: note.data.color }, siteId);
  const content = $derived(editState?.content ?? note.data.content);
  const color = $derived(editState?.color ?? note.data.color);
  const colorHex = $derived(getNoteColor(color) ?? token('colors.surface.default'));

  onDestroy(() => {
    editState?.flush();
  });

  const displayStatus = $derived(cancelling ? 'OPEN' : resolving ? 'RESOLVED' : note.data.status);

  $effect(() => {
    noteEdits.sync(note.data.id, { content: note.data.content, color: note.data.color });
  });

  const handleDeleteNote = () => {
    if (content.trim() === '') {
      onDelete();
      return;
    }

    Dialog.confirm({
      title: '노트를 삭제하시겠어요?',
      message: '삭제된 노트는 복구할 수 없어요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: onDelete,
    });
  };
</script>

<div class={css({ display: 'grid', minWidth: '0', width: 'full' })}>
  <div
    style:overflow="visible"
    style:opacity={dragging ? '0.5' : '1'}
    style:transition="opacity 180ms ease"
    class={cx(
      'group',
      flex({
        flexDirection: 'column',
        position: 'relative',
        minHeight: '0',
        backgroundColor: 'surface.subtle',
        borderRadius: '8px',
        cursor: reorderEnabled ? 'grab' : 'default',
      }),
    )}
    data-related-note-item-id={note.data.id}
    role="presentation"
    use:noteDrag={{
      disabled: palette,
      dragging,
      onDragStart,
      onDragMove,
      onDragEnd,
      onDragCancel,
    }}
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
          marginTop: compact ? '2px' : '4px',
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
        onclick={onToggleStatus}
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

      <div class={flex({ flexDirection: 'column', flexGrow: '1', minWidth: '0', gap: '3px' })}>
        <textarea
          style:transition={resolving || cancelling ? 'none' : 'text-decoration-color 150ms ease, opacity 150ms ease'}
          class={css({
            width: 'full',
            fontSize: compact ? '13px' : '14px',
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
          onblur={() => editState?.flush()}
          oninput={(event) => editState?.setContent(event.currentTarget.value)}
          onkeydown={(event) => {
            if (!(event.key === 'Enter' && (event.metaKey || event.ctrlKey))) return;
            event.stopPropagation();
            const addNote = () => {
              editState?.flush();
              onAddNote();
            };
            if (event.isComposing) {
              setTimeout(addNote, 0);
              return;
            }
            event.preventDefault();
            addNote();
          }}
          placeholder={displayStatus === 'RESOLVED' ? '(내용 없음)' : '떠오르는 생각을 적어보세요'}
          rows={1}
          value={content}
          use:autosize={{ cacheKey: `${compact ? 'widget' : 'document-panel'}-note-${note.data.id}`, value: content }}></textarea>
      </div>
    </div>

    {#if !palette}
      <!-- ⋯ More button with Menu -->
      <Menu
        style={center.raw({
          position: 'absolute',
          top: compact ? '8px' : '11px',
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
            <li class={css({ padding: '8px' })}>
              <NoteColorPalette
                menuItems
                onchange={(nextColor) => editState?.setColor(nextColor, { onSaved: () => onColorSaved(nextColor) })}
                selectedColor={color}
                size="14px"
              />
            </li>
          </Submenu>
          <MenuItem icon={displayStatus === 'RESOLVED' ? CircleIcon : CircleCheckIcon} onclick={onToggleStatus}>
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
</div>
