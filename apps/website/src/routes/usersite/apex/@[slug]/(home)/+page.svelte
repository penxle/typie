<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoverySectionHead from '../../(site)/DiscoverySectionHead.svelte';
  import PublicationListSection from '../PublicationListSection.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const space = $derived(query.data.spaceView);
</script>

<Helmet description={space.description ?? space.name} title={space.name} />

<DiscoverySectionHead count={space.publicationCount} title="글" />

<PublicationListSection initialHasMore={space.publications.hasMore} publications={space.publications.publications} />
