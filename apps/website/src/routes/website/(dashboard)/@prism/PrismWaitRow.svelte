<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { fade } from 'svelte/transition';
  import { fadeIn } from './lib/motion.ts';
  import PrismSpinner from './PrismSpinner.svelte';

  type Props = {
    label: string;
    text: string | null;
  };

  let { label, text }: Props = $props();

  const rowClass = flex({ alignItems: 'center', gap: '8px', minHeight: '20px', fontSize: '12px', color: 'text.faint' });
  const shimmerClass = css({
    width: '[fit-content]',
    color: '[transparent]',
    backgroundImage: '[linear-gradient(90deg, token(colors.text.faint) 30%, token(colors.text.default) 50%, token(colors.text.faint) 70%)]',
    backgroundSize: '[200% 100%]',
    backgroundClip: 'text',
    animation: '[shimmer 1.8s linear infinite]',
    _motionReduce: { animation: 'none', color: 'text.faint', backgroundImage: 'none' },
  });
</script>

<div class={rowClass} in:fade={fadeIn}>
  <PrismSpinner {label} />
  {#if text !== null}
    <span class={shimmerClass} in:fade={fadeIn}>{text}</span>
  {/if}
</div>
