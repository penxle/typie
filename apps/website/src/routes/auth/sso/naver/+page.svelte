<script lang="ts">
  import { onMount } from 'svelte';
  import { SingleSignOnProvider } from '@/enums';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { accessToken } from '$lib/graphql';
  import { center } from '$styled-system/patterns';

  const authorizeSingleSignOn = graphql(`
    mutation SSONaverPage_AuthorizeSingleSignOn_Mutation($input: AuthorizeSingleSignOnInput!) {
      authorizeSingleSignOn(input: $input) {
        accessToken
      }
    }
  `);

  onMount(async () => {
    const resp = await authorizeSingleSignOn({
      provider: SingleSignOnProvider.NAVER,
      params: Object.fromEntries(page.url.searchParams),
    });

    $accessToken = resp.accessToken;

    await goto('/', {
      replaceState: true,
    });
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>로그인 중...</div>
</div>
