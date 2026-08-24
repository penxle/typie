<script lang="ts">
  import { onMount } from 'svelte';
  import { prismRuntime } from './runtime.ts';
  import type { PrismRuntimeSnapshot, PrismTarget } from '@typie/prism-ui';
  import type { HTMLAttributes } from 'svelte/elements';

  type EdgeColor = string | readonly [number, number, number, number];
  type Props = Omit<HTMLAttributes<HTMLDivElement>, 'target'> & {
    edgeColor?: EdgeColor;
    interactive?: boolean;
    onStateChange?: (snapshot: PrismRuntimeSnapshot) => void;
    preload?: boolean;
    reducedMotion?: boolean;
    target: PrismTarget;
    targetDurationMs?: number;
  };

  let {
    class: className,
    edgeColor,
    interactive = false,
    onStateChange,
    preload = false,
    reducedMotion = false,
    target,
    targetDurationMs,
    ...rest
  }: Props = $props();
  let host: HTMLDivElement;
  let mounted: ReturnType<typeof prismRuntime.mountObject> | null = null;

  onMount(() => {
    mounted = prismRuntime.mountObject(host, {
      edgeColor,
      ...(interactive && { interactive: true }),
      preload,
      reducedMotion,
      target,
    });
    const unsubscribe = mounted.subscribe((snapshot) => onStateChange?.(snapshot));
    if (preload) void mounted.whenReady();
    return () => {
      unsubscribe();
      mounted?.destroy();
      mounted = null;
    };
  });

  $effect(() => {
    const nextTarget = target;
    const duration = targetDurationMs;
    if (duration === undefined) mounted?.setTarget(nextTarget);
    else mounted?.setTarget(nextTarget, { totalDurationMs: duration });
  });

  $effect(() => {
    const next = { edgeColor, reducedMotion };
    mounted?.update(next);
  });

  $effect(() => {
    mounted?.update({ interactive });
  });
</script>

<div {...rest} bind:this={host} class={['prism-object-host', { interactive }, className]}></div>

<style>
  .prism-object-host {
    display: grid;
    width: 160px;
    height: 160px;
    place-items: center;
  }

  .prism-object-host.interactive {
    width: 320px;
    height: 320px;
    pointer-events: auto;
  }
</style>
