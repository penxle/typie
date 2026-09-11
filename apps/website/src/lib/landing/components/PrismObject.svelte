<script lang="ts">
  import { resolvePrismCanvasSize } from '@typie/prism-ui';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { onMount } from 'svelte';
  import type { MountedPrismObject, PrismTarget } from '@typie/prism-ui';

  type Props = { prismSize?: number; target: PrismTarget };

  let { prismSize, target }: Props = $props();

  const NEAR_VIEWPORT = '100%';

  const canvasSize = resolvePrismCanvasSize(prismSize);

  const reduced = $derived(prefersReducedMotion.current);

  let host: HTMLDivElement;
  let mounted = $state<MountedPrismObject | null>(null);

  onMount(() => {
    let disposed = false;

    const observer = new IntersectionObserver(
      async ([entry]) => {
        if (!entry?.isIntersecting) return;
        observer.disconnect();
        const { prismRuntime } = await import('$lib/prism-ui/runtime');
        if (disposed) return;
        mounted = prismRuntime.mountObject(host, { hdr: 'off', prismSize, reducedMotion: reduced, target });
      },
      { rootMargin: NEAR_VIEWPORT },
    );
    observer.observe(host);

    return () => {
      disposed = true;
      observer.disconnect();
      mounted?.destroy();
      mounted = null;
    };
  });

  $effect(() => {
    mounted?.update({ reducedMotion: reduced });
  });
</script>

<div bind:this={host} style:width="{canvasSize}px" style:height="{canvasSize}px" class="prism-object-host"></div>

<style>
  .prism-object-host {
    display: grid;
    place-items: center;
  }
</style>
