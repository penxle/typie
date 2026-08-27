<script lang="ts">
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import type { Pane } from './types';

  type Props = {
    pane: Extract<Pane, { kind: 'entity' }>;
  };

  let { pane }: Props = $props();

  const openDocuments = getOpenDocuments();

  openDocuments.expectPane(pane.id);
  $effect(() => {
    openDocuments.upsert(pane.id, {
      kind: 'document',
      documentId: pane.slug,
      entityId: pane.slug,
      title: pane.slug,
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });
  });
</script>

<div>{pane.slug}</div>
