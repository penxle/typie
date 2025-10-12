<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Icon, Popover } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { animateFlip, debounce, getNoteColors, getRandomNoteColor, handleDragScroll } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import ExpandIcon from '~icons/lucide/expand';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { fragment, graphql } from '$graphql';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';

  type Props = {
    widgetId: string;
    disabled?: boolean;
    data?: Record<string, unknown>;
  };

  let { widgetId, disabled = false, data = {} }: Props = $props();

  const widgetContext = getWidgetContext();
  const { palette, $post: _post } = $derived(widgetContext.env);

  const post = fragment(
    // eslint-disable-next-line svelte/no-unused-svelte-ignore
    // svelte-ignore state_referenced_locally
    _post,
    graphql(`
      fragment Editor_Widget_PostRelatedNoteWidget_post on Post {
        id

        entity {
          id
          notes {
            id
            content
            color
            order
            createdAt
            updatedAt
            entity {
              id
            }
          }
        }
      }
    `),
  );

  const createNote = graphql(`
    mutation Editor_Widget_PostRelatedNoteWidget_CreateNote_Mutation($input: CreateNoteInput!) {
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
  `);

  const updateNote = graphql(`
    mutation Editor_Widget_PostRelatedNoteWidget_UpdateNote_Mutation($input: UpdateNoteInput!) {
      updateNote(input: $input) {
        id
        content
        updatedAt
      }
    }
  `);

  const deleteNote = graphql(`
    mutation Editor_Widget_PostRelatedNoteWidget_DeleteNote_Mutation($input: DeleteNoteInput!) {
      deleteNote(input: $input) {
        id
      }
    }
  `);

  const moveNote = graphql(`
    mutation Editor_Widget_PostRelatedNoteWidget_MoveNote_Mutation($input: MoveNoteInput!) {
      moveNote(input: $input) {
        id
        order
      }
    }
  `);

  let dragging = $state<{
    noteId: string;
    originalIndex: number;
  } | null>(null);
  let localNoteOrder = $state<string[]>([]);
  let scrollContainer = $state<HTMLElement | null>(null);
  let isExpanded = $state((data.isExpanded as boolean) ?? false);

  const toggleExpanded = () => {
    isExpanded = !isExpanded;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded });
  };

  const sortedNotes = $derived.by(() => {
    if (!$post) return [];
    const notes = $post.entity.notes;
    if (localNoteOrder.length === 0) {
      return notes.toSorted((a, b) => a.order.localeCompare(b.order));
    }
    return [...notes].toSorted((a, b) => {
      const indexA = localNoteOrder.indexOf(a.id);
      const indexB = localNoteOrder.indexOf(b.id);
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  });

  const notes = $derived(sortedNotes);

  let noteContents = $state<Record<string, string>>({});
  let noteLocalUpdatedAt = $state<Record<string, Date>>({});

  let lastAddedNoteId = $state<string>();

  $effect(() => {
    if (notes) {
      notes.forEach((note) => {
        const updatedAt = dayjs(note.updatedAt);
        if (!noteLocalUpdatedAt[note.id] || updatedAt.isAfter(dayjs(noteLocalUpdatedAt[note.id]))) {
          noteContents[note.id] = note.content;
          noteLocalUpdatedAt[note.id] = updatedAt.toDate();
        }
      });
    }
  });

  const saveNote = debounce(async (noteId: string, content: string) => {
    await updateNote({
      noteId,
      content,
    });
  }, 500);

  const handleNoteChange = (noteId: string, value: string) => {
    noteContents[noteId] = value;
    noteLocalUpdatedAt[noteId] = new Date();
    saveNote(noteId, value);
  };

  const handleAddNote = async (via: string) => {
    if (!$post?.entity.id) return;

    const randomColor = getRandomNoteColor();
    const result = await createNote({
      content: '',
      color: randomColor,
      entityId: $post.entity.id,
    });

    if (result?.id) {
      lastAddedNoteId = result.id;
      mixpanel.track('create_related_note', {
        via,
      });
      cache.invalidate({ __typename: 'Entity', id: $post.entity.id, field: 'notes' });
    }
  };

  const handleDeleteNote = async (noteId: string) => {
    if (!$post?.entity.id) return;

    await deleteNote({ noteId });
    mixpanel.track('delete_related_note');
    cache.invalidate({ __typename: 'Entity', id: $post.entity.id, field: 'notes' });
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
    if (!dragging || !$post?.entity.id) return;

    const currentIndex = localNoteOrder.indexOf(dragging.noteId);

    if (currentIndex !== -1 && dragging.originalIndex !== -1 && currentIndex !== dragging.originalIndex && sortedNotes.length > 1) {
      const lowerNote = sortedNotes[currentIndex - 1] ?? null;
      const upperNote = sortedNotes[currentIndex + 1] ?? null;

      try {
        await moveNote({
          noteId: dragging.noteId,
          lowerOrder: lowerNote?.order,
          upperOrder: upperNote?.order,
        });
        mixpanel.track('move_related_note');
        cache.invalidate({ __typename: 'Entity', id: $post.entity.id, field: 'notes' });
      } catch {
        localNoteOrder = $post.entity.notes.map((note) => note.id);
        Toast.error('노트 순서 변경에 실패했습니다. 잠시 후 다시 시도해주세요.');
      }
    }

    dragging = null;
  };

  let prevNoteIds = $state<string[]>([]);
  $effect(() => {
    const noteIds = $post?.entity.notes.map((n) => n.id) ?? [];
    const noteIdsStr = noteIds.join(',');
    const prevNoteIdsStr = prevNoteIds.join(',');

    if (noteIdsStr !== prevNoteIdsStr) {
      prevNoteIds = noteIds;
      localNoteOrder = noteIds;
    }

    const noteElement = document.querySelector(`[data-widget-note-id="${lastAddedNoteId}"] textarea`) as HTMLTextAreaElement;
    if (noteElement) {
      noteElement.focus();
      lastAddedNoteId = undefined;
    }
  });

  $effect(() => {
    if (!palette) {
      return handleDragScroll(scrollContainer, !!dragging);
    }
  });

  $effect.pre(() => {
    void localNoteOrder;
    if (!scrollContainer || palette) return;

    animateFlip('[data-widget-note-id]', 'widgetNoteId', scrollContainer);
  });
</script>

<Widget {disabled} icon={StickyNoteIcon} noPadding title="이 포스트 관련 노트" {widgetId} widgetType="postRelatedNote">
  {#snippet headerActions()}
    {#if !palette}
      <button
        class={center({
          size: '20px',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
          cursor: 'pointer',
        })}
        onclick={(e) => {
          e.stopPropagation();
          handleAddNote('button');
        }}
        onpointerdown={(e) => {
          e.stopPropagation();
        }}
        type="button"
      >
        <Icon icon={PlusIcon} size={14} />
      </button>
      <button
        class={center({
          size: '20px',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
          cursor: 'pointer',
        })}
        onclick={(e) => {
          e.stopPropagation();
          toggleExpanded();
        }}
        onpointerdown={(e) => {
          e.stopPropagation();
        }}
        type="button"
        use:tooltip={{ message: isExpanded ? '크기 제한' : '크기 제한 해제', placement: 'top' }}
      >
        <Icon icon={isExpanded ? Minimize2Icon : ExpandIcon} size={14} />
      </button>
    {/if}
  {/snippet}

  <div
    bind:this={scrollContainer}
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      maxHeight: isExpanded ? undefined : '400px',
      overflowY: 'auto',
      padding: '8px',
      paddingRight: '4px',
    })}
  >
    {#if notes.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          paddingY: '24px',
        })}
      >
        <div
          class={center({
            size: '48px',
            borderRadius: '12px',
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={StickyNoteIcon} size={20} />
        </div>

        <p class={css({ fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>
          떠오르는 생각이나 아이디어를
          <br />
          자유롭게 기록해보세요
        </p>
      </div>
    {:else}
      {#each notes as note (note.id)}
        {@const color = getNoteColors().find((color) => color.value === note.color)?.color ?? token('colors.prosemirror.white')}
        {@const isDragging = dragging?.noteId === note.id}
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
          data-widget-note-id={note.id}
          draggable={palette ? 'false' : 'true'}
          ondragend={handleDragEnd}
          ondragenter={() => handleDragEnter(note.id)}
          ondragover={(e) => {
            e.preventDefault();
          }}
          ondragstart={(e) => {
            if (palette) {
              e.preventDefault();
              return;
            }

            const target = e.target as HTMLElement;
            if (target.tagName === 'TEXTAREA') {
              e.preventDefault();
              return;
            }

            if (e.dataTransfer) {
              e.dataTransfer.effectAllowed = 'move';
              e.dataTransfer.setData('text', noteContents[note.id] || '');

              const currentTarget = e.currentTarget as HTMLElement;
              const rect = currentTarget.getBoundingClientRect();
              const ghost = document.createElement('div');

              const cloned = currentTarget.cloneNode(true) as HTMLElement;
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

            handleDragStart(note.id);
          }}
          role="listitem"
        >
          <textarea
            class={css({
              width: 'full',
              fontSize: '13px',
              padding: '12px',
              color: 'text.default',
              backgroundColor: 'transparent',
              resize: 'none',
            })}
            disabled={palette}
            oninput={(e) => handleNoteChange(note.id, e.currentTarget.value)}
            onkeydown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !e.isComposing) {
                e.preventDefault();
                handleAddNote('shortcut');
              }
            }}
            placeholder="기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요."
            rows={3}
            value={noteContents[note.id] || ''}
            use:autosize
          ></textarea>

          {#if !palette}
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
                  opacity: '100',
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
                    handleDeleteNote(note.id);
                    close();
                  }}
                  type="button"
                >
                  <Icon icon={Trash2Icon} size={14} />
                  노트 삭제
                </button>
              {/snippet}
            </Popover>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</Widget>
