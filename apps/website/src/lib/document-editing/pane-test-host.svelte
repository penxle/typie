<script lang="ts">
  import { onMount } from 'svelte';
  import { setupPaneGroup } from '../../routes/website/(dashboard)/[slug]/@pane/context.svelte';
  import type { PaneGroup } from '../../routes/website/(dashboard)/[slug]/@pane/types';

  const { siteId, onReady }: { siteId: string; onReady: (group: PaneGroup) => void } = $props();
  const group = setupPaneGroup(siteId, { userId: siteId, navigate: () => true, onSiteChange: () => true });
  group.handleNavigation('document');
  onMount(() => onReady(group));
</script>
