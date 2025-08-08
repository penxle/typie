<script lang="ts">
  import '../app.css';

  import { setupThemeContext } from '@typie/ui/context';
  import { NotificationProvider } from '@typie/ui/notification';

  let { children } = $props();

  setupThemeContext();
</script>

{@render children()}

{#if typeof window !== 'undefined' && !window.__webview__}
  <NotificationProvider />
{/if}
