<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoverySectionHead from '../../../(site)/DiscoverySectionHead.svelte';
  import SeriesCards from '../../SeriesCards.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const space = $derived(query.data.spaceView);
</script>

<Helmet description={space.description ?? space.name} title="시리즈" trailing={space.name} />

<DiscoverySectionHead count={space.collections.length} title="시리즈" />

<SeriesCards collections={space.collections} />
