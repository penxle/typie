<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoveryCardList from '../../../DiscoveryCardList.svelte';
  import DiscoveryPageHead from '../../../DiscoveryPageHead.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.tagQuery));
  const tag = $derived(query.data.discovery.tag);
</script>

{#if tag}
  <Helmet title={`#${tag.name}`} />

  <DiscoveryPageHead sub={`글 ${tag.count}개`} title={`#${tag.name}`} />

  {#key tag.name}
    <DiscoveryCardList
      initialHasMore={tag.publications.hasMore}
      publications={tag.publications.publications}
      source={{ kind: 'tag', name: tag.name }}
    />
  {/key}
{/if}
