<script lang="ts">
  // cspell:ignore onrestart

  import { onMount } from 'svelte';
  import TabStrip from './TabStrip.svelte';
  import UpdateButton from './UpdateButton.svelte';

  let tabs = $state<TabState[]>([]);
  let activeId = $state<string | null>(null);
  let updateReady = $state(false);

  onMount(() => {
    const offTabs = window.shell.onTabsState?.((state) => {
      tabs = state.tabs;
      activeId = state.activeId;
    });
    const offTheme = window.shell.onTheme?.((theme) => {
      document.documentElement.dataset.theme = theme.theme;
      document.documentElement.dataset.variantLight = theme.variantLight;
      document.documentElement.dataset.variantDark = theme.variantDark;
    });
    const offUpdate = window.shell.onUpdateReady?.(() => {
      updateReady = true;
    });
    return () => {
      offTabs?.();
      offTheme?.();
      offUpdate?.();
    };
  });
</script>

<TabStrip {activeId} {tabs}>
  {#if updateReady}
    <UpdateButton onrestart={() => window.shell.restartToUpdate?.()} />
  {/if}
</TabStrip>
