<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
</script>

<div class={center({ gap: '8px', color: 'text.subtle' })}>
  <div class={css({ size: '8px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
  <div class={css({ size: '8px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
  <div class={css({ size: '8px', borderWidth: '1px', borderColor: 'current', transform: 'rotate(45deg)' })}></div>
</div>
