<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Grain } from '@typie/ui/effects';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';

  let error = $derived(page.error);
  const seed = Math.floor(Math.random() * 1000);
</script>

<div class={center({ width: '[100dvw]', height: '[100dvh]' })}>
  <Grain style={css.raw({ position: 'absolute', inset: '0' })} freq={2.2} opacity={0.75} {seed} />

  <div
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      gap: '20px',
      borderRadius: '12px',
      marginX: '20px',
      padding: { base: '24px', lg: '48px' },
      width: 'full',
      maxWidth: '400px',
      backgroundColor: 'surface.default',
      textAlign: 'center',
      boxShadow: 'medium',
      zIndex: '1',
    })}
  >
    <Logo class={css({ height: '32px' })} />

    <h1 class={css({ fontSize: '24px', fontWeight: 'extrabold' })}>
      {#if page.status === 404}
        존재하지 않는 페이지에요
      {:else}
        앗! 문제가 발생했어요
      {/if}
    </h1>
    <div class={css({ fontSize: '14px', color: 'text.faint' })}>
      {#if page.status === 404}
        입력한 주소를 다시 한 번 확인해주세요.
      {:else if error?.code === 'unexpected_error'}
        잠시 후 다시 시도해주세요.
      {:else if error?.message}
        {error.message}
      {/if}
    </div>

    <Button style={css.raw({ width: 'full', height: '40px' })} href="/" size="lg" type="link">홈으로 돌아가기</Button>

    {#if error?.eventId}
      <p class={css({ fontFamily: 'mono', fontSize: '12px', color: 'text.disabled' })}>코드: {error.eventId}</p>
    {/if}
  </div>
</div>
