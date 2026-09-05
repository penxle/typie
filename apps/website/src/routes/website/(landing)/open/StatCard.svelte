<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import Sparkline from './Sparkline.svelte';

  type Props = {
    title: string;
    description: string;
    value: string;
    data: { date: string; value: number }[];
    type: 'daily' | 'accumulative';
  };

  let { title, description, value, data, type }: Props = $props();

  function calculateChange(data: { date: string; value: number }[], type: 'daily' | 'accumulative'): number {
    if (data.length < 2) return 0;

    let current: number;
    let previous: number;

    current = data.at(-1)?.value ?? 0;
    if (type === 'daily') {
      previous = data.at(-2)?.value ?? 0;
    } else {
      previous = data.at(0)?.value ?? 0;
    }

    if (previous === 0) return 0;
    return Math.round(((current - previous) / previous) * 100);
  }

  function formatChange(change: number): string {
    return change > 0 ? `+${change}%` : `${change}%`;
  }

  const change = $derived(calculateChange(data, type));
  const changeValue = $derived(formatChange(change));
  const changeColor = $derived(change === 0 ? 'text.hint' : change > 0 ? 'success.default' : 'danger.default');
</script>

<div
  class={css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    padding: { sm: '24px', lg: '28px' },
    position: 'relative',
    transition: '[all 0.2s ease-out]',
    _hover: {
      borderColor: 'border.emphasis',
      backgroundColor: 'surface.hover',
    },
  })}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '20px' })}>
    <div>
      <h3
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          marginBottom: '4px',
        })}
      >
        {title}
      </h3>
      <p class={css({ fontSize: '13px', color: 'text.muted' })}>{description}</p>
    </div>
    <Sparkline {data} height={28} width={80} />
  </div>

  <p
    class={css({
      fontSize: { sm: '[32px]', lg: '[36px]' },
      fontWeight: 'medium',
      color: 'text.default',
      lineHeight: '[1]',
      marginBottom: '10px',
      fontFamily: 'Paperlogy',
    })}
  >
    {value}
  </p>

  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <span class={css({ fontSize: '12px', fontFamily: 'mono', color: 'text.muted', textTransform: 'uppercase' })}>
      {type === 'daily' ? '전일 대비' : '30일 전 대비'}
    </span>
    <span class={css({ fontSize: '13px', fontFamily: 'mono', fontWeight: 'medium', color: changeColor })}>{changeValue}</span>
  </div>
</div>
