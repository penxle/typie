<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, RingSpinner } from '@typie/ui/components';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';

  type Session = { name: string; email: string; avatarUrl: string };

  let session = $state<Session | null>(null);
  let cancelled = $state(false);

  const query = Object.fromEntries(page.url.searchParams);
  const authorizeUrl = qs.stringifyUrl({
    url: `${env.PUBLIC_AUTH_URL}/authorize`,
    query: {
      client_id: query.client_id,
      response_type: 'code',
      redirect_uri: query.redirect_uri,
      scope: query.scope,
      state: query.state,
      code_challenge: query.code_challenge,
      code_challenge_method: query.code_challenge_method,
    },
  });

  onMount(async () => {
    const response = await fetch(`${env.PUBLIC_AUTH_URL}/session`, { credentials: 'include' });
    if (!response.ok) {
      location.replace(authorizeUrl);
      return;
    }
    session = await response.json();
  });

  const continueWithAccount = () => {
    location.replace(authorizeUrl);
  };

  const switchAccount = () => {
    const consentUrl = qs.stringifyUrl({ url: authorizeUrl, query: { prompt: 'consent' } });
    location.replace(qs.stringifyUrl({ url: `${env.PUBLIC_AUTH_URL}/logout`, query: { redirect_uri: consentUrl } }));
  };
</script>

<Helmet title="로그인 확인" />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  {#if cancelled}
    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold', wordBreak: 'keep-all' })}>요청을 취소했어요</h1>
      <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint', wordBreak: 'keep-all' })}>이 창은 닫아도 돼요.</div>
    </div>
  {:else}
    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold', wordBreak: 'keep-all' })}>
        데스크톱 앱에 로그인하세요
      </h1>
      <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint', wordBreak: 'keep-all' })}>
        직접 시작한 요청이 맞는지 확인해주세요.
      </div>
    </div>

    <div
      class={flex({
        alignItems: 'center',
        gap: '12px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '8px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        backgroundColor: 'surface.subtle',
        minHeight: '[64px]',
      })}
    >
      {#if session}
        <img
          class={css({ flexShrink: '0', size: '40px', borderRadius: 'full', objectFit: 'cover', backgroundColor: 'surface.muted' })}
          alt=""
          src={session.avatarUrl}
        />
        <div class={flex({ flexDirection: 'column', gap: '2px', minWidth: '0' })}>
          <div
            class={css({ fontSize: '14px', fontWeight: 'semibold', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}
          >
            {session.name}
          </div>
          <div class={css({ fontSize: '13px', color: 'text.faint', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
            {session.email}
          </div>
        </div>
      {:else}
        <RingSpinner style={css.raw({ size: '20px', color: 'text.faint' })} />
      {/if}
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <Button disabled={!session} onclick={continueWithAccount}>이 계정으로 계속</Button>
      <Button disabled={!session} onclick={switchAccount} variant="secondary">다른 계정으로 로그인</Button>
      <button
        class={css({
          alignSelf: 'center',
          fontSize: '13px',
          color: 'text.faint',
          _hover: { textDecoration: 'underline', textUnderlineOffset: '2px' },
        })}
        onclick={() => (cancelled = true)}
        type="button"
      >
        취소
      </button>
    </div>
  {/if}
</div>
