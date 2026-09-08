<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { DOWNLOADS } from '$lib/platform';
  import { detectedPlatform } from '$lib/platform.svelte';
  import AvailableOn from './AvailableOn.svelte';
  import DownloadButton from './DownloadButton.svelte';
  import StartButton from './StartButton.svelte';

  type Props = { badge?: string; marginTop?: string; justify?: 'flex-start' | 'center'; platforms?: boolean };

  let { badge, marginTop = '36px', justify = 'flex-start', platforms = true }: Props = $props();

  const detected = detectedPlatform();

  const primary = $derived(DOWNLOADS.find((platform) => platform.id === detected.current) ?? null);

  const stackClass = flex({ direction: 'column', gap: '24px' });
  const buttonsClass = flex({ align: 'center', gap: '12px', wrap: 'wrap' });
</script>

<div style:align-items={justify} style:margin-top={marginTop} class={stackClass}>
  <div style:justify-content={justify} class={buttonsClass}>
    <StartButton {badge} />
    {#if primary}
      <DownloadButton platform={primary} />
    {/if}
  </div>
  {#if platforms}
    <div style:justify-content={justify} class={flex({ width: 'full' })}>
      <AvailableOn />
    </div>
  {/if}
</div>
