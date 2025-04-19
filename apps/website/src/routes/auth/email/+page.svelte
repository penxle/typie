<script lang="ts">
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { deserializeOAuthState } from '$lib/utils';
  import { center } from '$styled-system/patterns';

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

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>logging in...</div>
</div>
