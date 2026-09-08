<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Point } from './open';

  type Props = { data: Point[]; width?: number; height?: number; tone?: 'accent' | 'muted' };

  let { data, width = 80, height = 28, tone = 'accent' }: Props = $props();

  const STROKE = 1.5;

  const points = $derived.by(() => {
    if (data.length < 2) return '';
    const values = data.map((point) => point.value);
    const min = Math.min(...values);
    const max = Math.max(...values);
    const span = max - min || 1;
    const innerWidth = width - STROKE * 2;
    const innerHeight = height - STROKE * 2;
    return values
      .map((value, index) => {
        const x = STROKE + (index / (values.length - 1)) * innerWidth;
        const y = STROKE + innerHeight - ((value - min) / span) * innerHeight;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  });

  const svgClass = css({
    display: 'block',
    '&[data-tone="accent"]': { color: 'accent.default' },
    '&[data-tone="muted"]': { color: 'text.muted' },
  });
</script>

<svg
  style:width="{width}px"
  style:height="{height}px"
  class={svgClass}
  aria-hidden="true"
  data-tone={tone}
  fill="none"
  viewBox="0 0 {width} {height}"
>
  <polyline
    {points}
    stroke="currentColor"
    stroke-linecap="round"
    stroke-linejoin="round"
    stroke-width={STROKE}
    vector-effect="non-scaling-stroke"
  />
</svg>
