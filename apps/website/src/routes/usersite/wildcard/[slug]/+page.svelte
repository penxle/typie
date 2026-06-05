<script lang="ts">
  import { hydrateQuery } from '$lib/graphql';
  import DocumentView from './DocumentView.svelte';
  import DocumentViewV2 from './DocumentViewV2.svelte';
  import FolderView from './FolderView.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
</script>

{#key query.data.entityView.id}
  {#if query.data.entityView.node.__typename === 'DocumentView'}
    {#if query.data.entityView.node.state}
      <DocumentViewV2 entityView$key={query.data.entityView} user$key={query.data.me} />
    {:else}
      <DocumentView entityView$key={query.data.entityView} user$key={query.data.me} />
    {/if}
  {:else if query.data.entityView.node.__typename === 'FolderView'}
    <FolderView entityView$key={query.data.entityView} />
  {/if}
{/key}
