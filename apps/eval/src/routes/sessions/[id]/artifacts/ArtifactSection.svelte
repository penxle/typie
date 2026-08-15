<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  // 아이브로는 이 산출물을 낳은 단계다(과정 화면의 단계 라벨과 같은 말) — 제목 위에 얹혀 9종이 어느 단계의 것인지 위계로 읽힌다.
  type Props = { id: string; stage: string; label: string; children: Snippet };
  const { id, stage, label, children }: Props = $props();
</script>

<section
  {id}
  class={css({
    scrollMarginTop: '16px',
    '&:not(:first-child)': { marginTop: '48px', paddingTop: '36px', borderTopWidth: '1px', borderColor: 'border.default' },
  })}
  data-artifact-section
>
  <p class={css({ fontSize: '11px', fontWeight: 'semibold', letterSpacing: '[0.02em]', color: 'text.faint' })}>{stage}</p>
  <h3 class={css({ marginTop: '4px', fontSize: '18px', fontWeight: 'semibold', letterSpacing: '[-0.01em]', color: 'text.default' })}>
    {label}
  </h3>
  <div class={css({ marginTop: '22px' })}>
    {@render children()}
  </div>
</section>
