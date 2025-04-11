<script lang="ts">
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
</script>

<div class={center({ gap: '8px', color: 'gray.700' })}>
  <div class={css({ size: '8px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
  <div class={css({ size: '8px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
  <div class={css({ size: '8px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
</div>
