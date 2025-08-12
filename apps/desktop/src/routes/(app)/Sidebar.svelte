<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
</script>

<aside
  class={flex({
    flexDirection: 'column',
    flexShrink: '0',
    width: '200px',
    height: 'full',
    backgroundColor: 'surface.subtle',
  })}
>
  <div style:-webkit-app-region="drag" class={css({ height: '40px' })} data-tauri-drag-region></div>
  <div class={css({ flexGrow: '1' })}></div>
</aside>
