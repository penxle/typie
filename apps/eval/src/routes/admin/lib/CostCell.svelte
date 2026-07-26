<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { formatKrw, formatUsd } from '$lib/domain/pricing.ts';
  import type { Cost } from '$lib/domain/pricing.ts';

  type Props = { cost: Cost; tokens: number };
  const { cost, tokens }: Props = $props();
</script>

{#if cost.kind === 'exact'}
  <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{formatKrw(cost.krw)}</span>
  <span class={css({ marginLeft: '4px', fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
    {formatUsd(cost.usd)}
  </span>
{:else if tokens === 0}
  <span class={css({ color: 'text.faint' })}>—</span>
{:else if cost.kind === 'mixed'}
  <!-- 단계마다 모델이 다르면 토큰이 한 칸에 뭉쳐 있어 어느 단가로도 몇 배씩 틀린다. -->
  <span class={css({ color: 'text.faint' })} title={cost.models.join(', ')}>혼합 모델</span>
{:else}
  <span class={css({ color: 'text.faint' })} title={cost.model ?? ''}>단가 미설정</span>
{/if}
