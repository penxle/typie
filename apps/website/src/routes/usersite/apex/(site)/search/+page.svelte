<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet, TextInput } from '@typie/ui/components';
  import SearchIcon from '~icons/lucide/search';
  import { getUsersiteChrome } from '../../../chrome.svelte';
  import SearchResults from '../SearchResults.svelte';

  let { data } = $props();

  const chrome = getUsersiteChrome();

  const title = $derived(data.q ? `‘${data.q}’ 검색 결과` : '검색');

  let query = $state(data.q);
  let input = $state<HTMLInputElement>();

  $effect(() => {
    query = data.q;
  });

  $effect(() => {
    chrome.searchInput = input ?? null;
    return () => {
      chrome.searchInput = null;
    };
  });

  const message = css.raw({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' });
</script>

<Helmet {title} />

<svelte:head>
  <meta name="robots" content="noindex, follow" />
</svelte:head>

<div
  class={css({
    display: 'grid',
    gridTemplateColumns: { base: 'minmax(0, 1fr)', lg: 'minmax(0, 1fr) 300px' },
    columnGap: '56px',
    alignItems: 'start',
    width: 'full',
    maxWidth: { base: '760px', lg: '1200px' },
    marginX: 'auto',
    paddingTop: { base: '20px', md: '28px' },
    paddingX: { base: '20px', md: '40px' },
    paddingBottom: '120px',
    wordBreak: 'keep-all',
    overflowWrap: 'anywhere',
  })}
>
  <h1 class={css({ srOnly: true })}>{title}</h1>

  <form class={css({ marginBottom: '12px', lg: { gridColumn: '1', gridRow: '1' } })} action="/search" method="GET" role="search">
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

  {#if data.searchQuery}
    <SearchResults q={data.q} searchQuery={data.searchQuery} />
  {:else}
    <section class={css({ minWidth: '0', lg: { gridColumn: '1', gridRow: '2' } })}>
      <p class={css(message)}>{data.failed ? '검색 결과를 불러오지 못했어요.' : '검색어를 입력해 주세요.'}</p>
    </section>
  {/if}
</div>
