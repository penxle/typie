<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { Snippet } from 'svelte';

  // 항목 한 건 — 모든 항목이 같은 세 줄 문법이다: 키커(자기 id, 연한 mono 꼬리표) → 리드(제목 또는 원고 인용) → 키/값 행.
  // 박스 없음, 형제 사이는 헤어라인. id는 교차 참조의 착지 자리 — 참조 칩이 이 id로 스크롤하고 data-flash로 잠깐
  // 강조한다(jump.ts). 강조는 본문 바깥으로 10px 번지는 그림자라 글자에 달라붙지 않는다.
  type Props = { id?: string; kicker?: Snippet; children: Snippet };
  const { id, kicker, children }: Props = $props();
</script>

<div
  {id}
  class={css({
    paddingY: '16px',
    borderRadius: '4px',
    transition: 'colors',
    transitionDuration: '[250ms]',
    '&:not(:first-child)': { borderTopWidth: '1px', borderColor: 'border.subtle' },
    _first: { paddingTop: '0' },
    _last: { paddingBottom: '0' },
    '&[data-flash]': { backgroundColor: 'accent.brand.subtle', boxShadow: '[0 0 0 10px {colors.accent.brand.subtle}]' },
  })}
>
  {#if kicker}
    <div class={flex({ align: 'center', wrap: 'wrap', gap: '6px', marginBottom: '4px' })}>
      {@render kicker()}
    </div>
  {/if}
  <div class={flex({ direction: 'column', gap: '10px' })}>
    {@render children()}
  </div>
</div>
