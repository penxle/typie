<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import PrismSpinner from '$lib/prism-ui/PrismSpinner.svelte';
  import { expand, rise } from './lib/motion.ts';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    label: string;
    spinnerAnchor?: HTMLElement;
    spinnerOwner?: 'panel' | 'row';
    text: string | null;
    style?: SystemStyleObject;
  };

  let { label, spinnerAnchor = $bindable(), spinnerOwner = 'row', text, style }: Props = $props();

  const rowStyle = flex.raw({ alignItems: 'center', gap: '8px', minHeight: '20px', fontSize: '12px', color: 'text.faint' });
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

<div class={css(rowStyle, style)} in:expand>
  <span
    bind:this={spinnerAnchor}
    class={css({ display: 'grid', flexShrink: '0', size: '18px', placeItems: 'center' })}
    data-prism-spinner-anchor
  >
    {#if spinnerOwner === 'row'}
      <PrismSpinner {label} />
    {/if}
  </span>
  {#if text !== null}
    <span class={shimmerClass} transition:rise>{text}</span>
  {/if}
</div>
