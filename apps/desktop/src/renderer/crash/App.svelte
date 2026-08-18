<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';

  const retry = () => window.shell.retry?.();
</script>

<main class={center({ height: '[100vh]' })}>
  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '16px' })}>
    <p class={css({ fontSize: '17px', fontWeight: 'bold' })}>문제가 생겼어요</p>
    <p class={css({ fontSize: '14px', color: 'text.subtle' })}>탭을 다시 불러올게요.</p>
    <button
      class={css({ paddingX: '16px', paddingY: '8px', borderRadius: '6px', backgroundColor: 'surface.muted', fontSize: '14px' })}
      onclick={retry}
      type="button"
    >
      다시 불러오기
    </button>
  </div>
</main>
