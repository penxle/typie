<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { formatKrw, formatUsd } from '$lib/domain/pricing.ts';
  import type { Cost, CostTotal } from '$lib/domain/pricing.ts';

  // 단계마다 모델이 다르면 실행 단위로는 금액이 안 나온다. 단계별 합이 온전하면 그걸 쓴다 —
  // 목록과 상세가 서로 다른 금액을 보이지 않도록 판정을 여기 한곳에 둔다.
  type Props = { cost: Cost; tokens: number; total?: CostTotal | null };
  const { cost, tokens, total = null }: Props = $props();
</script>

{#if cost.kind !== 'exact' && total?.complete && tokens > 0}
  <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{formatKrw(total.krw)}</span>
  <span class={css({ marginLeft: '4px', fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
    {formatUsd(total.usd)}
  </span>
{:else if cost.kind === 'exact'}
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
