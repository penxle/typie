<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoveryCardList from '../DiscoveryCardList.svelte';
  import DiscoveryPageHead from '../DiscoveryPageHead.svelte';

  let { data } = $props();

  const feedQuery = $derived(hydrateQuery(() => data.feedQuery));
  const feed = $derived(feedQuery.data.discovery.feed);
</script>

<Helmet title="타이피" trailing={null} />

<section>
  <DiscoveryPageHead title="홈" />
  <DiscoveryCardList initialHasMore={feed.hasMore} publications={feed.publications} source={{ kind: 'feed' }} />
</section>
