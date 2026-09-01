<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { onDestroy, untrack } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import ArrowUpDownIcon from '~icons/lucide/arrow-up-down';
  import CheckIcon from '~icons/lucide/check';
  import { invalidateRecentDocuments, recentDocumentInvalidationVersions } from '$lib/graphql/recent-documents';
  import { graphql } from '$mearie';
  import { getPaneGroup } from '../[slug]/@pane/context.svelte';
  import { DocumentPaneDragController } from '../[slug]/@pane/document-pane-drag.svelte';
  import DocumentPaneDragGhost from '../[slug]/@pane/DocumentPaneDragGhost.svelte';
  import Document from './Document.svelte';
  import SidebarSectionHeader from './SidebarSectionHeader.svelte';
  import { getTreeContext } from './state.svelte';
  import type { RecentDocumentSort } from '$lib/graphql/recent-documents';
  import type { DashboardLayout_RecentDocuments_site$key } from '$mearie';
  import type { TreeEntity } from './@selection/types';

  type Props = {
    site$key: DashboardLayout_RecentDocuments_site$key;
    canScrollUp: boolean;
    headerHeight?: number;
  };

  let { site$key, canScrollUp, headerHeight = $bindable(0) }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_RecentDocuments_site on Site {
        id

        recentlyViewedDocuments: recentDocuments(sort: VIEWED_AT, limit: 5) {
          hasMore

          documents {
            id

            entity {
              id
              icon
              iconColor

              parent {
                id
              }
            }

            ...DashboardLayout_EntityTree_Document_document
          }
        }

        recentlyUpdatedDocuments: recentDocuments(sort: UPDATED_AT, limit: 5) {
          hasMore

          documents {
            id

            entity {
              id
              icon
              iconColor

              parent {
                id
              }
            }

            ...DashboardLayout_EntityTree_Document_document
          }
        }
      }
    `),
    () => site$key,
  );

  const app = getAppContext();
  const treeState = getTreeContext();
  const documentPaneDrag = new DocumentPaneDragController({ paneGroup: getPaneGroup() });
  let siteId = $derived(site.data.id);
  let open = $derived(app.preference.current.sidebarRecentDocumentsOpen);
  let sort = $derived(app.preference.current.sidebarRecentDocumentsSort);
  let expansion = $state<{ key: string; count: number }>();
  let listKey = $derived(`${siteId}:${sort}`);
  let visibleCount = $derived(expansion?.key === listKey ? expansion.count : 5);
  let queryKey = $derived(`${listKey}:${visibleCount}`);
  let invalidationVersion = $derived($recentDocumentInvalidationVersions.get(siteId)?.[sort] ?? 0);
  // 첫 5개는 레이아웃 프래그먼트가 소유하므로 추가 로딩이나 갱신이 필요한 목록만 별도 쿼리를 활성화한다.
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

              entity {
                id
                icon
                iconColor

                parent {
                  id
                }
              }

              ...DashboardLayout_EntityTree_Document_document
            }
          }
        }
      }
    `),
    () => ({ siteId, sort, limit: visibleCount }),
    () => ({ fetchPolicy: 'cache-and-network', skip: !enabledQueries.has(queryKey) }),
  );

  const initialPage = $derived(sort === 'VIEWED_AT' ? site.data.recentlyViewedDocuments : site.data.recentlyUpdatedDocuments);
  type RecentDocumentsPage = (typeof site.data)['recentlyViewedDocuments'];
  let pendingPage = $state<{ queryKey: string; page: RecentDocumentsPage }>();
  const queryPage = $derived(query.data?.site.id === siteId ? query.data.site.recentDocuments : undefined);
  const page = $derived(queryPage ?? (pendingPage?.queryKey === queryKey ? pendingPage.page : initialPage));
  const documents = $derived(page.documents);
  const visibleDocumentIds = $derived(documents.map((document) => document.entity.id));

  const toggleOpen = () => {
    app.preference.current.sidebarRecentDocumentsOpen = !open;
    if (open && prefersReducedMotion.current) expansion = undefined;
  };

  const handleRevealTransitionEnd = (event: TransitionEvent) => {
    if (!open && event.target === event.currentTarget && event.propertyName === 'grid-template-rows') {
      expansion = undefined;
    }
  };

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

  $effect(() => {
    treeState.recentEntityMap = new SvelteMap<string, TreeEntity>(
      documents.map((document) => {
        const entity = document.entity;
        return [
          entity.id,
          {
            id: entity.id,
            type: 'Document' as const,
            icon: entity.icon,
            iconColor: entity.iconColor,
            parentId: entity.parent?.id,
          },
        ];
      }),
    );
  });

  const sortOptions: { value: RecentDocumentSort; label: string }[] = [
    { value: 'VIEWED_AT', label: '최근 본 순서' },
    { value: 'UPDATED_AT', label: '최근 수정한 순서' },
  ];

  onDestroy(() => documentPaneDrag.destroy());
