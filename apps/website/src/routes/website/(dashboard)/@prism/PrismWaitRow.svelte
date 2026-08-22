<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { expand, rise } from './lib/motion.ts';
  import PrismSpinner from './PrismSpinner.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    label: string;
    text: string | null;
    style?: SystemStyleObject;
  };

  let { label, text, style }: Props = $props();

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
  <PrismSpinner {label} />
  {#if text !== null}
    <span class={shimmerClass} transition:rise>{text}</span>
  {/if}
</div>
