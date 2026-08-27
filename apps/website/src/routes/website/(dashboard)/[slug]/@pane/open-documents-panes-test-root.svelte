<script lang="ts">
  import { setupOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { setupPaneGroup } from './context.svelte';
  import Panes from './Panes.svelte';
  import type { OpenDocumentRegistry } from '$lib/prism/open-documents.svelte';
  import type { Pane } from './types';

  type Props = {
    onRegistry: (registry: OpenDocumentRegistry) => void;
  };

  let { onRegistry }: Props = $props();

  const openDocuments = setupOpenDocuments();
  const paneGroup = setupPaneGroup('open-documents-panes-test', {
    userId: 'open-documents-panes-test',
    navigate: () => {
      // No navigation in this test host.
    },
    onSiteChange: () => {
      // The test stays on one site.
    },
  });

  const pane = (id: string, slug: string): Pane => ({ id, type: 'pane', kind: 'entity', slug });

  paneGroup.state.current.root = pane('p1', 'D1');
  paneGroup.state.current.focusedPaneId = 'p1';
  onRegistry(openDocuments);

  const root = $derived(paneGroup.state.current.root);

  export function replacePane() {
    paneGroup.state.current.root = pane('p2', 'D2');
    paneGroup.state.current.focusedPaneId = 'p2';
  }

  export function refreshPane() {
    paneGroup.state.current.root = pane('p1', 'D1');
  }
</script>

{#if root}
  <Panes {root} />
{/if}
