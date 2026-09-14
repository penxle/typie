<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { TextInput } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import SearchIcon from '~icons/lucide/search';
  import { graphql } from '$mearie';
  import TagChip from '../@[slug]/TagChip.svelte';
  import { discoveryTagPath } from './paths';

  type Props = {
    id: string;
    query: string;
    currentTag: string | null;
    onclose: () => void;
  };

  let { id, query = $bindable(), currentTag, onclose }: Props = $props();

  const tagsQuery = createQuery(
    graphql(`
      query UsersiteApex_ApexSearchPanel_Query {
        discovery {
          tags {
            name
            count
          }
        }
      }
    `),
  );

  const tags = $derived(tagsQuery.data?.discovery.tags.slice(0, 12) ?? []);

  let input = $state<HTMLInputElement>();

  onMount(() => {
    input?.focus({ preventScroll: true });
  });
</script>

<div
  class={css({
    position: 'fixed',
    inset: '0',
    top: '[var(--usersite-sticky-header-bottom, 52px)]',
    zIndex: '1',
    backgroundColor: '[rgba(10, 11, 14, 0.24)]',
    animation: '[apex-search-fade 180ms ease-out]',
    _motionReduce: { animation: '[none]' },
  })}
  aria-hidden="true"
  data-search-scrim
  onclick={onclose}
></div>

<div
  {id}
  class={css({
    position: 'absolute',
    top: '[100%]',
    left: '0',
    right: '0',
    zIndex: '2',
    borderBottomWidth: '1px',
    borderColor: 'border.default',
    backgroundColor: 'surface.default',
    boxShadow: 'lg',
    transformOrigin: 'top',
    animation: '[apex-search-drop 220ms cubic-bezier(0.23, 1, 0.32, 1)]',
    _motionReduce: { animation: '[none]' },
  })}
  data-search-panel
>
  <div
    class={flex({
      flexDirection: 'column',
      gap: '20px',
      maxWidth: '760px',
      marginX: 'auto',
      paddingTop: { base: '16px', md: '24px' },
      paddingBottom: { base: '20px', md: '28px' },
      paddingX: { base: '20px', md: '40px' },
    })}
  >
    <form action="/search" method="GET">
      <TextInput
        name="q"
        autocomplete="off"
        leftIcon={SearchIcon}
        placeholder="글, 스페이스, 태그 검색"
        size="lg"
        type="search"
        bind:element={input}
        bind:value={query}
      />
    </form>

    {#if tags.length > 0}
      <div>
        <h3 class={css({ marginBottom: '10px', fontSize: '13px', fontWeight: 'semibold', color: 'text.hint' })}>태그</h3>
        <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
          {#each tags as tag (tag.name)}
            <TagChip
              name={tag.name}
              count={tag.count}
              current={tag.name === currentTag}
              href={discoveryTagPath(tag.name)}
              noscroll={false}
              size="lg"
            />
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes -global-apex-search-fade {
    from {
      opacity: 0;
    }
  }

  @keyframes -global-apex-search-drop {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
  }
</style>
