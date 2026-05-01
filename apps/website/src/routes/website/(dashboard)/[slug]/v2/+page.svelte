<script lang="ts">
  import { hydrateQuery } from '$lib/graphql';
  import DocumentEditorV2 from './DocumentEditorV2.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const document$key = $derived(query.data.entity?.node.__typename === 'Document' ? query.data.entity.node : null);
</script>

{#if document$key}
  <DocumentEditorV2 {document$key} slug={data.slug} />
{/if}
