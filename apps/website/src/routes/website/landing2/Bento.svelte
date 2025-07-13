<script lang="ts">
  import { css } from '$styled-system/css';
</script>

<div
  class={css({
    padding: '28px',
    borderRadius: '[20px]',
    background: '[linear-gradient(135deg, rgba(255, 255, 255, 0.8) 0%, rgba(255, 255, 255, 0.6) 100%)]',
    backdropFilter: '[blur(20px) saturate(180%)]',
    boxShadow: '[0 20px 40px -10px rgba(0, 0, 0, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.8)]',
    border: '[1px solid]',
    borderColor: '[rgba(255, 255, 255, 0.5)]',
    height: 'full',
    display: 'flex',
    flexDirection: 'column',
  })}
>
  <slot />
</div>
