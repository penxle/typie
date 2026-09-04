<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, untrack } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import { invalidateRecentDocuments, recentDocumentInvalidationVersions } from '$lib/graphql/recent-documents';
  import { graphql } from '$mearie';
  import { getPaneGroup } from '../[slug]/@pane/context.svelte';
  import Document from './Document.svelte';
  import DragIndicator from './DragIndicator.svelte';
  import { EntityRowDragController } from './entity-row-drag.svelte';
  import EntityRowDragGhost from './EntityRowDragGhost.svelte';
  import type { DashboardLayout_RecentDocuments_site$key } from '$mearie';
  import type { DragIndicatorState } from './DragIndicator.svelte';
  import type { EntityRowDragItem, EntityRowDrop, EntityRowDropResult } from './entity-row-drag.svelte';

  type Props = {
    site$key: DashboardLayout_RecentDocuments_site$key;
    open: boolean;
    collapsed: boolean;
  };

  let { site$key, open, collapsed }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_RecentDocuments_site on Site {
        id

        recentlyViewedDocuments: recentDocuments(sort: VIEWED_AT, limit: 5) {
          hasMore

          documents {
            id

            ...DashboardLayout_EntityTree_Document_document
          }
        }

        recentlyUpdatedDocuments: recentDocuments(sort: UPDATED_AT, limit: 5) {
          hasMore

          documents {
            id

            ...DashboardLayout_EntityTree_Document_document
          }
        }
      }
    `),
    () => site$key,
  );

  const app = getAppContext();

  let pinIndicator = $state<DragIndicatorState>({});

  const resolveDrop = (x: number, y: number): EntityRowDrop | null => {
    const target = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop-target="pin"]');
    if (!target) {
      pinIndicator = {};
      return null;
    }

    const rect = target.getBoundingClientRect();
    pinIndicator = { top: rect.top, left: rect.left, width: rect.width, height: rect.height, opacity: 0.5, transform: undefined };
    return { kind: 'pin' };
  };

  const handleDrop = async (drop: EntityRowDropResult, item: EntityRowDragItem) => {
    if (drop.kind !== 'pin') return;

    try {
      await pinEntities({ input: { entityIds: [item.id] } });
      mixpanel.track('pin_entities', { totalCount: 1, via: 'drag_and_drop' });
      app.preference.current.sidebarQuickAccessTab = 'PINNED';
      app.preference.current.sidebarRecentDocumentsOpen = true;
    } catch {
      Toast.error('고정 중 오류가 발생했습니다');
    }
  };

  const rowDrag = new EntityRowDragController({
    paneGroup: getPaneGroup(),
    resolveDrop,
    onDrop: (drop, item) => void handleDrop(drop, item),
  });

  let siteId = $derived(site.data.id);
  let sort = $derived(app.preference.current.sidebarRecentDocumentsSort);
  let expansion = $state<{ key: string; count: number }>();
  let listKey = $derived(`${siteId}:${sort}`);
  let visibleCount = $derived(expansion?.key === listKey ? expansion.count : 5);
  let queryKey = $derived(`${listKey}:${visibleCount}`);
  let invalidationVersion = $derived($recentDocumentInvalidationVersions.get(siteId)?.[sort] ?? 0);
  const enabledQueries = new SvelteSet<string>();
  const handledInvalidations = new SvelteMap<string, number>();
  let activeListKey = untrack(() => listKey);
  let activeSiteId = untrack(() => siteId);

  const query = createQuery(
    graphql(`
      query DashboardLayout_RecentDocuments_Query($siteId: ID!, $sort: RecentDocumentSort!, $limit: Int!) {
        site(siteId: $siteId) {
          id

          recentDocuments(sort: $sort, limit: $limit) {
            hasMore

            documents {
              id

              ...DashboardLayout_EntityTree_Document_document
            }
          }
        }
      }
    `),
    () => ({ siteId, sort, limit: visibleCount }),
    () => ({ fetchPolicy: 'network-only', skip: !enabledQueries.has(queryKey) }),
  );

  const [pinEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_RecentDocuments_PinEntities_Mutation($input: PinEntitiesInput!) {
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

  const initialPage = $derived(sort === 'VIEWED_AT' ? site.data.recentlyViewedDocuments : site.data.recentlyUpdatedDocuments);
  type RecentDocumentsPage = (typeof site.data)['recentlyViewedDocuments'];
  let pendingPage = $state<{ queryKey: string; page: RecentDocumentsPage }>();
  const queryPage = $derived(query.data?.site.id === siteId ? query.data.site.recentDocuments : undefined);
  const page = $derived(queryPage ?? (pendingPage?.queryKey === queryKey ? pendingPage.page : initialPage));
  const documents = $derived(page.documents);

  $effect(() => {
    if (collapsed) expansion = undefined;
  });

  $effect(() => {
    const nextListKey = listKey;
    const nextSiteId = siteId;

    untrack(() => {
      if (nextListKey === activeListKey) return;

      activeListKey = nextListKey;
      expansion = undefined;
      if (nextSiteId === activeSiteId) return;

      activeSiteId = nextSiteId;
      invalidateRecentDocuments(nextSiteId);
    });
  });

  $effect(() => {
    const key = queryKey;
    const version = invalidationVersion;
    if (!open || version === 0) return;

    untrack(() => {
      if (handledInvalidations.get(key) === version) return;
      handledInvalidations.set(key, version);

      const enabled = enabledQueries.has(key);
      enabledQueries.add(key);
      if (enabled) query.refetch();
    });
  });

  const showMore = () => {
    if (query.loading || !page.hasMore) return;
    if (query.error) {
      query.refetch();
      return;
    }

    const nextCount = Math.min(visibleCount + 5, 50);
    const nextKey = `${listKey}:${nextCount}`;
    pendingPage = { queryKey: nextKey, page };
    handledInvalidations.set(nextKey, invalidationVersion);
    enabledQueries.add(nextKey);
    expansion = { key: listKey, count: nextCount };
  };

  onDestroy(() => rowDrag.destroy());
</script>

<svelte:window oncontextmenu={(event) => rowDrag.contextMenu(event)} />

<ul
  class={flex({ flexDirection: 'column', flexShrink: '0', paddingX: '12px', paddingY: '4px', userSelect: 'none' })}
  aria-label="최근 문서"
>
  {#if documents.length === 0}
    <li class={flex({ alignItems: 'center', height: '32px', paddingX: '8px' })}>
      <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.disabled' })}>
        {sort === 'VIEWED_AT' ? '최근 본 문서가 없어요' : '최근 수정한 문서가 없어요'}
      </p>
    </li>
  {:else}
    {#each documents as document (document.id)}
      <li>
        <Document document$key={document} {rowDrag} source="recent" />
      </li>
    {/each}
  {/if}

  {#if page.hasMore}
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
          color: 'text.disabled',
          textAlign: 'left',
          transition: 'common',
          _supportHover: { color: 'text.muted' },
        })}
        disabled={query.loading}
        onclick={showMore}
        type="button"
      >
        더 보기
      </button>
    </li>
  {/if}
</ul>

{#if rowDrag.drop?.kind === 'pin'}
  <DragIndicator indicator={pinIndicator} />
{/if}

{#if rowDrag.ghost}
  <EntityRowDragGhost ghost={rowDrag.ghost} />
{/if}
