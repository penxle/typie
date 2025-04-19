<script lang="ts">
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { match } from 'ts-pattern';
  import { SingleSignOnProvider } from '@/enums';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { deserializeOAuthState } from '$lib/utils';
  import { center } from '$styled-system/patterns';

  const authorizeSingleSignOn = graphql(`
    mutation SSOProviderPage_AuthorizeSingleSignOn_Mutation($input: AuthorizeSingleSignOnInput!) {
      authorizeSingleSignOn(input: $input)
    }
  `);

  onMount(async () => {
    const resp = await authorizeSingleSignOn({
      provider: match(page.params.provider)
        .with('google', () => SingleSignOnProvider.GOOGLE)
        .with('kakao', () => SingleSignOnProvider.KAKAO)
        .with('naver', () => SingleSignOnProvider.NAVER)
        .run(),
      params: Object.fromEntries(page.url.searchParams),
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

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>logging in with {page.params.provider}...</div>
</div>
