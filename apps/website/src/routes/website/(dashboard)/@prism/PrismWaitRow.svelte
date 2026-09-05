<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import PrismSpinner from '$lib/prism-ui/PrismSpinner.svelte';
  import { expand, rise } from './lib/motion.ts';
  import type { PrismRuntimeSnapshot } from '@typie/prism-ui';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    label: string;
    onSpinnerPlaybackChange?: (startedAt: number | null) => void;
    spinnerAnchor?: HTMLElement;
    text: string | null;
    style?: SystemStyleObject;
  };

  let { label, onSpinnerPlaybackChange, spinnerAnchor = $bindable(), text, style }: Props = $props();

  const handleSpinnerStateChange = (snapshot: PrismRuntimeSnapshot) => {
    if (snapshot.playbackStartedAt !== undefined) onSpinnerPlaybackChange?.(snapshot.playbackStartedAt);
    else if (snapshot.readiness !== 'loading') onSpinnerPlaybackChange?.(null);
  };

  const rowStyle = flex.raw({ alignItems: 'center', gap: '8px', minHeight: '20px', fontSize: '12px', color: 'text.muted' });
  const shimmerClass = css({
    width: '[fit-content]',
    color: 'transparent',
    backgroundImage: '[linear-gradient(90deg, token(colors.text.muted) 30%, token(colors.text.default) 50%, token(colors.text.muted) 70%)]',
    backgroundSize: '[200% 100%]',
    backgroundClip: 'text',
    animation: '[shimmer 1.8s linear infinite]',
    _motionReduce: { animation: 'none', color: 'text.muted', backgroundImage: 'none' },
  });
</script>

<div class={css(rowStyle, style)} in:expand>
  <span
    bind:this={spinnerAnchor}
    class={css({ display: 'grid', flexShrink: '0', size: '18px', placeItems: 'center' })}
    data-prism-spinner-anchor
  >
    <PrismSpinner {label} onStateChange={handleSpinnerStateChange} />
  </span>
  {#if text !== null}
    <span class={shimmerClass} transition:rise>{text}</span>
  {/if}
</div>
