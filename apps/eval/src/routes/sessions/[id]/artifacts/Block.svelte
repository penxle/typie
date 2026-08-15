<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import type { Snippet } from 'svelte';

  // 그룹 — 그룹 제목과 그 내용(항목 묶음·표·행)의 간격을 한 값으로 묶는다.
  type Props = { children: Snippet };
  const { children }: Props = $props();
</script>

<div class={flex({ direction: 'column', gap: '12px' })}>
  {@render children()}
</div>
