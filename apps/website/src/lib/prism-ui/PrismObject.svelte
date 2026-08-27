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
    spinnerPlaybackStartedAt?: number;
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
    spinnerPlaybackStartedAt,
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
      target: 'icon',
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
    const playbackStartedAt = spinnerPlaybackStartedAt;
    const requestOptions = {
      ...(nextTarget === 'spinner' && playbackStartedAt !== undefined && { spinnerPlaybackStartedAt: playbackStartedAt }),
      ...(duration !== undefined && { totalDurationMs: duration }),
    };
    if (Object.keys(requestOptions).length === 0) mounted?.setTarget(nextTarget);
    else mounted?.setTarget(nextTarget, requestOptions);
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
