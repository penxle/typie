<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import SeriesView from '../../../SeriesView.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.seriesQuery));
  const space = $derived(query.data.spaceView);
  const collection = $derived(space.collection);
</script>

<Helmet description={collection.description ?? space.name} title={collection.name} trailing={space.name} />

{#key collection.id}
  <SeriesView collectionView$key={collection} dateDisplay={space.dateDisplay} />
{/key}
