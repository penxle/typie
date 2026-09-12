<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { serializeOAuthState } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { pollBootstrapAssertion } from '$lib/bootstrap';
  import { EnvironmentBanner } from '$lib/components';
  import { hydrateQuery } from '$lib/graphql';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  onMount(pollBootstrapAssertion);

  onMount(() => {
    if (!query.data.me && !document.cookie.includes('typie-af')) {
      location.assign(
        qs.stringifyUrl({
          url: `${env.PUBLIC_AUTH_URL}/authorize`,
          query: {
            client_id: env.PUBLIC_OIDC_CLIENT_ID,
            response_type: 'code',
            redirect_uri: `${page.url.origin}/authorize`,
            state: serializeOAuthState({ redirect_uri: page.url.href }),
            prompt: 'none',
          },
        }),
      );
    }
  });

  $effect(() => {
    if (!query.data.me) {
      return;
    }

    mixpanel.identify(query.data.me.id);

    mixpanel.people.set({
      $email: query.data.me.email,
      $name: query.data.me.name,
      $avatar: qs.stringifyUrl({ url: query.data.me.avatar.url, query: { s: 256, f: 'png' } }),
    });
  });
</script>

<div class={flex({ flexDirection: 'column', minHeight: '[100dvh]' })}>
  <EnvironmentBanner />

  {@render children()}
</div>
