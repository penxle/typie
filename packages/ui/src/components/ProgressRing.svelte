<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    progress: number;
    state?: 'under' | 'achieved' | 'over' | 'excess';
    pie?: number | null;
    pieWarning?: boolean;
    size?: number;
    style?: SystemStyleObject;
  };

  let { progress, state = 'under', pie = null, pieWarning = false, size = 16, style }: Props = $props();

  const R = 13;
  const C = 2 * Math.PI * R;
  const PIE_R = 4.5;
  const PIE_C = 2 * Math.PI * PIE_R;

  const clamped = $derived(Math.min(1, Math.max(0, progress)));
  const ringFraction = $derived(state === 'under' ? clamped : 1);
  const ringDashArray = $derived(`${ringFraction * C} ${C}`);
  const pieDashArray = $derived(`${Math.min(1, Math.max(0, pie ?? 0)) * PIE_C} ${PIE_C}`);

  const strokeByState = {
    under: css.raw({ stroke: 'accent.default' }),
    achieved: css.raw({ stroke: 'success.default' }),
    over: css.raw({ stroke: 'warning.default' }),
    excess: css.raw({ stroke: 'danger.default' }),
  };
</script>

<svg class={css(style)} height={size} viewBox="0 0 32 32" width={size}>
  <circle class={css({ fill: '[none]', stroke: 'surface.inset' })} cx="16" cy="16" r={R} stroke-width="3.5" />
  <circle
    style:stroke-dasharray={ringDashArray}
    class={css({ fill: '[none]', transition: '[stroke 0.3s ease, stroke-dasharray 0.3s ease]' }, strokeByState[state])}
    cx="16"
    cy="16"
    r={R}
    stroke-linecap={ringFraction > 0 ? 'round' : 'butt'}
    stroke-width="3.5"
    transform="rotate(-90 16 16)"
  />
  {#if pie !== null && pie !== undefined}
    <circle
      style:stroke-dasharray={pieDashArray}
      class={css(
        { fill: '[none]', transition: '[stroke 0.3s ease, stroke-dasharray 0.3s ease]' },
        { stroke: pieWarning ? 'danger.default' : 'border.emphasis' },
      )}
      cx="16"
      cy="16"
      r={PIE_R}
      stroke-width="9"
      transform="rotate(-90 16 16)"
    />
  {/if}
</svg>
