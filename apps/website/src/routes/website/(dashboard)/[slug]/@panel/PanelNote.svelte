<script lang="ts">
  import { TinyColor } from '@ctrl/tinycolor';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { values } from '@typie/ui/tiptap/values-base';
  import { debounce } from '@typie/ui/utils';
  import ExpandIcon from '~icons/lucide/expand';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import PlusIcon from '~icons/lucide/plus';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$graphql';

  type Props = {
    entityId: string;
  };

  let { entityId }: Props = $props();

  const notesQuery = graphql(`
    query PanelNote_Notes_Query($entityId: ID!) @client {
      notes(entityId: $entityId) {
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
  `);

  const createNote = graphql(`
    mutation PanelNote_CreateNote_Mutation($input: CreateNoteInput!) {
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
    mutation PanelNote_UpdateNote_Mutation($input: UpdateNoteInput!) {
      updateNote(input: $input) {
        id
        content
      }
    }
  `);

  const deleteNote = graphql(`
    mutation PanelNote_DeleteNote_Mutation($input: DeleteNoteInput!) {
      deleteNote(input: $input) {
        id
      }
    }
  `);

  const app = getAppContext();
  const colors = values.textBackgroundColor.filter((color) => color.value !== 'none').map((color) => color.hex);

  const isLoading = $derived(!$notesQuery);
  const notes = $derived($notesQuery?.notes?.sort((a, b) => a.order.localeCompare(b.order)) || []);

  let noteContents = $state<Record<string, string>>({});

  $effect(() => {
    if (notes) {
      notes.forEach((note) => {
        if (!(note.id in noteContents)) {
          noteContents[note.id] = note.content;
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
    saveNote(noteId, value);
  };

  const handleAddNote = async () => {
    const randomColor = colors[Math.floor(Math.random() * colors.length)];
    const result = await createNote({
      content: '',
      color: randomColor,
      entityId,
    });

    if (result?.id) {
      await notesQuery.load({ entityId });

      // NOTE: 어째선지 tick으로 하면 동작하지 않는다
      setTimeout(() => {
        const noteElement = document.querySelector(`[data-related-note-id="${result.id}"] textarea`) as HTMLTextAreaElement;
        noteElement?.focus();
      });
    }
  };

  const handleDeleteNote = async (noteId: string) => {
    await deleteNote({ noteId });
    notesQuery.load({ entityId });
  };

  $effect(() => {
    notesQuery.load({ entityId });
  });
</script>

<div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflow: 'hidden' })}>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      paddingX: '20px',
      flexShrink: '0',
      height: '40px',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={StickyNoteIcon} size={12} />
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>이 포스트 관련 노트</div>
    </div>

    <div class={flex({ gap: '8px', alignItems: 'center' })}>
      <button
        class={center({
          size: '20px',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
          cursor: 'pointer',
        })}
        onclick={handleAddNote}
        type="button"
        use:tooltip={{ message: '노트 추가', placement: 'top' }}
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
        onclick={() => (app.preference.current.noteExpanded = !app.preference.current.noteExpanded)}
        type="button"
        use:tooltip={{ message: app.preference.current.noteExpanded ? '작게 보기' : '크게 보기', placement: 'top' }}
      >
        <Icon icon={app.preference.current.noteExpanded ? Minimize2Icon : ExpandIcon} size={12} />
      </button>
    </div>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      flexGrow: '1',
      overflowY: 'auto',
      paddingX: '8px',
      paddingTop: '8px',
      paddingBottom: '20px',
    })}
  >
    {#if isLoading}
      <div class={center({ paddingY: '40px' })}>
        <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
      </div>
    {:else if notes.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '20px',
          paddingY: '60px',
        })}
      >
        <div
          class={center({
            size: '64px',
            borderRadius: '16px',
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={StickyNoteIcon} size={28} />
        </div>

        <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
          <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
            떠오르는 생각이나 아이디어를
            <br />
            자유롭게 기록해보세요
          </p>
        </div>

        <Button onclick={handleAddNote} size="sm" variant="secondary">노트 추가</Button>
      </div>
    {:else}
      {#each notes as note (note.id)}
        <div
          style:background-color={`color-mix(in srgb, #fff, ${new TinyColor(note.color).toRgbString()} 75%)`}
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
          data-related-note-id={note.id}
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
            onblur={() => {
              if (noteContents[note.id] === '' && notes.length !== 1) {
                handleDeleteNote(note.id);
              }
            }}
            oninput={(e) => handleNoteChange(note.id, e.currentTarget.value)}
            onkeydown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !e.isComposing) {
                e.preventDefault();
                handleAddNote();
              }
            }}
            placeholder="기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요."
            rows={3}
            value={noteContents[note.id] || ''}
            use:autosize
          ></textarea>

          <button
            class={center({
              position: 'absolute',
              bottom: '8px',
              right: '8px',
              size: '20px',
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
            onclick={() => handleDeleteNote(note.id)}
            type="button"
            use:tooltip={{ message: '노트 삭제', placement: 'top' }}
          >
            <Icon icon={Trash2Icon} size={12} />
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>
