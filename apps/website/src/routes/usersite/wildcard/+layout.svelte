<script lang="ts">
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { serializeOAuthState } from '$lib/utils';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const query = graphql(`
    query UsersiteWildcard_Layout_Query {
      me {
        id
      }
    }
  `);

  onMount(() => {
    if (!$query.me && !document.cookie.includes('typie-af')) {
      location.href = qs.stringifyUrl({
        url: `${env.PUBLIC_AUTH_URL}/authorize`,
        query: {
          client_id: env.PUBLIC_OIDC_CLIENT_ID,
          response_type: 'code',
          redirect_uri: `${page.url.origin}/authorize`,
          state: serializeOAuthState({ redirect_uri: page.url.href }),
          prompt: 'none',
        },
      });
    }
  });
</script>

{@render children()}
