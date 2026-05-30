<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, TimeAgo } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PencilIcon from '~icons/lucide/pencil';
  import Trash2Icon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { CommentCardV2_comment$key } from '$mearie';

  type Props = {
    comment$key: CommentCardV2_comment$key;
    canEdit: boolean;
    canDelete: boolean;
    onedit: (content: string) => void | Promise<void>;
    ondelete: () => void | Promise<void>;
    ondirty?: (id: string, dirty: boolean) => void;
  };
  let { comment$key, canEdit, canDelete, onedit, ondelete, ondirty }: Props = $props();

  const comment = createFragment(
    graphql(`
      fragment CommentCardV2_comment on DocumentComment {
        id
        content
        createdAt
        updatedAt

        user {
          id
          name
          avatar {
            id
            ...Img_image
          }
        }
      }
    `),
    () => comment$key,
  );

  let menuOpen = $state(false);
  let editing = $state(false);
  let editText = $state(comment.data.content);
  let textareaEl = $state<HTMLTextAreaElement>();
  const hasEditText = $derived(editText.trim().length > 0);
  const hasEditContent = $derived(editText.length > 0);

  $effect(() => {
    if (editing && textareaEl) {
      textareaEl.focus();
      textareaEl.scrollIntoView({ block: 'nearest' });
    }
  });

  $effect(() => {
    ondirty?.(comment.data.id, editing && editText !== comment.data.content);
    return () => ondirty?.(comment.data.id, false);
  });

  const save = async () => {
    const trimmed = editText.trim();
    if (!trimmed || trimmed === comment.data.content) {
      editing = false;
      return;
    }
    try {
      await onedit(trimmed);
      editing = false;
    } catch {
      // ignore
    }
  };

  const cancel = () => {
    editText = comment.data.content;
    editing = false;
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void save();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      if (editText === comment.data.content) {
        cancel();
      } else {
        Dialog.confirm({
          title: '수정 취소',
          message: '수정을 취소하시겠어요?',
          action: 'danger',
          actionLabel: '수정 취소',
          actionHandler: () => {
            cancel();
          },
        });
      }
    }
  };
</script>

<div class={cx('group', css({ display: 'flex', gap: '8px' }))}>
  {#if comment.data.user.avatar}
    <Img
      style={css.raw({ size: '24px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
      alt={comment.data.user.name}
      image$key={comment.data.user.avatar}
      size={24}
    />
  {:else}
    <div class={css({ size: '24px', borderRadius: 'full', flexShrink: '0', marginTop: '1px', backgroundColor: 'surface.muted' })}></div>
  {/if}

  <div class={css({ flexGrow: '1', minWidth: '0' })}>
    <div class={css({ display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px', minHeight: '20px' })}>
      <span
        class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default', truncate: true, flexShrink: '1', minWidth: '0' })}
      >
        {comment.data.user.name}
      </span>
      <TimeAgo
        style={css.raw({ fontSize: '11px', color: 'text.faint', flexShrink: '0' })}
        timestamp={new Date(comment.data.createdAt).getTime()}
      />

      {#if canEdit || canDelete}
        <div
          style:opacity={menuOpen ? '1' : undefined}
          style:visibility={editing ? 'hidden' : undefined}
          style:pointer-events={editing ? 'none' : undefined}
          class={css({ marginLeft: 'auto', opacity: '0', transition: 'common', _groupHover: { opacity: '100' } })}
        >
          <Menu
            style={css.raw({ display: 'flex', padding: '0', borderWidth: '0', backgroundColor: 'transparent' })}
            listStyle={css.raw({ minWidth: '100px' })}
            offset={4}
            placement="bottom-end"
            bind:open={menuOpen}
          >
            {#snippet button()}
              <button
                class={css(
                  {
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    size: '20px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    color: 'text.faint',
                    transition: 'common',
                    _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
                  },
                  menuOpen && { backgroundColor: 'surface.muted', color: 'text.subtle' },
                )}
                type="button"
              >
                <Icon icon={EllipsisIcon} size={12} />
              </button>
            {/snippet}

            {#if canEdit}
              <MenuItem
                icon={PencilIcon}
                onclick={() => {
                  editText = comment.data.content;
                  editing = true;
                }}
              >
                수정
              </MenuItem>
            {/if}
            {#if canDelete}
              <MenuItem icon={Trash2Icon} onclick={() => void ondelete()} variant="danger">삭제</MenuItem>
            {/if}
          </Menu>
        </div>
      {/if}
    </div>

    {#if editing}
      <div class={css({ position: 'relative', display: 'flex' })}>
        <textarea
          bind:this={textareaEl}
          style:padding-right={hasEditContent ? '10px' : '40px'}
          style:padding-bottom={hasEditContent ? '40px' : '8px'}
          class={css({
            width: 'full',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '6px',
            paddingLeft: '10px',
            paddingTop: '8px',
            fontSize: '13px',
            lineHeight: '[1.4]',
            color: 'text.default',
            backgroundColor: 'surface.subtle',
            resize: 'none',
            minHeight: '36px',
            maxHeight: '120px',
            outline: 'none',
            transition: 'common',
            _focus: { borderColor: 'accent.brand.default', backgroundColor: 'surface.default' },
          })}
          onkeydown={handleKeydown}
          rows={1}
          bind:value={editText}
          use:autosize
        ></textarea>
        <div
          style:top={hasEditContent ? 'auto' : '50%'}
          style:bottom={hasEditContent ? '6px' : 'auto'}
          style:transform={hasEditContent ? undefined : 'translateY(-50%)'}
          class={css({ position: 'absolute', right: '6px', display: 'flex', alignItems: 'center', gap: '4px', transition: 'common' })}
        >
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              size: '22px',
              borderRadius: 'full',
              cursor: 'pointer',
              backgroundColor: 'surface.subtle',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
            })}
            onclick={cancel}
            type="button"
            use:tooltip={{ message: '수정 취소', placement: 'bottom' }}
          >
            <Icon icon={XIcon} size={12} />
          </button>
          <button
            class={css(
              {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                size: '22px',
                borderRadius: 'full',
                transition: 'common',
              },
              hasEditText
                ? {
                    cursor: 'pointer',
                    backgroundColor: 'accent.brand.default',
                    color: 'text.bright',
                    _hover: { backgroundColor: 'accent.brand.hover' },
                    _active: { backgroundColor: 'accent.brand.active' },
                  }
                : { cursor: 'default', backgroundColor: 'surface.muted', color: 'text.disabled' },
            )}
            disabled={!hasEditText}
            onclick={() => void save()}
            type="button"
            use:tooltip={{ message: '저장', placement: 'bottom' }}
          >
            <Icon icon={ArrowUpIcon} size={12} />
          </button>
        </div>
      </div>
    {:else}
      <p
        class={css({
          margin: '0',
          fontSize: '13px',
          lineHeight: '[1.4]',
          color: 'text.subtle',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          userSelect: 'text',
          cursor: 'text',
        })}
      >
        {comment.data.content}
      </p>
    {/if}
  </div>
</div>
