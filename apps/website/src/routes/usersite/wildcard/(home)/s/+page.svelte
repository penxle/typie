<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import PublicationListSection from '../../PublicationListSection.svelte';
  import SeriesCards from '../../SeriesCards.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const space = $derived(query.data.spaceView);
</script>

<Helmet description={space.description ?? space.name} title="시리즈" trailing={space.name} />

<div class={css({ display: { base: 'block', lg: 'none' } })}>
  <SeriesCards collections={space.collections} />
</div>

<div class={css({ display: { base: 'none', lg: 'block' } })}>
  <PublicationListSection
    dateDisplay={space.dateDisplay}
    initialHasMore={space.publications.hasMore}
    publications={space.publications.publications}
  />
</div>
