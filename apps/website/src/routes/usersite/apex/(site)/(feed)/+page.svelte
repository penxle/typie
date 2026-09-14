<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoveryCardList from '../DiscoveryCardList.svelte';
  import DiscoverySectionHead from '../DiscoverySectionHead.svelte';

  let { data } = $props();

  const feedQuery = $derived(hydrateQuery(() => data.feedQuery));
  const publications = $derived(feedQuery.data.discovery.publications);
</script>

<Helmet title="타이피" trailing={null} />

<section>
  <DiscoverySectionHead title="최신 글" />
  <DiscoveryCardList initialHasMore={publications.hasMore} publications={publications.publications} />
</section>
