<script lang="ts">
  import { onMount } from 'svelte';
  import { prismRuntime } from './runtime.ts';
  import type { PrismRuntimeSnapshot } from '@typie/prism-ui';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLDivElement> & {
    label: string;
    onStateChange?: (snapshot: PrismRuntimeSnapshot) => void;
    reducedMotion?: boolean;
  };

  let { class: className, label, onStateChange, reducedMotion = false, ...rest }: Props = $props();
  let host: HTMLDivElement;
  let mounted: ReturnType<typeof prismRuntime.mountSpinner> | null = null;

  onMount(() => {
    mounted = prismRuntime.mountSpinner(host, { reducedMotion });
    const unsubscribe = mounted.subscribe((snapshot) => onStateChange?.(snapshot));
    return () => {
      unsubscribe();
      mounted?.destroy();
      mounted = null;
    };
  });

  $effect(() => {
    mounted?.update({ reducedMotion });
  });
</script>

<div {...rest} bind:this={host} class={['prism-spinner-host', className]} aria-label={label} role="status"></div>

<style>
  .prism-spinner-host {
    display: grid;
    width: 18px;
    height: 18px;
    flex: none;
    place-items: center;
  }
</style>
