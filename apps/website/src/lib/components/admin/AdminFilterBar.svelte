<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();
</script>

<div class={flex({ alignItems: 'center', gap: '8px', flexWrap: 'wrap' })}>
  {@render children()}
</div>
