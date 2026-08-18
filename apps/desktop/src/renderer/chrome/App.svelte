<script lang="ts">
  // cspell:ignore ondismiss onrestart

  import { onMount } from 'svelte';
  import TabStrip from './TabStrip.svelte';
  import UpdateBanner from './UpdateBanner.svelte';

  let tabs = $state<TabState[]>([]);
  let activeId = $state<string | null>(null);
  let updateVersion = $state<string | null>(null);

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
    const offUpdate = window.shell.onUpdateReady?.((version) => {
      updateVersion = version;
    });
    return () => {
      offTabs?.();
      offTheme?.();
      offUpdate?.();
    };
  });
</script>

<TabStrip {activeId} {tabs}>
  {#if updateVersion}
    <UpdateBanner
      ondismiss={() => {
        updateVersion = null;
        window.shell.dismissUpdate?.();
      }}
      onrestart={() => window.shell.restartToUpdate?.()}
      version={updateVersion}
    />
  {/if}
</TabStrip>
