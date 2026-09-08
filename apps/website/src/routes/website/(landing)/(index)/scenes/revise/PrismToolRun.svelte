<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { fade } from 'svelte/transition';
  import CheckIcon from '~icons/lucide/check';
  import { revealCount } from './reveal.svelte';

  type Props = { labels: readonly string[]; shown: boolean };

  let { labels, shown }: Props = $props();

  const STEP_MS = 220;

  const reduced = $derived(prefersReducedMotion.current);
  const count = revealCount(() => ({ length: labels.length, shown, step: STEP_MS }));

  const rowClass = flex({
    align: 'center',
    gap: '6px',
    fontSize: '12px',
    lineHeight: '[1.75]',
    color: 'text.hint',
    whiteSpace: 'nowrap',
  });
</script>

{#if count.current > 0}
  <div>
    {#each labels.slice(0, count.current) as label (label)}
      <div class={rowClass} in:fade={{ duration: reduced ? 0 : 160 }}>
        <Icon icon={CheckIcon} size={10} />
        {label}
      </div>
    {/each}
  </div>
{/if}
