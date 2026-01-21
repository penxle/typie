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

    if (type === 'daily') {
      current = data.at(-1)?.value ?? 0;
      previous = data.at(-2)?.value ?? 0;
    } else {
      current = data.at(-1)?.value ?? 0;
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
  const changeColor = $derived(change === 0 ? 'dark.gray.500' : change > 0 ? '[#22c55e]' : '[#ef4444]');
</script>

<div
  class={css({
    backgroundColor: 'dark.gray.900',
    borderWidth: '1px',
    borderColor: 'dark.gray.800',
    padding: { sm: '24px', lg: '28px' },
    position: 'relative',
    transition: '[all 0.2s ease-out]',
    _hover: {
      borderColor: 'dark.gray.700',
      backgroundColor: 'dark.gray.900/80',
    },
  })}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '20px' })}>
    <div>
      <h3
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'dark.gray.200',
          marginBottom: '4px',
        })}
      >
        {title}
      </h3>
      <p class={css({ fontSize: '13px', color: 'dark.gray.500' })}>{description}</p>
    </div>
    <Sparkline {data} height={28} width={80} />
  </div>

  <p
    class={css({
      fontSize: { sm: '[32px]', lg: '[36px]' },
      fontWeight: 'medium',
      color: 'dark.gray.100',
      lineHeight: '[1]',
      marginBottom: '10px',
      fontFamily: 'Paperlogy',
    })}
  >
    {value}
  </p>

  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <span class={css({ fontSize: '12px', fontFamily: 'mono', color: 'dark.gray.400', textTransform: 'uppercase' })}>
      {type === 'daily' ? '전일 대비' : '30일 전 대비'}
    </span>
    <span class={css({ fontSize: '13px', fontFamily: 'mono', fontWeight: 'medium', color: changeColor })}>{changeValue}</span>
  </div>
</div>
