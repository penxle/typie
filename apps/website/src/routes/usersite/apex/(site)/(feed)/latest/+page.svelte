<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoveryCardList from '../../DiscoveryCardList.svelte';
  import DiscoveryPageHead from '../../DiscoveryPageHead.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.latestQuery));
  const publications = $derived(query.data.discovery.publications);
</script>

<Helmet title="최신 글" />

<DiscoveryPageHead title="최신 글" />
<DiscoveryCardList initialHasMore={publications.hasMore} publications={publications.publications} source={{ kind: 'publications' }} />
