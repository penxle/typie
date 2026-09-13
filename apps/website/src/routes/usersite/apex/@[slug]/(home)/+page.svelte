<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import PublicationListSection from '../PublicationListSection.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const space = $derived(query.data.spaceView);
</script>

<Helmet description={space.description ?? space.name} title={space.name} />

<PublicationListSection
  dateDisplay={space.dateDisplay}
  initialHasMore={space.publications.hasMore}
  publications={space.publications.publications}
/>
