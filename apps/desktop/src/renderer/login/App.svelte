<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { onMount } from 'svelte';
  import GlobeIcon from '~icons/lucide/globe';
  import Logo from '../assets/logo.svg?component';

  const params = new URLSearchParams(location.search);
  const version = params.get('version') ?? '';
  const envName = params.get('env') ?? '';

  let phase = $state<'idle' | 'waiting'>('idle');
  let loginUrl = $state<string | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);
  let showFallback = $state(false);
  let primary = $state<HTMLButtonElement | null>(null);
  let fallbackTimer: ReturnType<typeof setTimeout> | undefined;
  const FALLBACK_DELAY_MS = 8000;

  const scheduleFallback = () => {
    clearTimeout(fallbackTimer);
    showFallback = false;
    fallbackTimer = setTimeout(() => (showFallback = true), FALLBACK_DELAY_MS);
  };

  onMount(() => {
    primary?.focus();
    return window.shell.onAuthError?.((message) => {
      clearTimeout(fallbackTimer);
      showFallback = false;
      error = message;
      phase = 'idle';
    });
  });

  const login = async () => {
    error = null;
    copied = false;
    phase = 'waiting';
    scheduleFallback();
    try {
      const url = await window.shell.login?.();
      loginUrl = url ?? null;
    } catch {
      error = '브라우저를 열 수 없어요. 잠시 후 다시 시도해주세요.';
      phase = 'idle';
    }
  };

  const cancel = () => {
    window.shell.cancelLogin?.();
    clearTimeout(fallbackTimer);
    showFallback = false;
    phase = 'idle';
    loginUrl = null;
    copied = false;
    primary?.focus();
  };

  const copyLink = async () => {
    if (!loginUrl) return;
    await navigator.clipboard.writeText(loginUrl);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  };

  const buttonStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '8px',
    width: 'full',
    height: '44px',
    borderRadius: '8px',
    fontSize: '15px',
    fontWeight: 'semibold',
    transitionProperty: '[background-color, transform, opacity]',
    transitionDuration: '[100ms]',
    _active: { transform: 'scale(0.98)' },
    _disabled: { opacity: '40', cursor: 'default', transform: 'none' },
  });

  const linkStyle = css.raw({
    fontWeight: 'medium',
    color: 'text.muted',
    _hover: { color: 'text.default', textDecoration: 'underline', textUnderlineOffset: '2px' },
  });

  const linkClass = css(linkStyle);
  const primaryClass = css(buttonStyle, {
    backgroundColor: 'accent.default',
    color: 'surface.default',
    _hover: { backgroundColor: '[color-mix(in oklch, token(colors.accent.default) 88%, black)]' },
  });
  const cancelClass = css(linkStyle, { alignSelf: 'center', fontSize: '13px', color: 'text.hint' });
</script>

<div
  style:-webkit-app-region="drag"
  class={css({ position: 'fixed', top: '0', left: '0', right: '0', height: '[40px]', zIndex: '1' })}
></div>

<main
  style:--grid-line-color={token('colors.border.default')}
  style:--cross-line-color={token('colors.border.hairline')}
  style:--grid-size="30px"
  style:--line-thickness="1px"
  class={center({
    position: 'relative',
    flexDirection: 'column',
    height: '[100vh]',
    padding: '20px',
    backgroundColor: 'surface.default',
    backgroundImage:
      '[repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size))]',
    backgroundSize: 'var(--grid-size) var(--grid-size)',
  })}
>
  <section
    class={flex({
      flexDirection: 'column',
      gap: '24px',
      width: 'full',
      maxWidth: '400px',
      padding: '40px',
      borderRadius: '12px',
      backgroundColor: 'surface.default',
      boxShadow: 'md',
    })}
    aria-labelledby="login-title"
  >
    <div class={flex({ justifyContent: 'flex-start' })}>
      <Logo class={css({ size: '32px' })} />
    </div>

    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <h1 id="login-title" class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold', wordBreak: 'keep-all' })}>
        타이피에 오신 것을 환영해요!
      </h1>
      <p class={css({ fontSize: { base: '13px', lg: '14px' }, lineHeight: '[1.6]', color: 'text.hint', wordBreak: 'keep-all' })}>
        작성부터 공유까지, 타이피 하나로 해결해요.
      </p>
    </div>

    {#if phase === 'idle'}
      <div class={flex({ flexDirection: 'column', gap: '10px' })}>
        <button bind:this={primary} class={primaryClass} onclick={login} type="button">
          <GlobeIcon class={css({ size: '18px' })} />
          브라우저로 로그인
        </button>
        <p class={css({ fontSize: '12px', textAlign: 'center', color: 'text.hint' })}>로그인은 브라우저에서 안전하게 진행돼요.</p>
        {#if error}
          <p class={css({ fontSize: '13px', textAlign: 'center', color: 'danger.default', wordBreak: 'keep-all' })} role="alert">{error}</p>
        {/if}
      </div>
    {:else}
      <div class={flex({ flexDirection: 'column', gap: '16px' })} aria-live="polite">
        <div
          class={flex({
            alignItems: 'center',
            gap: '12px',
            paddingX: '16px',
            paddingY: '14px',
            borderRadius: '8px',
            borderWidth: '1px',
            borderColor: 'border.hairline',
            backgroundColor: 'surface.canvas',
          })}
        >
          <span
            class={css({
              flexShrink: '0',
              size: '18px',
              borderRadius: 'full',
              borderWidth: '2px',
              borderColor: 'border.default',
              borderTopColor: 'accent.default',
              animation: 'spin 0.8s linear infinite',
            })}
            aria-hidden="true"
          ></span>
          <div class={flex({ flexDirection: 'column', gap: '2px', minWidth: '0' })}>
            <div class={css({ fontSize: '14px', fontWeight: 'semibold' })}>브라우저에서 로그인을 마쳐주세요</div>
            <div class={css({ fontSize: '12px', color: 'text.hint', wordBreak: 'keep-all' })}>
              {#if showFallback}
                브라우저가 열리지 않았나요?
                <button class={linkClass} onclick={login} type="button">다시 열기</button>
                <span aria-hidden="true">·</span>
                <button class={linkClass} disabled={!loginUrl} onclick={copyLink} type="button">{copied ? '복사됨' : '링크 복사'}</button>
              {:else}
                로그인이 끝나면 자동으로 돌아와요.
              {/if}
            </div>
          </div>
        </div>

        <button class={cancelClass} onclick={cancel} type="button">취소</button>
      </div>
    {/if}
  </section>

  {#if version}
    <div class={css({ position: 'absolute', bottom: '16px', fontSize: '11px', color: 'text.hint', userSelect: 'none' })}>
      v{version}{envName && envName !== 'prod' ? ` · ${envName}` : ''}
    </div>
  {/if}
</main>
