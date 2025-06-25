<script lang="ts">
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
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
  const changeColor = $derived(change === 0 ? 'gray.500' : change > 0 ? 'green.600' : 'red.600');
</script>

<div
  class={css({
    backgroundColor: 'white',
    border: '1px solid',
    borderColor: 'gray.200',
    borderRadius: '[16px]',
    padding: '28px',
    position: 'relative',
    transition: 'all',
    boxShadow: '[0 1px 3px rgba(0, 0, 0, 0.04)]',
    _hover: {
      borderColor: 'gray.300',
      boxShadow: '[0 4px 12px rgba(0, 0, 0, 0.08)]',
    },
  })}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '24px' })}>
    <div>
      <h3
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'gray.700',
          marginBottom: '4px',
        })}
      >
        {title}
      </h3>
      <p class={css({ fontSize: '13px', color: 'gray.500' })}>{description}</p>
    </div>
    <Sparkline color="#6b7280" {data} height={24} width={80} />
  </div>

  <p
    class={css({
      fontSize: '[36px]',
      fontWeight: 'bold',
      color: 'gray.900',
      lineHeight: '[1]',
      marginBottom: '8px',
    })}
  >
    {value}
  </p>
  <p class={css({ fontSize: '13px', color: 'gray.600', display: 'flex', alignItems: 'center', gap: '4px' })}>
    <span class={css({ color: 'gray.500' })}>이전 기간 대비</span>
    <span class={css({ color: changeColor })}>
      {changeValue}
    </span>
  </p>
</div>
