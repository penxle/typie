<script lang="ts">
  import dayjs from 'dayjs';
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { fragment, graphql } from '$graphql';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { UsersiteWildcardSlugPage_Comment_comment } from '$graphql';

  type Props = {
    $comment: UsersiteWildcardSlugPage_Comment_comment;
  };

  let { $comment: _comment }: Props = $props();

  const comment = fragment(
    _comment,
    graphql(`
      fragment UsersiteWildcardSlugPage_Comment_comment on Comment {
        id
        content
        state
        createdAt
      }
    `),
  );

  const deleteComment = graphql(`
    mutation UsersiteWildcardSlugPage_DeleteComment_Mutation($input: DeleteCommentInput!) {
      deleteComment(input: $input) {
        id
        state
      }
    }
  `);
</script>

<div>
  {#if $comment.state === 'ACTIVE'}
    <div class={flex({ align: 'center', justify: 'space-between' })}>
      <p class={css({ fontSize: '15px', color: 'gray.600' })}>익명</p>

      <Menu placement="bottom-end">
        {#snippet button()}
          <div
            class={css({
              borderRadius: '4px',
              padding: '2px',
              color: 'gray.400',
              transition: 'common',
              _hover: { backgroundColor: 'gray.200' },
            })}
          >
            <Icon icon={EllipsisVerticalIcon} size={20} />
          </div>
        {/snippet}

        <MenuItem
          onclick={() => {
            Dialog.confirm({
              title: '댓글 삭제',
              message: '정말로 이 댓글을 삭제하시겠어요?',
              actionLabel: '삭제',
              actionHandler: async () => {
                await deleteComment({ commentId: $comment.id });
              },
            });
          }}
          variant="danger"
        >
          <Icon icon={Trash2Icon} size={12} />
          <span>삭제</span>
        </MenuItem>
      </Menu>
    </div>

    <div class={css({ marginTop: '8px', fontSize: '14px', color: 'gray.800' })}>
      {$comment.content}
    </div>
  {:else}
    <div class={css({ fontSize: '14px', color: 'gray.500' })}>삭제된 댓글입니다</div>
  {/if}

  <date class={css({ marginTop: '4px', fontSize: '13px', color: 'gray.400' })} datetime={$comment.createdAt}>
    {dayjs($comment.createdAt).formatAsDateTime()}
  </date>
</div>
