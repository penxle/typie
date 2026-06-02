<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { SvelteSet } from 'svelte/reactivity';
  import CheckIcon from '~icons/lucide/check';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import Trash2Icon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import { canDeleteComment, canManageThread, canUpdateComment, isRootComment } from '$lib/editor-ffi/comments';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import CommentCard from './CommentCard.svelte';
  import CommentComposer from './CommentComposer.svelte';
  import CommentPopoverCard from './CommentPopoverCard.svelte';
  import { getCommentContext } from './context.svelte';

  const { editor } = getEditorContext();
  const comments = getCommentContext();

  const threadFragment = createFragment(
    graphql(`
      fragment CommentPopoverV2_thread on DocumentCommentThread {
        id

        user {
          id
        }

        comments {
          id

          user {
            id
          }

          ...CommentCardV2_comment
        }
      }
    `),
    () => comments.activeThread,
  );

  const open = $derived(comments.composing || comments.activeThreadId !== null);
  const thread = $derived(threadFragment.data);
  const anchor = $derived(comments.activeAnchor);

  let point = $state<{ x: number; y: number } | null>(null);

  $effect(() => {
    if (!open || !editor || !anchor) {
      point = null;
      return;
    }

    point = editor.localToOffset(anchor.page, anchor.x, anchor.y + anchor.height);
  });

  let composerDirty = $state(false);
  let headerMenuOpen = $state(false);
  const dirtyCommentIds = new SvelteSet<string>();
  const hasUnsavedChanges = $derived(composerDirty || dirtyCommentIds.size > 0);

  $effect(() => {
    if (!open) return;
    return pushEscapeHandler(() => {
      if (!open) return false;
      tryClose();
      return true;
    });
  });

  function tryClose() {
    if (hasUnsavedChanges) {
      Dialog.confirm({
        title: '작성 중인 내용 삭제',
        message: '작성 중인 내용이 사라집니다. 닫으시겠어요?',
        action: 'danger',
        actionLabel: '닫기',
        actionHandler: () => comments.close(),
      });
    } else {
      comments.close();
    }
  }

  function confirmDeleteThread(threadId: string) {
    Dialog.confirm({
      title: '코멘트 스레드 삭제',
      message: '이 코멘트 스레드를 삭제하시겠어요? 되돌릴 수 없어요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: () => {
        void comments.deleteThread(threadId);
      },
    });
  }

  function confirmDeleteComment(commentId: string) {
    Dialog.confirm({
      title: '코멘트 삭제',
      message: '코멘트를 삭제하시겠어요?',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: () => {
        void comments.deleteComment(commentId);
      },
    });
  }

  const headerButtonStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    size: '20px',
    borderRadius: '4px',
    cursor: 'pointer',
    color: 'text.faint',
    transition: 'common',
    _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
  });
</script>

{#if open && point}
  <CommentPopoverCard onclickoutside={tryClose} x={point.x} y={point.y}>
    {#if comments.composing}
      <CommentComposer
        autofocus
        oncancel={tryClose}
        ondirty={(d) => (composerDirty = d)}
        onsubmit={(content) => comments.createThread(content)}
      />
    {:else if thread}
      <div
        class={css({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingX: '10px',
          paddingY: '8px',
          borderBottomWidth: '1px',
          borderColor: 'border.subtle',
        })}
      >
        <div class={css({ display: 'flex', alignItems: 'center', gap: '6px' })}>
          <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>코멘트</span>
          {#if thread.comments.length > 0}
            <span
              class={css({
                fontSize: '11px',
                fontWeight: 'bold',
                lineHeight: 'none',
                color: 'text.subtle',
                backgroundColor: 'surface.muted',
                borderRadius: '4px',
                paddingX: '6px',
                paddingY: '2px',
              })}
            >
              {thread.comments.length}
            </span>
          {/if}
        </div>

        <div class={css({ display: 'flex', alignItems: 'center', gap: '2px' })}>
          {#if canManageThread(thread, comments.myId, comments.isOwner)}
            <Menu
              style={css.raw({ display: 'flex', padding: '0', borderWidth: '0', backgroundColor: 'transparent' })}
              listStyle={css.raw({ minWidth: '120px' })}
              offset={4}
              placement="bottom-end"
              bind:open={headerMenuOpen}
            >
              {#snippet button()}
                <button
                  class={css(headerButtonStyle, headerMenuOpen && { backgroundColor: 'surface.muted', color: 'text.subtle' })}
                  type="button"
                >
                  <Icon icon={EllipsisIcon} size={12} />
                </button>
              {/snippet}

              <MenuItem icon={Trash2Icon} onclick={() => confirmDeleteThread(thread.id)} variant="danger">스레드 삭제</MenuItem>
            </Menu>
            <button
              class={css(headerButtonStyle)}
              onclick={() => comments.resolveThread(thread.id)}
              type="button"
              use:tooltip={{ message: '해결', placement: 'bottom' }}
            >
              <Icon icon={CheckIcon} size={12} />
            </button>
          {/if}
          <button class={css(headerButtonStyle)} onclick={tryClose} type="button">
            <Icon icon={XIcon} size={12} />
          </button>
        </div>
      </div>

      <div
        class={css({
          display: 'flex',
          flexDirection: 'column',
          gap: '2px',
          maxHeight: '300px',
          overflowX: 'hidden',
          overflowY: 'auto',
          paddingY: '4px',
        })}
      >
        {#each thread.comments as c (c.id)}
          {@const root = isRootComment(thread, c.id)}
          <div class={css({ paddingX: '10px', paddingY: '6px' })}>
            <CommentCard
              canDelete={root
                ? canManageThread(thread, comments.myId, comments.isOwner)
                : canDeleteComment(thread, c.id, comments.myId, comments.isOwner)}
              canEdit={canUpdateComment(c, comments.myId)}
              comment$key={c}
              ondelete={() => (root ? confirmDeleteThread(thread.id) : confirmDeleteComment(c.id))}
              ondirty={(id, dirty) => {
                if (dirty) dirtyCommentIds.add(id);
                else dirtyCommentIds.delete(id);
              }}
              onedit={(content) => comments.editComment(c.id, content)}
            />
          </div>
        {/each}
      </div>

      <div class={css({ borderTopWidth: '1px', borderColor: 'border.subtle' })}></div>
      <CommentComposer
        oncancel={tryClose}
        ondirty={(d) => (composerDirty = d)}
        onsubmit={(content) => comments.reply(thread.id, content)}
        placeholder="답글을 입력하세요"
      />
    {/if}
  </CommentPopoverCard>
{/if}
