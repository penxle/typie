<script lang="ts">
  import { hydrateQuery } from '$lib/graphql';
  import PublicationViewV2 from './PublicationViewV2.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
</script>

<svelte:head>
  {#if !query.data.siteView.allowIndexing}
    <meta name="robots" content="noindex, nofollow" />
  {/if}
</svelte:head>

{#key query.data.siteView.publication.id}
  <PublicationViewV2 publicationView$key={query.data.siteView.publication} user$key={query.data.me} />
{/key}
