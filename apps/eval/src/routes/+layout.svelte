<script lang="ts">
  import '../app.css';

  import type { Snippet } from 'svelte';

  type Props = { children: Snippet };
  const { children }: Props = $props();
</script>

{@render children()}
