<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { fade } from 'svelte/transition';
  import { words } from '$lib/text';
  import { revealCount } from './reveal.svelte';

  type Props = { text: string; shown: boolean; prose?: boolean };

  let { text, shown, prose = false }: Props = $props();

  const STEP_MS = 22;

  const list = words(text);

  const reduced = $derived(prefersReducedMotion.current);
  const count = revealCount(() => ({ length: list.length, shown, step: STEP_MS }));

  const textClass = css({
    fontFamily: 'ui',
    fontSize: { base: '15px', lg: '16px' },
    lineHeight: '[1.75]',
    color: 'text.default',
    wordBreak: 'keep-all',
    '&[data-prose="true"]': {
      borderLeftWidth: '2px',
      borderColor: 'border.emphasis',
      paddingLeft: '10px',
      fontFamily: 'prose',
      fontStyle: 'italic',
      lineHeight: '[1.7]',
      color: 'text.muted',
    },
  });
</script>

{#if count.current > 0}
  <p class={textClass} data-prose={prose}>
    {#each list.slice(0, count.current) as word, index (index)}
      <span in:fade={{ duration: reduced ? 0 : 140 }}>{word + ' '}</span>
    {/each}
  </p>
{/if}
