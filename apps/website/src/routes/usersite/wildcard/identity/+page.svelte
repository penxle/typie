<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import { TypieError } from '@/errors';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$graphql';
  import { Helmet, RingSpinner } from '$lib/components';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';

  const verifyPersonalIdentity = graphql(`
    mutation UsersiteWildcardIdentityPage_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
      verifyPersonalIdentity(input: $input) {
        id
      }
    }
  `);

  onMount(async () => {
    const redirectUri = sessionStorage.getItem('redirect_uri');
    sessionStorage.removeItem('redirect_uri');

    try {
      await verifyPersonalIdentity({
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        identityVerificationId: page.url.searchParams.get('identityVerificationId')!,
      });

      mixpanel.track('verify_personal_identity_success');
      Toast.success('본인인증이 완료되었어요');
    } catch (err) {
      const errorMessages: Record<string, string> = {
        identity_verification_failed: '인증에 실패했습니다.',
        same_identity_exists: '이미 다른 계정에 인증된 정보입니다.',
      };

      if (err instanceof TypieError) {
        const message = errorMessages[err.code] || err.code;
        Toast.error(message);
      }
    } finally {
      await goto(redirectUri ?? '/', { replaceState: true });
    }
  });
</script>

<Helmet title="본인인증 중..." />

<div
  style:--grid-line-color={token('colors.brand.100')}
  style:--cross-line-color={token('colors.brand.50')}
  style:--grid-size="30px"
  style:--line-thickness="1px"
  class={center({
    padding: '20px',
    width: '[100dvw]',
    minHeight: '[100dvh]',
    height: 'full',
    overflowY: 'auto',
    backgroundColor: 'white',
    backgroundImage:
      '[repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) calc(var(--grid-size) - var(--line-thickness)), var(--grid-line-color) var(--grid-size)), repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2 - var(--line-thickness)), var(--cross-line-color) calc(var(--grid-size) / 2), transparent calc(var(--grid-size) / 2), transparent var(--grid-size))]',
    backgroundSize: 'var(--grid-size) var(--grid-size)',
  })}
>
  <div
    class={css({
      borderRadius: '12px',
      padding: { base: '24px', lg: '48px' },
      maxWidth: '400px',
      width: 'full',
      backgroundColor: 'white',
      boxShadow: 'medium',
    })}
  >
    <div class={flex({ flexDirection: 'column', gap: '24px' })}>
      <div class={flex({ justifyContent: 'flex-start' })}>
        <Logo class={css({ height: '20px' })} />
      </div>

      <div class={flex({ flexDirection: 'column', gap: '4px' })}>
        <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>본인인증 중...</h1>
        <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'gray.500' })}>잠시만 기다려주세요.</div>
      </div>

      <div class={center({ height: '100px' })}>
        <RingSpinner style={css.raw({ size: '50px', color: 'brand.500' })} />
      </div>
    </div>
  </div>
</div>
