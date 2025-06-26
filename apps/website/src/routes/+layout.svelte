<script lang="ts">
  import '../app.css';

  import { setupThemeContext } from '$lib/context';
  import { NotificationProvider } from '$lib/notification';

  let { children } = $props();

  setupThemeContext();
</script>

{@render children()}

{#if typeof window !== 'undefined' && !window.__webview__}
  <NotificationProvider />
{/if}
