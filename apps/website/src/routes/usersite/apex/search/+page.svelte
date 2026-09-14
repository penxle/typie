<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import SearchResults from '../SearchResults.svelte';

  let { data } = $props();

  const title = $derived(data.q ? `‘${data.q}’ 검색 결과` : '검색');
</script>

<Helmet {title} />

<svelte:head>
  <meta name="robots" content="noindex, follow" />
</svelte:head>

<div
  class={css({
    width: 'full',
    maxWidth: '[760px]',
    marginX: 'auto',
    paddingX: { base: '20px', md: '40px' },
    paddingTop: '28px',
    paddingBottom: '120px',
  })}
>
  <h1 class={css({ marginBottom: '24px', fontSize: '18px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>
    {title}
  </h1>

  {#if data.searchQuery}
    <SearchResults searchQuery={data.searchQuery} />
  {:else if data.failed}
    <p class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>검색 결과를 불러오지 못했어요.</p>
  {:else}
    <p class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>검색어를 입력해 주세요.</p>
  {/if}
</div>
