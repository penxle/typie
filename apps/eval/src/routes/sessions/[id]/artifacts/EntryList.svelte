<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  // 항목 묶음 — 박스 대신 헤어라인으로 항목을 가른다(Entry가 첫째가 아닐 때 윗선을 긋는다).
  type Props = { children: Snippet };
  const { children }: Props = $props();
</script>

<div class={css({ display: 'flex', flexDirection: 'column' })}>
  {@render children()}
</div>
