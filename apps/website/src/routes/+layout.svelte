<script lang="ts">
  import '../app.css';

  import { setClient } from '@mearie/svelte';
  import { setupThemeContext } from '@typie/ui/context';
  import { NotificationProvider } from '@typie/ui/notification';
  import { onMount } from 'svelte';
  import { invalidate } from '$app/navigation';
  import { mearieClient } from '$lib/graphql';

  let { children } = $props();

  setClient(mearieClient);
  setupThemeContext();

  onMount(() => {
    const interval = setInterval(() => invalidate('app:bootstrap'), 60_000);
    return () => clearInterval(interval);
  });
</script>

{@render children()}

{#if typeof window !== 'undefined' && !window.__webview__}
  <NotificationProvider />
{/if}
