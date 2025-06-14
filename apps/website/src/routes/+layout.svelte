<script lang="ts">
  import '../app.css';

  import { NotificationProvider } from '$lib/notification';

  let { children } = $props();
</script>

{@render children()}

{#if typeof window !== 'undefined' && !window.__webview__}
  <NotificationProvider />
{/if}
