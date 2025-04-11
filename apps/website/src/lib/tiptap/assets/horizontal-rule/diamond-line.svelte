<script lang="ts">
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
</script>

<div class={center({ width: '400px', gap: '10px', color: 'gray.700' })}>
  <div class={css({ flexGrow: '1', height: '1px', backgroundColor: 'current' })}></div>
  <div class={css({ size: '10px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
  <div class={css({ flexGrow: '1', height: '1px', backgroundColor: 'current' })}></div>
</div>
