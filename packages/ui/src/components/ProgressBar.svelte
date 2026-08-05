<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    progress: number;
    state?: 'under' | 'achieved' | 'over' | 'excess';
    style?: SystemStyleObject;
  };

  let { progress, state = 'under', style }: Props = $props();

  const clamped = $derived(Math.min(1, Math.max(0, progress)));
  const fillFraction = $derived(state === 'under' ? clamped : 1);

  const fillByState = {
    under: css.raw({ backgroundColor: { base: 'gray.500', _dark: 'dark.gray.400' } }),
    achieved: css.raw({ backgroundColor: 'accent.success.default' }),
    over: css.raw({ backgroundColor: 'accent.warning.default' }),
    excess: css.raw({ backgroundColor: 'accent.danger.default' }),
  };
</script>

<div
  class={css(
    { height: '6px', borderRadius: 'full', backgroundColor: { base: 'gray.200', _dark: 'dark.gray.800' }, overflow: 'hidden' },
    style,
  )}
>
  <div
    style:width="{fillFraction * 100}%"
    class={css({ height: 'full', borderRadius: 'full', transition: '[width 0.3s ease, background-color 0.3s ease]' }, fillByState[state])}
  ></div>
</div>
