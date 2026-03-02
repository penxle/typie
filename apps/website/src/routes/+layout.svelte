<script lang="ts">
  import '../app.css';

  import { setupThemeContext } from '@typie/ui/context';
  import { NotificationProvider } from '@typie/ui/notification';
  import { setupMearieContext } from '$lib/graphql';
  import WasmPanicOverlay from './WasmPanicOverlay.svelte';

  let { children } = $props();

  setupMearieContext();
  setupThemeContext();
</script>

{@render children()}

{#if typeof window !== 'undefined' && !window.__webview__}
  <NotificationProvider />
{/if}

<WasmPanicOverlay />
