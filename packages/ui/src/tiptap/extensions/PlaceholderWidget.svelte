<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
</script>

<div
  class={flex({
    position: 'relative',
    flexDirection: 'column',
    gap: '4px',
    color: 'text.disabled',
    pointerEvents: 'none',
  })}
>
  <div>내용을 입력하거나 /를 입력해 블록 삽입하기...</div>
</div>
