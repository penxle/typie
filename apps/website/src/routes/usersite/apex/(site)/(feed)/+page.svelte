<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { latestOfSpaceRuns } from '$lib/discovery/feed-grouping';
  import { hydrateQuery } from '$lib/graphql';
  import { discoveryCardList } from '../discovery-styles';
  import DiscoveryCard from '../DiscoveryCard.svelte';
  import DiscoverySectionHead from '../DiscoverySectionHead.svelte';
  import { discoveryLatestPath } from '../paths';

  let { data } = $props();

  const feedQuery = $derived(hydrateQuery(() => data.feedQuery));

  const discovery = $derived(feedQuery.data.discovery);
  const latest = $derived(latestOfSpaceRuns(discovery.publications.publications).slice(0, 9));
</script>

<Helmet title="타이피" trailing={null} />

<section>
  <DiscoverySectionHead moreHref={discoveryLatestPath} title="최신 글" />
  {#if latest.length > 0}
    <div class={css(discoveryCardList)}>
      {#each latest as publication (publication.id)}
        <DiscoveryCard publicationView$key={publication} />
      {/each}
    </div>
  {:else}
    <p class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>아직 발행된 글이 없어요</p>
  {/if}
</section>
