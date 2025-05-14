<script lang="ts">
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  onMount(() => {
    window.__webview__ = true;
  });
</script>

{@render children()}
