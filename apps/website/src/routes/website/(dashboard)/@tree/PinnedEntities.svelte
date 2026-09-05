<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Toast } from '@typie/ui/notification';
  import { createDragScroll, elementScrollViewport } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, untrack } from 'svelte';
  import { graphql } from '$mearie';
  import { getPaneGroup } from '../[slug]/@pane/context.svelte';
  import Document from './Document.svelte';
  import DragIndicator from './DragIndicator.svelte';
  import { EntityRowDragController, UNPIN_HOLD_MS } from './entity-row-drag.svelte';
  import EntityRowDragGhost from './EntityRowDragGhost.svelte';
  import { pinnedPlacementIndicator, resolvePinnedOrders, resolvePinnedPlacementAt } from './pinned-placement';
  import PinnedFolder from './PinnedFolder.svelte';
  import type { DashboardLayout_PinnedEntities_site$key } from '$mearie';
  import type { DragIndicatorState } from './DragIndicator.svelte';
  import type { EntityRowDragItem, EntityRowDrop, EntityRowDropResult } from './entity-row-drag.svelte';

  const PAGE_SIZE = 5;

  type Props = {
    site$key: DashboardLayout_PinnedEntities_site$key;
    sectionElement?: HTMLElement;
    collapsed: boolean;
  };

  let { site$key, sectionElement, collapsed }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_PinnedEntities_site on Site {
        id

        pinnedEntities {
          id
          pinnedOrder

          node {
            __typename

            ... on Document {
              id
              ...DashboardLayout_EntityTree_Document_document
            }

            ... on Folder {
              id
              ...DashboardLayout_PinnedFolder_folder
            }
          }
        }
      }
    `),
    () => site$key,
  );

  const [pinEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_PinnedEntities_PinEntities_Mutation($input: PinEntitiesInput!) {
        pinEntities(input: $input) {
          id
          pinnedOrder

          site {
            id
            ...DashboardLayout_PinnedEntities_site
          }
        }
      }
    `),
  );

  const [unpinEntity] = createMutation(
    graphql(`
      mutation DashboardLayout_PinnedEntities_UnpinEntity_Mutation($input: UnpinEntityInput!) {
        unpinEntity(input: $input) {
          id
          pinnedOrder

          site {
            id
            ...DashboardLayout_PinnedEntities_site
          }
        }
      }
    `),
  );

  const paneGroup = getPaneGroup();
  const entities = $derived(site.data.pinnedEntities);
  const orderedItems = $derived(entities.map((entity) => ({ id: entity.id, pinnedOrder: entity.pinnedOrder ?? '' })));

  let visibleCount = $state(PAGE_SIZE);
  let activeSiteId = untrack(() => site.data.id);

  $effect(() => {
    const siteId = site.data.id;
    untrack(() => {
      if (siteId === activeSiteId) return;
      activeSiteId = siteId;
      visibleCount = PAGE_SIZE;
    });
  });

  $effect(() => {
    if (collapsed) visibleCount = PAGE_SIZE;
  });

  const visible = $derived(entities.slice(0, visibleCount));
  const hasMore = $derived(entities.length > visibleCount);

  let listElement = $state<HTMLUListElement>();
  let indicator = $state<DragIndicatorState>({});

  const resolveDrop = (x: number, y: number, item: EntityRowDragItem): EntityRowDrop | null => {
    const hit = document.elementFromPoint(x, y);
    const placement = listElement ? resolvePinnedPlacementAt(listElement, hit, y) : null;

    if (placement && listElement) {
      const orders = resolvePinnedOrders(orderedItems, [item.id], placement);
      indicator = orders ? pinnedPlacementIndicator(listElement, placement) : {};
      return orders ? { kind: 'reorder', ...orders } : null;
    }

    indicator = {};
    if (sectionElement && hit && sectionElement.contains(hit)) return null;
    return { kind: 'outside' };
  };

  const handleDrop = async (drop: EntityRowDropResult, item: EntityRowDragItem) => {
    if (drop.kind === 'reorder') {
      try {
        await pinEntities({ input: { entityIds: [item.id], lowerOrder: drop.lowerOrder, upperOrder: drop.upperOrder } });
        mixpanel.track('pin_entities', { totalCount: 1, via: 'drag_and_drop' });
      } catch {
        Toast.error('고정 중 오류가 발생했습니다');
      }
      return;
    }

    if (drop.kind === 'unpin') {
      try {
        await unpinEntity({ input: { entityId: item.id } });
        mixpanel.track('unpin_entity', { via: 'drag_and_drop' });
      } catch {
        Toast.error('고정 중 오류가 발생했습니다');
      }
    }
  };

  const rowDrag = new EntityRowDragController({
    paneGroup,
    resolveDrop,
    onDrop: (drop, item) => void handleDrop(drop, item),
    holdOutside: { ms: UNPIN_HOLD_MS, cue: '고정 해제' },
  });

  let dragScroll: ReturnType<typeof createDragScroll> | null = null;

  $effect(() => {
    const active = rowDrag.active;
    const surface = listElement?.closest<HTMLElement>('[data-entity-row-drag-scroll-surface]');
    if (!surface || !active) return;

    const ghost = untrack(() => rowDrag.ghost);
    const current = createDragScroll(elementScrollViewport(surface), {
      initialPointer: ghost ? { clientX: ghost.x, clientY: ghost.y } : undefined,
      onScroll: (clientX, clientY) => rowDrag.updatePointer(clientX, clientY),
    });
    dragScroll = current;

    return () => {
      current.destroy();
      if (dragScroll === current) dragScroll = null;
    };
  });

  $effect(() => {
    const ghost = rowDrag.ghost;
    if (ghost) dragScroll?.updatePointer(ghost.x, ghost.y);
  });

  onDestroy(() => rowDrag.destroy());
</script>

<svelte:window oncontextmenu={(event) => rowDrag.contextMenu(event)} />

<ul
  bind:this={listElement}
  class={flex({ flexDirection: 'column', flexShrink: '0', paddingX: '12px', paddingY: '4px', userSelect: 'none' })}
  aria-label="고정한 항목"
  data-drop-target="pin"
>
  {#if entities.length === 0}
    <li class={flex({ alignItems: 'center', height: '32px', paddingX: '8px' })}>
      <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.hint' })}>고정한 항목이 없어요</p>
    </li>
  {:else}
    {#each visible as entity (entity.id)}
      <li>
        {#if entity.node.__typename === 'Document'}
          <Document document$key={entity.node} {rowDrag} source="pinned" />
        {:else if entity.node.__typename === 'Folder'}
          <PinnedFolder folder$key={entity.node} {rowDrag} />
        {/if}
      </li>
    {/each}
  {/if}

  {#if hasMore}
    <li>
      <button
        class={flex({
          alignItems: 'center',
          width: 'full',
          paddingLeft: '50px',
          paddingRight: '8px',
          paddingY: '6px',
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          textAlign: 'left',
          transition: 'common',
          _supportHover: { color: 'text.default' },
        })}
        onclick={() => (visibleCount += PAGE_SIZE)}
        type="button"
      >
        더 보기
      </button>
    </li>
  {/if}
</ul>

{#if rowDrag.drop?.kind === 'reorder'}
  <DragIndicator {indicator} />
{/if}

{#if rowDrag.ghost}
  <EntityRowDragGhost ghost={rowDrag.ghost} />
{/if}
