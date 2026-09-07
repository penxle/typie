<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import TagView from '../../../TagView.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.tagQuery));
  const space = $derived(query.data.spaceView);
  const tag = $derived(space.tag);
</script>

<Helmet description={space.name} title={tag.name} trailing={space.name} />

{#key tag.name}
  <TagView
    active={tag.name}
    dateDisplay={space.dateDisplay}
    hasMore={tag.publications.hasMore}
    publications={tag.publications.publications}
    tags={space.tags}
  />
{/key}
