<script lang="ts">
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Helmet, RingSpinner } from '$lib/components';
  import { deserializeOAuthState } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const authorizeSignUpEmail = graphql(`
    mutation EmailPage_AuthorizeSignUpEmail_Mutation($input: AuthorizeSignUpEmailInput!) {
      authorizeSignUpEmail(input: $input)
    }
  `);

  onMount(async () => {
    const resp = await authorizeSignUpEmail({
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      code: page.url.searchParams.get('code')!,
    });

    location.href = qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/authorize`,
      query: {
        client_id: env.PUBLIC_OIDC_CLIENT_ID,
        response_type: 'code',
        ...deserializeOAuthState(resp),
      },
    });
  });
</script>

<Helmet title="이메일 인증 중..." />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>이메일 인증 중...</h1>
    <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'gray.500' })}>잠시만 기다려주세요.</div>
  </div>

  <div class={center({ height: '100px' })}>
    <RingSpinner style={css.raw({ size: '50px', color: 'brand.500' })} />
  </div>
</div>
