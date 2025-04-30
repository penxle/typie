<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { EntityVisibility } from '@/enums';
  import BlendIcon from '~icons/lucide/blend';
  import CopyIcon from '~icons/lucide/copy';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, Menu, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
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

  const app = getAppContext();
  const active = $derived(app.state.current === $post.entity.id);

  let element = $state<HTMLAnchorElement>();

  $effect(() => {
    if (active) {
      element?.scrollIntoView({ behavior: 'instant', block: 'nearest' });
    }
  });
</script>

<a
  bind:this={element}
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
        '&:has([aria-pressed="true"])': { backgroundColor: 'gray.100' },
      },
      $post.entity.depth > 0 && {
        borderLeftWidth: '1px',
        borderLeftRadius: '0',
        marginLeft: '-1px',
        paddingLeft: '14px',
        _hover: { borderLeftColor: 'gray.300' },
      },
      active && {
        backgroundColor: 'gray.100',
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
      $post.entity.visibility === EntityVisibility.UNLISTED && { backgroundColor: 'brand.500' },
    )}
  ></div>

  <Icon style={css.raw({ color: 'gray.500' })} icon={FileIcon} size={14} />

  <span
    class={css(
      {
        flexGrow: '1',
        fontSize: '14px',
        fontWeight: 'medium',
        color: 'gray.600',
        wordBreak: 'break-all',
        lineClamp: '1',
      },
      active && { fontWeight: 'bold', color: 'gray.950' },
    )}
  >
    {$post.title}
  </span>

  <Menu placement="bottom-start">
    {#snippet button({ open })}
      <div
        class={center({
          borderRadius: '4px',
          size: '16px',
          color: 'gray.400',
          opacity: '0',
          transition: 'common',
          _hover: { backgroundColor: 'gray.200' },
          _groupHover: { opacity: '100' },
          _pressed: { backgroundColor: 'gray.200', opacity: '100' },
        })}
        aria-pressed={open}
      >
        <Icon icon={EllipsisIcon} size={14} />
      </div>
    {/snippet}

    <MenuItem icon={BlendIcon} onclick={() => (app.state.shareOpen = $post.entity.id)}>공유</MenuItem>

    <MenuItem
      icon={CopyIcon}
      onclick={async () => {
        const resp = await duplicatePost({ postId: $post.id });
        mixpanel.track('duplicate_post');
        await goto(`/${resp.entity.slug}`);
      }}
    >
      복제
    </MenuItem>

    <HorizontalDivider color="secondary" />

    <MenuItem
      icon={TrashIcon}
      onclick={async () => {
        Dialog.confirm({
          title: '포스트 삭제',
          message: '정말 이 포스트를 삭제하시겠어요?',
          action: 'danger',
          actionLabel: '삭제',
          actionHandler: async () => {
            await deletePost({ postId: $post.id });
            mixpanel.track('delete_post', { via: 'tree' });
            app.state.ancestors = [];
            app.state.current = undefined;
          },
        });
      }}
      variant="danger"
    >
      삭제
    </MenuItem>
  </Menu>
</a>
