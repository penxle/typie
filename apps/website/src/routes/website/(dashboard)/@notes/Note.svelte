<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu, TimeAgo } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { onDestroy, untrack } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CircleIcon from '~icons/lucide/circle';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import LinkIcon from '~icons/lucide/link';
  import Trash2Icon from '~icons/lucide/trash-2';
  import UnlinkIcon from '~icons/lucide/unlink';
  import { getNoteColor, noteColors } from '$lib/note/note-colors';
  import { noteDrag } from '$lib/note/note-drag.svelte';
  import { getNoteEditsContext } from '$lib/note/note-edit-state.svelte';
  import NoteColorPalette from '$lib/note/NoteColorPalette.svelte';
  import EntityIcon from '../@context-menu/EntityIcon.svelte';
  import type { NoteListDragPosition } from '$lib/note/NoteList.svelte';
  import type { EntityIcon_entity$key } from '$mearie';

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
      site: { id: string };
      entities: ArrayLike<NoteEntity> & Iterable<NoteEntity>;
    };
    cancelling: boolean;
    dragging: boolean;
    anyDragging: boolean;
    expanded: boolean;
    resolving: boolean;
    reorderEnabled: boolean;
    onexpand: (id: string) => void;
    oncollapse: (options?: CollapseOptions) => void;
    ondelete: (id: string) => void;
    ontogglestatus: (id: string) => void;
    ondragstart: (pointer: { clientX: number; clientY: number }) => boolean;
    ondragend: () => void;
    ondragcancel: () => void;
    ondragmove: (position: NoteListDragPosition) => void;
    onaddentity: (noteId: string) => void;
    onremoveentity: (noteId: string, entityId: string) => void;
    onColorSaved: (color: string) => void;
  };

  let {
    note,
    cancelling,
    dragging,
    anyDragging,
    expanded,
    resolving,
    reorderEnabled,
    onexpand,
    oncollapse,
    ondelete,
    ontogglestatus,
    ondragstart,
    ondragend,
    ondragcancel,
    ondragmove,
    onaddentity,
    onremoveentity,
    onColorSaved,
  }: Props = $props();

  const noteEdits = getNoteEditsContext();
  const editState = noteEdits.get(note.id, { content: note.content, color: note.color }, note.site.id);
  let textareaEl = $state<HTMLTextAreaElement>();

  const content = $derived(editState?.content ?? note.content);
  const color = $derived(editState?.color ?? note.color);
  const displayStatus = $derived(cancelling ? 'OPEN' : resolving ? 'RESOLVED' : note.status);
  const isResolved = $derived(displayStatus === 'RESOLVED');
  const colorHex = $derived(getNoteColor(color) ?? token('colors.surface.default'));

  function getEntityTitle(entity: NoteEntity): string {
    if (entity.node.__typename === 'Document') return entity.node.title || '(제목 없음)';
    if (entity.node.__typename === 'Folder') return entity.node.name || '(이름 없음)';
    return '(제목 없음)';
  }

  onDestroy(() => {
    editState?.flush();
  });

  $effect(() => {
    const noteId = note.id;
    const snapshot = { content: note.content, color: note.color };
    noteEdits.sync(noteId, snapshot);
  });

  $effect(() => {
    if (!expanded) untrack(() => editState?.flush());
  });

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
    editState?.flush();
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
</script>

<div class={css({ display: 'grid', minWidth: '0', width: 'full' })}>
  <div
    style:overflow="visible"
    style:opacity={dragging ? '0.5' : '1'}
    style:transition="opacity 180ms ease, background-color 150ms, box-shadow 150ms, border-color 150ms"
    class={cx(
      'group',
      flex({
        flexDirection: 'column',
        position: 'relative',
        minHeight: '0',
        borderRadius: '10px',
        cursor: reorderEnabled ? 'grab' : 'default',
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
    role="button"
    tabindex="0"
    use:noteDrag={{
      dragging,
      onDragStart: ondragstart,
      onDragMove: ondragmove,
      onDragEnd: ondragend,
      onDragCancel: ondragcancel,
      onPress: expanded ? undefined : () => onexpand(note.id),
    }}
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
              editState?.flush();
            }}
            oninput={(event) => {
              editState?.setContent(event.currentTarget.value);
            }}
            onkeydown={handleEditorShortcut}
            placeholder={isResolved ? '(내용 없음)' : '떠오르는 생각을 적어보세요'}
            rows={1}
            value={content}
            use:autosize={{ cacheKey: `note-${note.id}`, value: content }}></textarea>
        {:else}
          <p
            style:transition={resolving || cancelling ? 'none' : 'text-decoration-color 150ms ease, opacity 150ms ease'}
            class={css({
              fontSize: '14px',
              lineHeight: '[1.55]',
              paddingRight: '22px',
              color: content.trim() ? 'text.default' : 'text.faint',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
              lineClamp: '3',
              opacity: isResolved && !resolving ? '50' : '100',
              textDecorationLine: isResolved ? 'line-through' : 'none',
              textDecorationColor: isResolved ? 'text.faint' : 'transparent',
            })}
          >
            {content.trim() || '(내용 없음)'}
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
          <div class={flex({ alignItems: 'center', gap: '8px', minWidth: '0', width: 'full' })}>
            <!-- Color dots -->
            <NoteColorPalette
              onchange={(nextColor) => editState?.setColor(nextColor, { onSaved: () => onColorSaved(nextColor) })}
              selectedColor={color}
              size="12px"
            />

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

            <span
              class={flex({
                alignItems: 'center',
                justifyContent: 'flex-end',
                flex: '1',
                minWidth: '0',
                overflow: 'hidden',
                fontSize: '11px',
                color: 'text.faint',
                whiteSpace: 'nowrap',
              })}
              aria-atomic="true"
              aria-live="polite"
            >
              {#if editState?.saveDisplay === 'saving'}
                저장 중...
              {:else if editState?.saveDisplay === 'failed'}
                <span class={css({ color: 'text.danger' })}>저장 실패</span>
              {/if}
            </span>
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
            <MenuItem onclick={() => editState?.setColor(c.value, { onSaved: () => onColorSaved(c.value) })}>
              {#snippet prefix()}
                <div
                  style:background-color={c.color}
                  class={center({ width: '14px', height: '14px', borderRadius: 'full', flexShrink: '0' })}
                >
                  {#if c.value === color}
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