</script>

<svelte:window oncontextmenu={(event) => documentPaneDrag.contextMenu(event)} />

<section class={css({ flexShrink: '0', marginBottom: '4px' })}>
  <SidebarSectionHeader dividerVisible={canScrollUp} label="최근" onToggle={toggleOpen} {open} bind:height={headerHeight}>
    {#snippet actions()}
      <Menu
        style={css.raw({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '4px',
          size: '24px',
          color: 'text.faint',
          opacity: '50',
          transition: 'common',
          _hover: { color: 'text.subtle', opacity: '100' },
          _focusVisible: { opacity: '100' },
          _expanded: { color: 'text.subtle', backgroundColor: 'surface.muted', opacity: '100' },
        })}
        buttonAriaLabel="최근 문서 정렬"
        placement="bottom-start"
      >
        {#snippet button()}
          <Icon icon={ArrowUpDownIcon} size={14} />
        {/snippet}

        <div
          class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', fontWeight: 'medium', color: 'text.disabled' })}
          role="presentation"
        >
          정렬 기준
        </div>

        {#each sortOptions as option (option.value)}
          <MenuItem
            aria-checked={sort === option.value}
            onclick={() => (app.preference.current.sidebarRecentDocumentsSort = option.value)}
            role="menuitemradio"
          >
            {option.label}
            {#if sort === option.value}
              <Icon style={css.raw({ marginLeft: 'auto', color: 'text.brand' })} icon={CheckIcon} size={14} />
            {/if}
          </MenuItem>
        {/each}
      </Menu>
    {/snippet}
  </SidebarSectionHeader>

  <div
    class={css({
      display: 'grid',
      gridTemplateRows: open ? '1fr' : '0fr',
      transition: '[grid-template-rows 160ms ease-out]',
      _motionReduce: { transition: '[none]' },
    })}
    aria-hidden={!open}
    inert={!open}
    ontransitionend={handleRevealTransitionEnd}
  >
    <div
      style:opacity={open ? '1' : '0'}
      class={css({
        minHeight: '0',
        overflow: 'hidden',
        transition: '[opacity 120ms ease-out]',
        _motionReduce: { transition: '[none]' },
      })}
    >
      <ul
        class={flex({ flexDirection: 'column', flexShrink: '0', paddingX: '12px', paddingY: '4px', userSelect: 'none' })}
        aria-label="최근 문서"
      >
        {#if documents.length === 0}
          <li class={center({ height: '32px' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
              {sort === 'VIEWED_AT' ? '최근 본 문서가 없어요' : '최근 수정한 문서가 없어요'}
            </p>
          </li>
        {:else}
          {#each documents as document (document.id)}
            <li>
              <Document document$key={document} {documentPaneDrag} selectionOrder={visibleDocumentIds} source="recent" />
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
    </div>
  </div>
</section>

{#if documentPaneDrag.ghost}
  <DocumentPaneDragGhost ghost={documentPaneDrag.ghost} />
{/if}
