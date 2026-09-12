<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoveryListSection from '../../DiscoveryListSection.svelte';
  import DiscoveryTagChips from '../../DiscoveryTagChips.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.tagQuery));
  const discovery = $derived(query.data.discovery);
  const tag = $derived(discovery.tag);
</script>

{#if tag}
  <Helmet title={`#${tag.name}`} />

  <div class={css({ width: 'full', maxWidth: '[760px]', marginX: 'auto', paddingX: { base: '20px', md: '40px' }, paddingBottom: '120px' })}>
    <DiscoveryTagChips active={tag.name} tags={discovery.tags} />

    <div class={css({ paddingTop: '16px', paddingBottom: '14px', borderBottomWidth: '1px', borderColor: 'border.hairline' })}>
      <h1 class={css({ fontSize: '18px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>#{tag.name}</h1>
      <div class={css({ marginTop: '8px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
        글 {tag.count}개
      </div>
    </div>

    {#key tag.name}
      <DiscoveryListSection initialHasMore={tag.publications.hasMore} publications={tag.publications.publications} tagName={tag.name} />
    {/key}
  </div>
{/if}
