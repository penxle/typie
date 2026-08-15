<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';

  // 판정점 1~4 — 총평 레터의 눈금(16×3 막대)과 같은 정보 층위라 같은 문법을 쓴다.
  type Props = { point: number };
  const { point }: Props = $props();

  const POINTS = [1, 2, 3, 4];
</script>

<span class={flex({ align: 'center', gap: '8px' })}>
  <span class={css({ fontFamily: 'mono', fontSize: '12px', letterSpacing: '0', color: 'text.default' })}>{point}</span>
  <span class={flex({ align: 'center', gap: '3px' })} aria-hidden="true">
    {#each POINTS as step (step)}
      <span
        class={css({
          width: '16px',
          height: '3px',
          borderRadius: 'full',
          backgroundColor: step <= point ? 'text.default' : 'border.default',
        })}
      ></span>
    {/each}
  </span>
</span>
