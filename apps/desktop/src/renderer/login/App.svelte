<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { onMount } from 'svelte';
  import GlobeIcon from '~icons/lucide/globe';

  let error = $state<string | null>(null);
  let waiting = $state(false);

  onMount(() =>
    window.shell.onAuthError?.((message) => {
      error = message;
      waiting = false;
    }),
  );

  const login = () => {
    error = null;
    waiting = true;
    window.shell.login?.();
  };
</script>

<div style:-webkit-app-region="drag" class={css({ position: 'fixed', top: '0', left: '0', right: '0', height: '[40px]' })}></div>

<main class={center({ height: '[100vh]', backgroundColor: 'surface.default' })}>
  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '24px' })}>
    <div class={css({ fontSize: '24px', fontWeight: 'extrabold' })}>타이피</div>
    <p class={css({ fontSize: '15px', textAlign: 'center', lineHeight: '[1.6]', color: 'text.subtle' })}>
      작성, 정리, 공유까지.
      <br />
      <span class={css({ fontWeight: 'bold', color: 'text.default' })}>글쓰기의 모든 과정을 타이피 하나로 해결해요.</span>
    </p>
    <button
      class={css({
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        paddingX: '20px',
        paddingY: '12px',
        borderRadius: '8px',
        backgroundColor: 'accent.brand.default',
        color: 'text.bright',
        fontSize: '15px',
        fontWeight: 'semibold',
        _disabled: { opacity: '[0.6]', cursor: 'default' },
      })}
      disabled={waiting}
      onclick={login}
      type="button"
    >
      <GlobeIcon />
      {waiting ? '브라우저에서 로그인을 마쳐주세요' : '브라우저로 로그인'}
    </button>
    {#if waiting}
      <button class={css({ fontSize: '13px', color: 'text.faint', textDecoration: 'underline' })} onclick={login} type="button">
        브라우저가 열리지 않았나요? 다시 시도
      </button>
    {/if}
    {#if error}
      <p class={css({ fontSize: '13px', color: 'text.danger' })}>{error}</p>
    {/if}
  </div>
</main>
