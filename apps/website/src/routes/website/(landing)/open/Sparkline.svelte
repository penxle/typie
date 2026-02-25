<script lang="ts">
  import * as Plot from '@observablehq/plot';
  import { css } from '@typie/styled-system/css';

  type SparklineProps = {
    data: { date: string; value: number }[];
    width?: number;
    height?: number;
  };

  let { data, width = 100, height = 30 }: SparklineProps = $props();

  let chartContainer: HTMLDivElement;

  $effect(() => {
    if (!chartContainer || !data || data.length === 0) return;

    // eslint-disable-next-line svelte/no-dom-manipulating
    chartContainer.replaceChildren();

    const chartData = data.map((d) => ({
      date: new Date(d.date),
      value: d.value,
    }));

    const strokeWidth = 1.5;
    const chart = Plot.plot({
      width,
      height,
      margin: strokeWidth,
      x: {
        type: 'time',
        axis: null,
      },
      y: {
        axis: null,
        domain: [Math.min(...chartData.map((d) => d.value)), Math.max(...chartData.map((d) => d.value))],
      },
      marks: [
        Plot.lineY(chartData, {
          x: 'date',
          y: 'value',
          stroke: '#525252',
          strokeWidth,
        }),
      ],
      style: {
        background: 'transparent',
      },
    });

    // eslint-disable-next-line svelte/no-dom-manipulating
    chartContainer.append(chart);
  });
</script>

<div
  bind:this={chartContainer}
  class={css({
    width: `[${width}px]`,
    height: `[${height}px]`,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  })}
></div>
