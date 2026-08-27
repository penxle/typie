<script lang="ts">
  import { css } from '@typie/styled-system/css';
</script>

<span
  class={css({
    position: 'absolute',
    top: '-1px',
    right: '-1px',
    size: '6px',
    borderRadius: 'full',
    backgroundColor: 'accent.info.default',
  })}
  aria-hidden="true"
></span>
<span class={css({ srOnly: true })}>새 상태 변화 있음</span>
