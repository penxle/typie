<script lang="ts">
  import { css } from '$styled-system/css';
</script>

<div
  class={css({
    padding: '32px',
    backgroundColor: 'white',
    border: '4px solid',
    borderColor: 'gray.900',
    boxShadow: '[8px 8px 0 0 #000]',
    height: 'full',
    display: 'flex',
    flexDirection: 'column',
    transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
    _hover: {
      transform: 'translate(-4px, -4px)',
      boxShadow: '[12px 12px 0 0 #000]',
    },
  })}
>
  <slot />
</div>
