<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';

  // 본문 문단 렌더 — 줄바꿈(문단 경계)을 화면 간격으로 세운다. pre-line은 줄만 바꿔 문단이 붙어 보였다
  // (오너 관측 2026-08-17). understanding이 쓰던 줄바꿈 split 렌더를 본문류 전체의 공용 문법으로 넓힌 조각.
  // 전달받은 class는 래퍼에 얹는다 — 글자 스타일은 상속으로 문단에 닿고, 바깥 여백(marginTop)은 한 번만 선다.
  type Props = { text: string; class?: string };
  const { text, class: className }: Props = $props();
  const paragraphs = $derived(
    text
      .split('\n')
      .map((paragraph) => paragraph.trim())
      .filter((paragraph) => paragraph.length > 0),
  );
</script>

<div class={cx(css({ display: 'flex', flexDirection: 'column', gap: '8px' }), className)}>
  {#each paragraphs as paragraph, index (index)}
    <p>{paragraph}</p>
  {/each}
</div>
