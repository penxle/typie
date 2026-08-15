<script lang="ts">
  import { css } from '@typie/styled-system/css';

  // 리드가 산문일 때(미결 질문처럼 본문이 곧 항목인 경우) — 14px regular.
  type Props = { text: string };
  const { text }: Props = $props();
</script>

<p class={css({ fontSize: '14px', lineHeight: '[1.7]', color: 'text.default', wordBreak: 'keep-all', overflowWrap: 'anywhere' })}>{text}</p>
