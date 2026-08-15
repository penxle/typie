<script lang="ts">
  import { css } from '@typie/styled-system/css';

  // 원고의 말 — 세리프(RIDIBatang)는 이 화면에서 인용에만 쓴다. 총평 레터의 인용 문법(「」)과 같은 층위라 같은 괄호를 쓰되
  // 여기는 버튼이 아니라 정적 텍스트다(원고로 가는 이동은 없다). head·tail이 같으면(한 문장) 하나만 보인다.
  type Props = { head: string; tail?: string };
  const { head, tail }: Props = $props();

  const single = $derived(!tail || tail === head);
</script>

<blockquote
  class={css({
    fontFamily: 'RIDIBatang',
    fontSize: '14px',
    lineHeight: '[1.8]',
    color: 'text.default',
    wordBreak: 'keep-all',
    overflowWrap: 'anywhere',
  })}
>
  「{head}」{#if !single}<span class={css({ marginX: '4px', fontFamily: 'ui', color: 'text.faint' })}>⋯</span>
    「{tail}」{/if}
</blockquote>
