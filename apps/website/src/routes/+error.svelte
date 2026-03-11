<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { Grain } from '@typie/ui/effects';
  import dayjs from 'dayjs';
  import HeadphonesIcon from '~icons/lucide/headphones';
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

    {#if error?.maintenance}
      <h1 class={css({ fontSize: '24px', fontWeight: 'extrabold' })}>
        {error.maintenance.title}
      </h1>

      <div class={css({ fontSize: '14px', color: 'text.faint', whiteSpace: 'pre-line' })}>
        {error.maintenance.message}
      </div>

      {#if error.maintenance.until}
        <div class={css({ fontSize: '13px', color: 'text.disabled' })}>
          점검 종료 예정: {dayjs(error.maintenance.until).formatAsDateTime()}
        </div>
      {/if}

      <a
        class={css({
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '14px',
          paddingY: '8px',
          fontSize: '13px',
          color: 'text.subtle',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '6px',
          transition: 'common',
          _hover: { borderColor: 'border.strong' },
        })}
        href="https://penxle.channel.io/home"
        rel="noopener noreferrer"
        target="_blank"
      >
        <Icon icon={HeadphonesIcon} size={14} />
        고객센터
      </a>
    {:else}
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

      {#if typeof window !== 'undefined' && !window.__webview__}
        <Button style={css.raw({ width: 'full' })} href="/" size="lg" type="link">홈으로 돌아가기</Button>
      {/if}

      {#if error?.eventId}
        <p class={css({ fontFamily: 'mono', fontSize: '12px', color: 'text.disabled' })}>코드: {error.eventId}</p>
      {/if}
    {/if}
  </div>
</div>
