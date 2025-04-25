<script lang="ts">
  import CopyIcon from '~icons/lucide/copy';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { Icon, Menu, MenuItem } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import type { DashboardLayout_EntityTree_Post_post } from '$graphql';

  type Props = {
    $post: DashboardLayout_EntityTree_Post_post;
  };

  let { $post: _post }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment DashboardLayout_EntityTree_Post_post on Post {
        id
        title

        entity {
          id
          depth
          order
          slug
        }

        postOption: option {
          id
          visibility
        }
      }
    `),
  );

  const duplicatePost = graphql(`
    mutation DashboardLayout_EntityTree_Post_DuplicatePost_Mutation($input: DuplicatePostInput!) {
      duplicatePost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deletePost = graphql(`
    mutation DashboardLayout_EntityTree_Post_DeletePost_Mutation($input: DeletePostInput!) {
      deletePost(input: $input) {
        id
      }
    }
  `);
</script>

<a
  class={cx(
    'group',
    css(
      {
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        paddingX: '8px',
        paddingY: '6px',
        borderRadius: '6px',
        transition: 'common',
        _hover: { backgroundColor: 'gray.100' },
      },
      $post.entity.depth > 0 && {
        borderLeftWidth: '1px',
        borderLeftRadius: '0',
        marginLeft: '-1px',
        paddingLeft: '14px',
        _hover: { borderLeftColor: 'gray.900' },
      },
    ),
  )}
  aria-selected="false"
  data-depth={$post.entity.depth}
  data-id={$post.entity.id}
  data-order={$post.entity.order}
  data-type="post"
  draggable="false"
  href="/{$post.entity.slug}"
  role="treeitem"
>
  <div
    class={css(
      { flex: 'none', borderRadius: 'full', backgroundColor: 'gray.200', size: '4px' },
      $post.postOption.visibility === 'UNLISTED' && { backgroundColor: 'brand.500' },
    )}
  ></div>

  <Icon style={css.raw({ color: 'gray.500' })} icon={FileIcon} size={14} />

  <span
    class={css({
      flexGrow: '1',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'gray.600',
      wordBreak: 'break-all',
      lineClamp: '1',
    })}
  >
    {$post.title}
  </span>

  <Menu placement="bottom-start">
    {#snippet button({ open })}
      <div
        class={css(
          {
            display: 'none',
            justifyContent: 'center',
            alignItems: 'center',
            borderRadius: '4px',
            size: '16px',
            color: 'gray.400',
            _hover: { backgroundColor: 'gray.200' },
            _groupHover: { display: 'block', opacity: '100' },
          },
          open && { display: 'block', opacity: '100' },
        )}
      >
        <Icon icon={EllipsisIcon} size={14} />
      </div>
    {/snippet}

    <MenuItem
      onclick={async () => {
        const resp = await duplicatePost({ postId: $post.id });
        await goto(`/${resp.entity.slug}`);
      }}
    >
      <Icon icon={CopyIcon} size={12} />
      <span>포스트 복제</span>
    </MenuItem>

    <MenuItem
      onclick={async () => {
        Dialog.confirm({
          title: '포스트 삭제',
          message: '정말 이 포스트를 삭제하시겠어요?',
          action: 'danger',
          actionLabel: '삭제',
          actionHandler: async () => {
            await deletePost({ postId: $post.id });
          },
        });
      }}
    >
      <Icon icon={Trash2Icon} size={12} />
      <span>포스트 삭제</span>
    </MenuItem>
  </Menu>
</a>
