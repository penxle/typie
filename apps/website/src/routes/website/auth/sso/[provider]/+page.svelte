<script lang="ts">
  import { onMount } from 'svelte';
  import { match } from 'ts-pattern';
  import { SingleSignOnProvider } from '@/enums';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { center } from '$styled-system/patterns';

  const authorizeSingleSignOn = graphql(`
    mutation SSOProviderPage_AuthorizeSingleSignOn_Mutation($input: AuthorizeSingleSignOnInput!) {
      authorizeSingleSignOn(input: $input) {
        id
      }
    }
  `);

  onMount(async () => {
    await authorizeSingleSignOn({
      provider: match(page.params.provider)
        .with('google', () => SingleSignOnProvider.GOOGLE)
        .with('kakao', () => SingleSignOnProvider.KAKAO)
        .with('naver', () => SingleSignOnProvider.NAVER)
        .run(),
      params: Object.fromEntries(page.url.searchParams),
    });

    await goto('/', {
      replaceState: true,
    });
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div>logging in with {page.params.provider}...</div>
</div>
