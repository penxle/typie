<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
</script>

<div
  class={css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    padding: { sm: '24px', lg: '28px' },
    position: 'relative',
  })}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '20px' })}>
    <div>
      <div
        class={css({
          width: '[100px]',
          height: '14px',
          backgroundColor: 'skeleton.base',
          borderRadius: '2px',
          marginBottom: '8px',
          animation: 'pulse 1.5s ease-in-out infinite',
        })}
      ></div>
      <div
        class={css({
          width: '[160px]',
          height: '13px',
          backgroundColor: 'skeleton.base/50',
          borderRadius: '2px',
          animation: 'pulse 1.5s ease-in-out infinite',
        })}
      ></div>
    </div>
    <div
      class={css({
        width: '[80px]',
        height: '28px',
        backgroundColor: 'skeleton.base',
        borderRadius: '2px',
        animation: 'pulse 1.5s ease-in-out infinite',
      })}
    ></div>
  </div>

  <div
    class={css({
      width: '[140px]',
      height: { sm: '[32px]', lg: '[36px]' },
      backgroundColor: 'skeleton.base',
      borderRadius: '4px',
      marginBottom: '10px',
      animation: 'pulse 1.5s ease-in-out infinite',
    })}
  ></div>

  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <div
      class={css({
        width: '[60px]',
        height: '12px',
        backgroundColor: 'skeleton.base/50',
        borderRadius: '2px',
        animation: 'pulse 1.5s ease-in-out infinite',
      })}
    ></div>
    <div
      class={css({
        width: '[40px]',
        height: '13px',
        backgroundColor: 'skeleton.base',
        borderRadius: '2px',
        animation: 'pulse 1.5s ease-in-out infinite',
      })}
    ></div>
  </div>
</div>
