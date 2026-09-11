<script lang="ts">
  import { prefersReducedMotion } from '@typie/ui/state';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import type { Snippet } from 'svelte';

  type Props = { shown: boolean; delay?: number; children: Snippet };

  let { shown, delay = 200, children }: Props = $props();

  const reduced = $derived(prefersReducedMotion.current);
</script>

<div style:visibility={shown ? 'visible' : 'hidden'} aria-hidden={!shown}>
  {#key shown}
    <div in:fly={{ y: 16, duration: reduced ? 0 : 500, easing: cubicOut, delay: reduced ? 0 : delay }}>
      {@render children()}
    </div>
  {/key}
</div>
