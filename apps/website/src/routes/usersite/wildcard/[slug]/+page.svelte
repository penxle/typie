<script lang="ts">
  import { hydrateQuery } from '$lib/graphql';
  import DocumentView from './DocumentView.svelte';
  import FolderView from './FolderView.svelte';
  import PostView from './PostView.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
</script>

{#key query.data.entityView.id}
  {#if query.data.entityView.node.__typename === 'PostView'}
    <PostView entityView$key={query.data.entityView} user$key={query.data.me} />
  {:else if query.data.entityView.node.__typename === 'DocumentView'}
    <DocumentView entityView$key={query.data.entityView} user$key={query.data.me} />
  {:else}
    <FolderView entityView$key={query.data.entityView} />
  {/if}
{/key}
