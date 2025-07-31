<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import CopyIcon from '~icons/lucide/copy';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, Menu, MenuItem } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import EntitySelectionIndicator from './EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './MultiEntitiesMenu.svelte';
  import type { DashboardLayout_EntityTree_Canvas_canvas, DashboardLayout_EntityTree_site } from '$graphql';

  type Props = {
    $canvas: DashboardLayout_EntityTree_Canvas_canvas;
    $site: DashboardLayout_EntityTree_site;
  };

  let { $canvas: _canvas, $site: _site }: Props = $props();

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

  const duplicateCanvas = graphql(`
    mutation DashboardLayout_EntityTree_Canvas_DuplicateCanvas_Mutation($input: DuplicateCanvasInput!) {
      duplicateCanvas(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deleteCanvas = graphql(`
    mutation DashboardLayout_EntityTree_Canvas_DeleteCanvas_Mutation($input: DeleteCanvasInput!) {
      deleteCanvas(input: $input) {
        id
      }
    }
  `);

  const app = getAppContext();
  const active = $derived(app.state.current === $canvas.entity.id);
  const selected = $derived(app.state.tree.selectedEntityIds.has($canvas.entity.id));

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
      },
    ),
  )}
  aria-selected="false"
  data-id={$canvas.entity.id}
  data-order={$canvas.entity.order}
  data-path-depth={$canvas.entity.depth}
  data-type="canvas"
  draggable="false"
  href="/{$canvas.entity.slug}"
  role="treeitem"
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

    {#if app.state.tree.selectedEntityIds.size > 1 && app.state.tree.selectedEntityIds.has($canvas.entity.id)}
      <MultiEntitiesMenu $site={_site} />
    {:else}
      <MenuItem
        icon={CopyIcon}
        onclick={async () => {
          const resp = await duplicateCanvas({ canvasId: $canvas.id });
          mixpanel.track('duplicate_canvas', { via: 'tree' });
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
            title: '캔버스 삭제',
            message: `정말 "${$canvas.title}" 캔버스를 삭제하시겠어요?`,
            action: 'danger',
            actionLabel: '삭제',
            actionHandler: async () => {
              await deleteCanvas({ canvasId: $canvas.id });
              mixpanel.track('delete_canvas', { via: 'tree' });
              app.state.ancestors = [];
              app.state.current = undefined;
              if (app.state.tree.selectedEntityIds.has($canvas.entity.id)) {
                app.state.tree.selectedEntityIds.delete($canvas.entity.id);
              }
              if (app.state.tree.lastSelectedEntityId === $canvas.entity.id) {
                app.state.tree.lastSelectedEntityId = undefined;
              }
            },
          });
        }}
        variant="danger"
      >
        삭제
      </MenuItem>
    {/if}
  </Menu>
</a>
