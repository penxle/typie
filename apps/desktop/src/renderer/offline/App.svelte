<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { onMount } from 'svelte';

  const retry = () => window.shell.retry?.();

  onMount(() => {
    window.addEventListener('online', retry);
    return () => window.removeEventListener('online', retry);
  });
</script>

<main class={center({ height: '[100vh]' })}>
  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '16px' })}>
    <p class={css({ fontSize: '17px', fontWeight: 'bold' })}>인터넷에 연결되어 있지 않아요</p>
    <p class={css({ fontSize: '14px', color: 'text.subtle' })}>연결이 돌아오면 자동으로 다시 시도해요.</p>
    <button
      class={css({ paddingX: '16px', paddingY: '8px', borderRadius: '6px', backgroundColor: 'surface.muted', fontSize: '14px' })}
      onclick={retry}
      type="button"
    >
      다시 시도
    </button>
  </div>
</main>
