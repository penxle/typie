<script lang="ts">
  import { getAppContext } from '@typie/ui/context';
  import { onMount } from 'svelte';
  import { prismRuntime } from './runtime.ts';
  import type { PrismHdrMode, PrismRuntimeSnapshot } from '@typie/prism-ui';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLDivElement> & {
    label: string;
    onStateChange?: (snapshot: PrismRuntimeSnapshot) => void;
    reducedMotion?: boolean;
  };

  let { class: className, label, onStateChange, reducedMotion = false, ...rest }: Props = $props();
  const app = getAppContext();
  const hdr: PrismHdrMode = $derived(app.preference.current.prismHdrEnabled ? 'auto' : 'off');
  let host: HTMLDivElement;
  let mounted: ReturnType<typeof prismRuntime.mountSpinner> | null = null;

  onMount(() => {
    mounted = prismRuntime.mountSpinner(host, { hdr, reducedMotion });
    const unsubscribe = mounted.subscribe((snapshot) => onStateChange?.(snapshot));
    return () => {
      unsubscribe();
      mounted?.destroy();
      mounted = null;
    };
  });

  $effect(() => {
    const next = { hdr, reducedMotion };
    mounted?.update(next);
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
