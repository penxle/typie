<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { contextMenu, tooltip } from '@typie/ui/actions';
  import { Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import { graphql } from '$mearie';
  import DividerMenu from '../@context-menu/DividerMenu.svelte';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import DividerTooltip from './DividerTooltip.svelte';
  import { entityTreeRevealState } from './entity-reveal.svelte';
  import { getTreeContext } from './state.svelte';
  import type { DashboardLayout_EntityTree_Divider_divider$key } from '$mearie';

  type Props = {
    divider$key: DashboardLayout_EntityTree_Divider_divider$key;
  };

  let { divider$key }: Props = $props();

  const divider = createFragment(
    graphql(`
      fragment DashboardLayout_EntityTree_Divider_divider on Divider {
        id
        createdAt

        entity {
          id
          order
          depth

          parent {
            id
          }

          site {
            id
          }
        }
      }
    `),
    () => divider$key,
  );

  const app = getAppContext();
  const treeState = getTreeContext();
  const selected = $derived(treeState.selectedEntityIds.has(divider.data.entity.id));
  const isCut = $derived(app.state.clipboard?.mode === 'cut' && app.state.clipboard.entityIds.includes(divider.data.entity.id));

  let element = $state<HTMLDivElement>();

  $effect(() => {
    const request = entityTreeRevealState.current;
    if (request?.targetEntityId !== divider.data.entity.id) {
      return;
    }

    element?.scrollIntoView({ behavior: 'instant', block: 'nearest' });
    entityTreeRevealState.consume(request);
  });
</script>

<div
  bind:this={element}
  style:opacity={isCut ? 0.5 : 1}
  class={cx(
    'group',
    css(
      {
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        paddingX: '8px',
        paddingY: '4px',
        borderRadius: '6px',
        transition: 'common',
        _supportHover: { backgroundColor: 'surface.hover' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'surface.active', _supportHover: { backgroundColor: 'surface.active' } },
        '&[data-context-menu-open="true"]': { backgroundColor: 'surface.active' },
      },
      divider.data.entity.depth > 0 && {
        borderLeftWidth: '1px',
        borderLeftRadius: '0',
        marginLeft: '-1px',
        paddingLeft: '14px',
        _supportHover: { borderColor: 'border.emphasis' },
      },
      selected && {
        backgroundColor: 'surface.active',
        _supportHover: { backgroundColor: 'surface.active' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'surface.active' },
        '&[data-context-menu-open="true"]': {
          backgroundColor: 'surface.active',
        },
      },
    ),
  )}
  aria-label="구분선"
  aria-selected="false"
  data-id={divider.data.entity.id}
  data-name="구분선"
  data-order={divider.data.entity.order}
  data-path-depth={divider.data.entity.depth}
  data-type="divider"
  role="treeitem"
  use:contextMenu={{ content: contextMenuContent }}
  use:tooltip={{ message: tooltipContent, placement: 'right', delay: 1000 }}
>
  <EntitySelectionIndicator dot={false} entityId={divider.data.entity.id} />

  <div class={css({ flexGrow: '1', height: '1px', backgroundColor: 'border.default' })}></div>

  <Menu placement="bottom-start">
    {#snippet button({ open })}
      <div
        class={center({
          borderRadius: '4px',
          size: '16px',
          color: 'text.muted',
          opacity: '0',
          transition: 'common',
          _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
          _groupHover: { opacity: '100' },
          _pressed: { backgroundColor: 'surface.active', color: 'text.default', opacity: '100' },
        })}
        aria-pressed={open}
      >
        <Icon icon={EllipsisIcon} size={14} />
      </div>
    {/snippet}

    {@render contextMenuContent()}
  </Menu>
</div>

{#snippet contextMenuContent()}
  {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has(divider.data.entity.id)}
    <MultiEntitiesMenu />
  {:else}
    <DividerMenu divider={divider.data} entity={divider.data.entity} via="tree" />
  {/if}
{/snippet}

{#snippet tooltipContent()}
  <DividerTooltip createdAt={divider.data.createdAt} />
{/snippet}
