<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Button, Img, Menu, MenuItem } from '$lib/components';
  import { AdminImpersonateBanner } from '$lib/components/admin';
  import { serializeOAuthState } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  let { children }: Props = $props();

  const query = graphql(`
    query UsersiteWildcard_Layout_Query {
      me {
        id
        name
        email

        avatar {
          id
          url

          ...Img_image
        }
      }

      ...AdminImpersonateBanner_query
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

  $effect(() => {
    if ($query.me) {
      mixpanel.identify($query.me.id);

      mixpanel.people.set({
        $email: $query.me.email,
        $name: $query.me.name,
        $avatar: qs.stringifyUrl({ url: $query.me.avatar.url, query: { s: 256, f: 'png' } }),
      });
    }
  });

  const authorizeUrl = $derived(
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/authorize`,
      query: {
        client_id: env.PUBLIC_OIDC_CLIENT_ID,
        response_type: 'code',
        redirect_uri: `${page.url.origin}/authorize`,
        state: serializeOAuthState({ redirect_uri: page.url.href }),
      },
    }),
  );
</script>

<div class={flex({ flexDirection: 'column', minHeight: '[100dvh]' })}>
  <header
    class={flex({
      flexDirection: 'column',
      position: 'sticky',
      top: '0',
      zIndex: '50',
      backgroundColor: 'surface.default',
    })}
  >
    <AdminImpersonateBanner {$query} />

    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        borderBottomWidth: '1px',
        borderColor: 'border.default',
        paddingX: '20px',
        height: '52px',
        backgroundColor: 'surface.default',
      })}
    >
      <a class={css({ flexShrink: '0', height: '20px' })} href={env.PUBLIC_WEBSITE_URL} rel="noopener noreferrer" target="_blank">
        <Logo class={css({ height: 'full' })} />
      </a>

      {#if $query.me}
        <Menu>
          {#snippet button()}
            {#if $query.me?.avatar}
              <Img
                style={css.raw({ size: '32px', borderWidth: '1px', borderColor: 'border.subtle', borderRadius: 'full' })}
                $image={$query.me.avatar}
                alt={`${$query.me.name}의 아바타`}
                size={32}
              />
            {:else}
              <div
                class={css({
                  size: '32px',
                  borderWidth: '1px',
                  borderColor: 'border.subtle',
                  borderRadius: 'full',
                  backgroundColor: 'interactive.hover',
                })}
              ></div>
            {/if}
          {/snippet}

          <MenuItem href={`${env.PUBLIC_WEBSITE_URL}/home`} type="link">내 홈으로</MenuItem>
          <MenuItem
            onclick={() => {
              mixpanel.track('logout', { via: 'header' });

              location.href = qs.stringifyUrl({
                url: `${env.PUBLIC_AUTH_URL}/logout`,
                query: {
                  redirect_uri: page.url.href,
                },
              });
            }}
          >
            로그아웃
          </MenuItem>
        </Menu>
      {:else}
        <Button external href={authorizeUrl} size="sm" type="link" variant="primary">시작하기</Button>
      {/if}
    </div>
  </header>

  <main class={flex({ flexDirection: 'column', flex: '1' })}>
    {@render children()}
  </main>
</div>
