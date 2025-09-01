<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { contextMenu } from '@typie/ui/actions';
  import { Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import { fragment, graphql } from '$graphql';
  import CanvasMenu from '../@context-menu/CanvasMenu.svelte';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import { getTreeContext } from './state.svelte';
  import type { DashboardLayout_EntityTree_Canvas_canvas } from '$graphql';

  type Props = {
    $canvas: DashboardLayout_EntityTree_Canvas_canvas;
  };

  let { $canvas: _canvas }: Props = $props();

  const canvas = fragment(
    _canvas,
    graphql(`
      fragment DashboardLayout_EntityTree_Canvas_canvas on Canvas {
        id
        title

        entity {
          id
          slug
          depth
          order
          visibility
          url
        }
      }
    `),
  );

  const app = getAppContext();
  const treeState = getTreeContext();
  const active = $derived(app.state.current === $canvas.entity.id);
  const selected = $derived(treeState.selectedEntityIds.has($canvas.entity.id));

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
      $canvas.entity.depth > 0 && {
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
  data-id={$canvas.entity.id}
  data-order={$canvas.entity.order}
  data-path-depth={$canvas.entity.depth}
  data-slug={$canvas.entity.slug}
  data-type="canvas"
  draggable="false"
  href="/{$canvas.entity.slug}"
  role="treeitem"
  use:contextMenu={{ content: contextMenuContent }}
>
  <EntitySelectionIndicator entityId={$canvas.entity.id} visibility={$canvas.entity.visibility} />

  <Icon style={css.raw({ color: 'text.faint' })} icon={LineSquiggleIcon} size={14} />

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
    {$canvas.title}
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
  {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has($canvas.entity.id)}
    <MultiEntitiesMenu />
  {:else}
    <CanvasMenu canvas={$canvas} entity={$canvas.entity} via="tree" />
  {/if}
{/snippet}
