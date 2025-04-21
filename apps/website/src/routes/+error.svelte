<script lang="ts">
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { Button } from '$lib/components';
  import { Grain } from '$lib/effects';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  let error = $derived(page.error);
  const seed = Math.floor(Math.random() * 1000);
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <Grain style={css.raw({ position: 'absolute', inset: '0' })} freq={2.2} opacity={0.75} {seed} />

  <div
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      gap: '16px',
      borderRadius: '12px',
      padding: '48px',
      width: 'full',
      maxWidth: '400px',
      backgroundColor: 'white',
      boxShadow: 'large',
      textAlign: 'center',
      zIndex: '1',
    })}
  >
    <Logo class={css({ height: '20px' })} />

    <h1 class={css({ fontSize: '24px', fontWeight: 'extrabold' })}>앗! 문제가 발생했어요</h1>

    {#if error?.message}
      <p class={css({ fontSize: '14px', color: 'gray.600' })}>{error.message}</p>
    {/if}

    {#if page.status}
      <p class={css({ fontSize: '12px', color: 'gray.400' })}>에러 코드: {page.status}</p>
    {/if}

    <Button style={css.raw({ width: 'full', height: '40px' })} href="/" size="lg" type="link">홈페이지로 돌아가기</Button>
  </div>
</div>
