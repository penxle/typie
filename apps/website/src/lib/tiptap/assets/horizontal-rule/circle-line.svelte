<script lang="ts">
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
</script>

<div class={center({ width: '400px', gap: '10px', color: 'text.subtle' })}>
  <div class={css({ flexGrow: '1', height: '1px', backgroundColor: 'current' })}></div>
  <div class={css({ size: '10px', borderRadius: 'full', backgroundColor: 'current' })}></div>
  <div class={css({ flexGrow: '1', height: '1px', backgroundColor: 'current' })}></div>
</div>
