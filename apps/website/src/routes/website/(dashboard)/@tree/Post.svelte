<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { contextMenu } from '@typie/ui/actions';
  import { Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { PostType } from '@/enums';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import ShapesIcon from '~icons/lucide/shapes';
  import { fragment, graphql } from '$graphql';
  import PostMenu from '../@context-menu/PostMenu.svelte';
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
        '&[data-context-menu-open="true"]': { backgroundColor: 'surface.muted' },
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
        '&[data-context-menu-open="true"]': { backgroundColor: 'accent.brand.subtle' },
      },
    ),
  )}
  aria-selected="false"
  data-id={$post.entity.id}
  data-order={$post.entity.order}
  data-path-depth={$post.entity.depth}
  data-slug={$post.entity.slug}
  data-type="post"
  draggable="false"
  href="/{$post.entity.slug}"
  role="treeitem"
  use:contextMenu={{ content: contextMenuContent }}
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

    {@render contextMenuContent()}
  </Menu>
</a>

{#snippet contextMenuContent()}
  {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has($post.entity.id)}
    <MultiEntitiesMenu />
  {:else}
    <PostMenu entity={$post.entity} post={$post} via="tree" />
  {/if}
{/snippet}
