<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import CheckIcon from '~icons/lucide/check';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import FolderIcon from '~icons/lucide/folder';
  import SearchIcon from '~icons/lucide/search';
  import { getNoteOperationsContext } from '$lib/note/note-mutation';
  import { graphql } from '$mearie';
  import EntityIcon from '../@context-menu/EntityIcon.svelte';
  import type { EntityIcon_entity$key } from '$mearie';

  type Props = {
    noteId: string;
    existingEntityIds: string[];
    open: boolean;
    onclose: () => void;
  };

  let { noteId, existingEntityIds, open, onclose }: Props = $props();

  const app = getAppContext();
  const noteOperations = getNoteOperationsContext();
  const currentSiteId = $derived(app.preference.current.currentSiteId ?? null);

  let query = $state('');
  let selectedIndex = $state(0);
  let debounceTimeout: ReturnType<typeof setTimeout> | null = null;
  let debouncedQuery = $state('');
  let addRequestId = 0;
  let activeAddRequest: { id: number; noteId: string; siteId: string } | null = null;
  let activeAddIdentity: string | null = null;

  $effect(() => {
    const identity = open && currentSiteId ? JSON.stringify([currentSiteId, noteId]) : null;
    if (activeAddIdentity !== identity) activeAddRequest = null;
    activeAddIdentity = identity;
  });

  $effect(() => {
    if (!open) {
      return;
    }

    query = '';
    debouncedQuery = '';
    selectedIndex = 0;
  });

  const handleQueryInput = () => {
    if (debounceTimeout) clearTimeout(debounceTimeout);
    debounceTimeout = setTimeout(() => {
      debouncedQuery = query;
      selectedIndex = 0;
    }, 300);
  };

  const recentQuery = createQuery(
    graphql(`
      query NoteEntitySearchModal_Recent_Query($siteId: ID) {
        me @required {
          id
          recentlyViewedEntities(siteId: $siteId) {
            id
            slug
            site {
              id
            }
            ...EntityIcon_entity
            node {
              __typename
              ... on Document {
                id
                title
              }
              ... on Folder {
                id
                name
              }
            }
          }
        }
      }
    `),
    () => ({ siteId: currentSiteId }),
    () => ({ skip: !open || !currentSiteId }),
  );

  const searchQuery = createQuery(
    graphql(`
      query NoteEntitySearchModal_Search_Query($query: String!, $siteId: ID!) {
        search(query: $query, siteId: $siteId) {
          hits {
            __typename
            ... on SearchHitDocument {
              document {
                id
                title
                entity {
                  id
                  slug
                  site {
                    id
                  }
                  ...EntityIcon_entity
                }
              }
            }
            ... on SearchHitFolder {
              folder {
                id
                title: name
                entity {
                  id
                  slug
                  site {
                    id
                  }
                  ...EntityIcon_entity
                }
              }
            }
          }
        }
      }
    `),
    () => ({ query: debouncedQuery, siteId: currentSiteId ?? '' }),
    () => ({ skip: !debouncedQuery || !open || !currentSiteId }),
  );

  type ResultItem = {
    entityId: string;
    slug: string;
    title: string;
    type: 'document' | 'folder';
    isLinked: boolean;
    entity: EntityIcon_entity$key;
  };

  const results = $derived.by((): ResultItem[] => {
    if (debouncedQuery && searchQuery.data?.search) {
      return searchQuery.data.search.hits
        .map((hit): ResultItem | null => {
          if (hit.__typename === 'SearchHitDocument' && hit.document?.entity.site.id === currentSiteId) {
            return {
              entityId: hit.document.entity.id,
              slug: hit.document.entity.slug,
              title: hit.document.title || '(제목 없음)',
              type: 'document',
              isLinked: existingEntityIds.includes(hit.document.entity.id),
              entity: hit.document.entity,
            };
          }
          if (hit.__typename === 'SearchHitFolder' && hit.folder?.entity.site.id === currentSiteId) {
            return {
              entityId: hit.folder.entity.id,
              slug: hit.folder.entity.slug,
              title: hit.folder.title || '(제목 없음)',
              type: 'folder',
              isLinked: existingEntityIds.includes(hit.folder.entity.id),
              entity: hit.folder.entity,
            };
          }
          return null;
        })
        .filter((item): item is ResultItem => item !== null);
    }

    if (recentQuery.data?.me) {
      return recentQuery.data.me.recentlyViewedEntities
        .filter((entity) => entity.site.id === currentSiteId)
        .slice(0, 10)
        .map((entity) => ({
          entityId: entity.id,
          slug: entity.slug,
          title:
            entity.node.__typename === 'Document'
              ? entity.node.title || '(제목 없음)'
              : entity.node.__typename === 'Folder'
                ? entity.node.name || '(제목 없음)'
                : '(제목 없음)',
          type: entity.node.__typename === 'Folder' ? ('folder' as const) : ('document' as const),
          isLinked: existingEntityIds.includes(entity.id),
          entity,
        }));
    }

    return [];
  });
  const activeQuery = $derived(debouncedQuery ? searchQuery : recentQuery);
  const emptyMessage = $derived(
    activeQuery.loading
      ? debouncedQuery
        ? '검색 중...'
        : '불러오는 중...'
      : debouncedQuery
        ? '검색 결과가 없어요.'
        : '최근 항목이 없어요.',
  );
  const errorMessage = $derived(
    activeQuery.error ? (debouncedQuery ? '검색 결과를 불러오지 못했어요.' : '최근 항목을 불러오지 못했어요.') : null,
  );

  $effect(() => {
    const resultCount = results.length;
    if (resultCount === 0 || !Number.isSafeInteger(selectedIndex) || selectedIndex >= resultCount) {
      selectedIndex = 0;
    }
  });

  const retryActiveQuery = () => {
    activeQuery.refetch();
  };

  const handleSelect = async (item: ResultItem) => {
    if (activeAddRequest !== null || item.isLinked) return;

    const siteId = currentSiteId;
    const selectedNoteId = noteId;
    if (!siteId || !selectedNoteId) return;

    const requestId = ++addRequestId;
    activeAddRequest = { id: requestId, noteId: selectedNoteId, siteId };
    try {
      const outcome = await noteOperations.addEntity(
        { noteId: selectedNoteId, entityId: item.entityId },
        {
          lastKnown: { siteId, noteId: selectedNoteId },
        },
      );
      if (currentSiteId !== siteId || noteId !== selectedNoteId || activeAddRequest?.id !== requestId) return;
      if (outcome.status === 'failure') {
        Toast.error('연결을 추가하지 못했어요.');
      } else if (outcome.status !== 'subscription_gated') {
        onclose();
      }
    } finally {
      if (activeAddRequest?.id === requestId) activeAddRequest = null;
    }
  };

  const scrollSelectedIntoView = () => {
    const el = document.querySelector(`[data-note-search-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (errorMessage) return;
    if (results.length === 0) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % results.length;
      scrollSelectedIntoView();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + results.length) % results.length;
      scrollSelectedIntoView();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const item = results[selectedIndex];
      if (item && !item.isLinked) {
        handleSelect(item);
      }
    }
  };
</script>

<Modal
  style={css.raw({
    maxWidth: '480px',
    height: '460px',
    maxHeight: 'full',
    padding: '0',
  })}
  {onclose}
  {open}
>
  <div class={flex({ flexDirection: 'column', width: 'full', height: 'full' })}>
    <div
      class={flex({
        alignItems: 'center',
        gap: '10px',
        paddingX: '16px',
        paddingY: '12px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
      })}
    >
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={SearchIcon} size={18} />
      <input
        class={css({
          flexGrow: '1',
          fontSize: '15px',
          color: 'text.default',
        })}
        oninput={handleQueryInput}
        onkeydown={handleKeyDown}
        placeholder="항목 검색..."
        bind:value={query}
      />
    </div>

    <div
      class={flex({
        flexDirection: 'column',
        flexGrow: '1',
        overflowY: 'auto',
        scrollbarWidth: 'none',
        paddingY: '4px',
      })}
    >
      {#if errorMessage}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            gap: '12px',
            paddingX: '16px',
            paddingY: '32px',
            color: 'text.faint',
            fontSize: '13px',
          })}
        >
          <span>{errorMessage}</span>
          <Button onclick={retryActiveQuery} size="sm" variant="secondary">다시 시도</Button>
        </div>
      {:else if results.length === 0}
        <div class={center({ paddingY: '32px', color: 'text.faint', fontSize: '13px' })}>
          {emptyMessage}
        </div>
      {:else}
        {#each results as item, index (item.entityId)}
          <button
            class={flex({
              alignItems: 'center',
              gap: '10px',
              paddingX: '16px',
              paddingY: '8px',
              cursor: item.isLinked ? 'default' : 'pointer',
              opacity: item.isLinked ? '50' : '100',
              backgroundColor: index === selectedIndex ? 'surface.muted' : 'transparent',
            })}
            data-note-search-index={index}
            onclick={() => handleSelect(item)}
            onpointermove={() => (selectedIndex = index)}
            type="button"
          >
            <EntityIcon
              style={css.raw({ flexShrink: '0' })}
              entity$key={item.entity}
              fallback={item.type === 'folder' ? FolderIcon : undefined}
              size={16}
            />
            <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', textAlign: 'left', lineClamp: '1' })}>
              {item.title}
            </span>
            {#if item.isLinked}
              <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'accent.success.default' })} icon={CheckIcon} size={16} />
            {/if}
          </button>
        {/each}
      {/if}
    </div>

    <div
      class={flex({
        alignItems: 'center',
        gap: '16px',
        paddingX: '16px',
        paddingY: '8px',
        borderTopWidth: '1px',
        borderColor: 'border.subtle',
        fontSize: '11px',
        color: 'text.faint',
        flexShrink: '0',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <div class={flex({ alignItems: 'center' })}>
          <Icon icon={ArrowUpIcon} size={10} />
          <Icon icon={ArrowDownIcon} size={10} />
        </div>
        <span>이동</span>
      </div>
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <Icon icon={CornerDownLeftIcon} size={10} />
        <span>연결</span>
      </div>
      <span>ESC 닫기</span>
    </div>
  </div>
</Modal>
