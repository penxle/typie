<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { Icon, Popover } from '@typie/ui/components';
  import { values } from '@typie/ui/tiptap/values-base';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import FileIcon from '~icons/lucide/file';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import Trash2Icon from '~icons/lucide/trash-2';

  type Props = {
    note: {
      id: string;
      content: string;
      color: string;
      entity?: {
        id: string;
        slug: string;
        node: {
          __typename: string;
          id?: string;
          title?: string;
          type?: string;
        };
      } | null;
    };
    onedit: (id: string) => void;
    ondelete: (id: string) => void;
    isDragging?: boolean;
    dropTargetNoteId: string | null;
    ondragstart?: () => void;
    ondragend?: () => void;
    ondragenter?: () => void;
  };

  let { note, onedit, ondelete, isDragging = false, dropTargetNoteId, ondragstart, ondragend, ondragenter }: Props = $props();

  let noteHeight = $state<number | undefined>();
  let draggingNoteMinHeight = $state<number | undefined>();

  const measureHeight = (noteId: string) => {
    const element = document.querySelector(`[data-note-id="${noteId}"]`) as HTMLElement;
    if (element) {
      const height = element.querySelector('[data-note-content-area]')?.scrollHeight || 0;
      const rowSpan = Math.max(Math.ceil(height / 20) + 2, 5);
      return rowSpan;
    }
  };

  $effect(() => {
    void note.content;
    tick().then(() => {
      const height = measureHeight(note.id);
      if (height) {
        noteHeight = height;
      }
    });
  });

  $effect(() => {
    if (dropTargetNoteId && isDragging) {
      const height = measureHeight(dropTargetNoteId);
      if (height) {
        draggingNoteMinHeight = height;
      }
    } else {
      draggingNoteMinHeight = undefined;
    }
  });

  const color = $derived(
    values.textBackgroundColor.find((color) => color.value === note.color)?.color ?? token('colors.prosemirror.white'),
  );
</script>

<div
  style:background-color={`color-mix(in srgb, ${token('colors.prosemirror.white')}, ${color} 75%)`}
  style:grid-row-end={`span ${Math.max(noteHeight || 0, draggingNoteMinHeight || 0) || 'auto'}`}
  class={css({
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
    opacity: isDragging ? '50' : '100',
    padding: '16px',
    paddingBottom: '20px',
    boxShadow: 'small',
    clipPath: 'polygon(0 0, 100% 0, 100% calc(100% - 12px), calc(100% - 12px) 100%, 0 100%)',
    cursor: 'pointer',
    transition: '[opacity 0.2s, transform 0.2s]',
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
  })}
  data-note-id={note.id}
  draggable="true"
  onclick={(e) => {
    const target = e.target as HTMLElement;
    if (!target.closest('button') && !target.closest('a')) {
      onedit(note.id);
    }
  }}
  {ondragend}
  {ondragenter}
  ondragstart={(e) => {
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'copyMove';
      e.dataTransfer.setData('text', note.content);

      const target = e.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();
      const ghost = document.createElement('div');

      const cloned = target.cloneNode(true) as HTMLElement;
      cloned.style.pointerEvents = 'none';
      cloned.style.transform = 'rotate(3deg) scale(1.05)';
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
    ondragstart?.();
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onedit(note.id);
    }
  }}
  role="button"
  tabindex="0"
  in:fly={{ y: -15, duration: 250 }}
>
  <div
    class={flex({
      gap: '4px',
      alignItems: 'center',
      justifyContent: 'space-between',
      position: 'absolute',
      bottom: '8px',
      left: '12px',
      right: '8px',
    })}
  >
    <div>
      {#if note.entity}
        <a
          class={flex({
            alignItems: 'center',
            gap: '4px',
            color: 'text.subtle',
            borderRadius: '4px',
            paddingX: '4px',
            _hover: { color: 'text.default', backgroundColor: 'surface.dark/10' },
            _focus: { color: 'text.default', backgroundColor: 'surface.dark/10' },
          })}
          href={`/${note.entity.slug}`}
        >
          <Icon icon={note.entity.node.__typename === 'Post' ? FileIcon : LineSquiggleIcon} size={12} />
          <span class={css({ fontSize: '12px', fontWeight: 'medium', lineClamp: '1' })}>
            {note.entity.node.__typename === 'Post' || note.entity.node.__typename === 'Canvas'
              ? note.entity.node.title || '(제목 없음)'
              : '(제목 없음)'}
          </span>
        </a>
      {/if}
    </div>
    <div class={flex({ gap: '4px' })}>
      <Popover
        style={center.raw({
          size: '24px',
          borderRadius: '4px',
          cursor: 'pointer',
          transition: 'common',
          color: 'text.subtle',
          _hover: {
            color: 'text.default',
            backgroundColor: 'surface.dark/20',
          },
          _focus: {
            color: 'text.default',
            backgroundColor: 'surface.dark/20',
          },
        })}
        contentStyle={css.raw({ paddingX: '0', paddingY: '0' })}
      >
        {#snippet trigger()}
          <Icon icon={Trash2Icon} size={16} />
        {/snippet}
        {#snippet children({ close })}
          <button
            class={flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '12px',
              paddingY: '8px',
              fontSize: '14px',
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
              ondelete(note.id);
              close();
            }}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
            노트 삭제
          </button>
        {/snippet}
      </Popover>
    </div>
  </div>
  <p
    class={css({
      fontSize: '16px',
      color: 'text.default',
      fontWeight: 'medium',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-word',
    })}
    data-note-content-area
  >
    {note.content}
  </p>
</div>
