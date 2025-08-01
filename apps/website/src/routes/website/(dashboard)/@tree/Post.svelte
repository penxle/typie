<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { EntityAvailability, EntityVisibility, PostType } from '@/enums';
  import { TypieError } from '@/errors';
  import BlendIcon from '~icons/lucide/blend';
  import CopyIcon from '~icons/lucide/copy';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import FileIcon from '~icons/lucide/file';
  import InfoIcon from '~icons/lucide/info';
  import ShapesIcon from '~icons/lucide/shapes';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, Menu, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog, Toast } from '$lib/notification';
  import { comma } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import { getTreeContext } from './state.svelte';
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
        type
        title
        characterCount

        entity {
          id
          depth
          order
          slug
          visibility
          availability
          url
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

        entity {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_Trash_site
            ...DashboardLayout_PlanUsageWidget_site
          }

          user {
            id

            recentlyViewedEntities {
              id
            }
          }
        }
      }
    }
  `);

  const app = getAppContext();
  const treeState = getTreeContext();
  const active = $derived(app.state.current === $post.entity.id);
  const selected = $derived(treeState.selectedEntityIds.has($post.entity.id));

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
        _supportHover: { backgroundColor: 'surface.muted' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
      },
      $post.entity.depth > 0 && {
        borderLeftWidth: '1px',
        borderLeftRadius: '0',
        marginLeft: '-1px',
        paddingLeft: '14px',
        _supportHover: { borderColor: 'border.strong' },
      },
      active && {
        backgroundColor: 'surface.muted',
      },
      selected && {
        backgroundColor: 'accent.brand.subtle',
        _supportHover: { backgroundColor: 'accent.brand.subtle' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'accent.brand.subtle' },
      },
    ),
  )}
  aria-selected="false"
  data-id={$post.entity.id}
  data-order={$post.entity.order}
  data-path-depth={$post.entity.depth}
  data-type="post"
  draggable="false"
  href="/{$post.entity.slug}"
  role="treeitem"
>
  <EntitySelectionIndicator entityId={$post.entity.id} visibility={$post.entity.visibility} />

  {#if $post.type === PostType.NORMAL}
    <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} size={14} />
  {:else if $post.type === PostType.TEMPLATE}
    <Icon style={css.raw({ color: 'text.faint' })} icon={ShapesIcon} size={14} />
  {/if}

  <span
    class={css(
      {
        flexGrow: '1',
        fontSize: '14px',
        fontWeight: 'medium',
        color: 'text.muted',
        wordBreak: 'break-all',
        lineClamp: '1',
      },
      active && { fontWeight: 'bold', color: 'text.default' },
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
          color: 'text.disabled',
          opacity: '0',
          transition: 'common',
          _hover: { backgroundColor: 'interactive.hover' },
          _groupHover: { opacity: '100' },
          _pressed: { backgroundColor: 'interactive.hover', opacity: '100' },
        })}
        aria-pressed={open}
      >
        <Icon icon={EllipsisIcon} size={14} />
      </div>
    {/snippet}

    {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has($post.entity.id)}
      <MultiEntitiesMenu />
    {:else}
      <MenuItem external href={$post.entity.url} icon={ExternalLinkIcon} type="link">사이트에서 열기</MenuItem>

      <HorizontalDivider color="secondary" />

      <MenuItem icon={BlendIcon} onclick={() => (app.state.shareOpen = $post.entity.id)}>공유 및 게시</MenuItem>

      <MenuItem
        icon={CopyIcon}
        onclick={async () => {
          try {
            const resp = await duplicatePost({ postId: $post.id });
            mixpanel.track('duplicate_post', { via: 'tree' });
            await goto(`/${resp.entity.slug}`);
          } catch (err) {
            const errorMessages: Record<string, string> = {
              character_count_limit_exceeded: '현재 플랜의 글자 수 제한을 초과했어요.',
              blob_size_limit_exceeded: '현재 플랜의 파일 크기 제한을 초과했어요.',
            };

            if (err instanceof TypieError) {
              const message = errorMessages[err.code] || err.code;
              Toast.error(message);
            }
          }
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
            message: `정말 "${$post.title}" 포스트를 삭제하시겠어요?`,
            children: deleteDetailsView,
            action: 'danger',
            actionLabel: '삭제',
            actionHandler: async () => {
              await deletePost({ postId: $post.id });
              mixpanel.track('delete_post', { via: 'tree' });
              app.state.ancestors = [];
              app.state.current = undefined;
              if (treeState.selectedEntityIds.has($post.entity.id)) {
                treeState.selectedEntityIds.delete($post.entity.id);
              }
              if (treeState.lastSelectedEntityId === $post.entity.id) {
                treeState.lastSelectedEntityId = undefined;
              }
            },
          });
        }}
        variant="danger"
      >
        삭제
      </MenuItem>

      {#snippet deleteDetailsView()}
        <div
          class={flex({
            alignItems: 'center',
            gap: '6px',
            borderRadius: '8px',
            paddingX: '12px',
            paddingY: '8px',
            backgroundColor: 'surface.subtle',
          })}
        >
          <Icon style={css.raw({ color: 'text.muted' })} icon={InfoIcon} size={14} />
          <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>삭제 후 30일 동안 휴지통에 보관돼요</span>
        </div>
      {/snippet}

      <HorizontalDivider color="secondary" />

      <div class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', color: 'text.disabled', userSelect: 'none' })}>
        <div class={css({ fontWeight: 'medium' })}>
          {#if $post.entity.visibility === EntityVisibility.UNLISTED || $post.entity.availability === EntityAvailability.UNLISTED}
            <span class={css({ color: 'accent.brand.default' })}>
              {#if $post.entity.visibility === EntityVisibility.UNLISTED && $post.entity.availability === EntityAvailability.UNLISTED}
                링크 조회/편집 가능 포스트
              {:else if $post.entity.visibility === EntityVisibility.UNLISTED}
                링크 조회 가능 포스트
              {:else if $post.entity.availability === EntityAvailability.UNLISTED}
                링크 편집 가능 포스트
              {/if}
            </span>
          {:else}
            <span>비공개 포스트</span>
          {/if}
        </div>

        <span>총 {comma($post.characterCount)}자</span>
      </div>
    {/if}
  </Menu>
</a>
