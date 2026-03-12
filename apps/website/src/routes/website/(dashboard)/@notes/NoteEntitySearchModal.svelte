<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import CheckIcon from '~icons/lucide/check';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import SearchIcon from '~icons/lucide/search';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';

  type Props = {
    noteId: string;
    existingEntityIds: string[];
    open: boolean;
    onclose: () => void;
  };

  let { noteId, existingEntityIds, open, onclose }: Props = $props();

  const app = getAppContext();

  let query = $state('');
  let selectedIndex = $state(0);
  let debounceTimeout: ReturnType<typeof setTimeout> | null = null;
  let debouncedQuery = $state('');

  $effect(() => {
    if (open) {
      query = '';
      debouncedQuery = '';
      selectedIndex = 0;
    }
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
    () => ({ siteId: app.preference.current.currentSiteId }),
    () => ({ skip: !open }),
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
                }
              }
            }
          }
        }
      }
    `),
    () => ({ query: debouncedQuery, siteId: app.preference.current.currentSiteId ?? '' }),
    () => ({ skip: !debouncedQuery || !open }),
  );

  const [addNoteEntity] = createMutation(
    graphql(`
      mutation NoteEntitySearchModal_AddNoteEntity_Mutation($input: AddNoteEntityInput!) {
        addNoteEntity(input: $input) {
          id
        }
      }
    `),
  );

  type ResultItem = {
    entityId: string;
    slug: string;
    title: string;
    type: 'document' | 'folder';
    isLinked: boolean;
  };

  const results = $derived.by((): ResultItem[] => {
    if (debouncedQuery && searchQuery.data?.search) {
      return searchQuery.data.search.hits
        .map((hit) => {
          if (hit.__typename === 'SearchHitDocument' && hit.document) {
            return {
              entityId: hit.document.entity.id,
              slug: hit.document.entity.slug,
              title: hit.document.title || '(제목 없음)',
              type: 'document' as const,
              isLinked: existingEntityIds.includes(hit.document.entity.id),
            };
          }
          if (hit.__typename === 'SearchHitFolder' && hit.folder) {
            return {
              entityId: hit.folder.entity.id,
              slug: hit.folder.entity.slug,
              title: hit.folder.title || '(제목 없음)',
              type: 'folder' as const,
              isLinked: existingEntityIds.includes(hit.folder.entity.id),
            };
          }
          return null;
        })
        .filter((item): item is ResultItem => item !== null);
    }

    if (recentQuery.data?.me) {
      return recentQuery.data.me.recentlyViewedEntities.slice(0, 10).map((entity) => ({
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
      }));
    }

    return [];
  });

  const handleSelect = async (item: ResultItem) => {
    if (item.isLinked) return;
    await addNoteEntity({ input: { noteId, entityId: item.entityId } });
    cache.invalidate({ __typename: 'Query', $field: 'notes' });
    cache.invalidate({ __typename: 'Entity', id: item.entityId, $field: 'notes' });
    onclose();
  };

  const scrollSelectedIntoView = () => {
    const el = document.querySelector(`[data-note-search-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  };

  const handleKeyDown = (e: KeyboardEvent) => {
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
      {#if results.length === 0}
        <div class={center({ paddingY: '32px', color: 'text.faint', fontSize: '13px' })}>
          {debouncedQuery ? '검색 결과가 없습니다' : '최근 항목이 없습니다'}
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
            <Icon
              style={css.raw({ flexShrink: '0', color: 'text.faint' })}
              icon={item.type === 'folder' ? FolderIcon : FileIcon}
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
