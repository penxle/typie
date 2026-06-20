<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { serializeOAuthState } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import EclipseIcon from '~icons/lucide/eclipse';
  import HouseIcon from '~icons/lucide/house';
  import LogOutIcon from '~icons/lucide/log-out';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import SunIcon from '~icons/lucide/sun';
  import { page } from '$app/state';
  import WordmarkBlack from '$assets/logos/wordmark-black.svg?component';
  import WordmarkWhite from '$assets/logos/wordmark-white.svg?component';
  import { env } from '$env/dynamic/public';
  import { EnvironmentBanner, Img } from '$lib/components';
  import { AdminImpersonateBanner } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';
  import type { Theme } from '@typie/ui/context';
  import type { Component } from 'svelte';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const themes: Record<Theme, { icon: Component; label: string }> = {
    auto: { icon: MonitorIcon, label: '시스템 설정' },
    light: { icon: SunIcon, label: '라이트' },
    dark: { icon: MoonIcon, label: '다크' },
  };

  const themeNames: Theme[] = ['auto', 'light', 'dark'];
  const theme = getThemeContext();

  let themeMenuOpen = $state(false);
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
  <EnvironmentBanner />
  <header
    class={flex({
      flexDirection: 'column',
      position: 'sticky',
      top: '0',
      zIndex: '50',
      backgroundColor: 'surface.default',
    })}
  >
    <AdminImpersonateBanner query$key={query.data} />

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
      <a class={css({ flexShrink: '0', height: '18px' })} href={env.PUBLIC_WEBSITE_URL} rel="noopener noreferrer" target="_blank">
        <WordmarkBlack class={css({ height: 'full', _dark: { display: 'none' } })} />
        <WordmarkWhite class={css({ height: 'full', display: 'none', _dark: { display: 'block' } })} />
      </a>

      <div class={flex({ flex: '1' })}></div>

      <div class={flex({ alignItems: 'center', gap: '12px' })}>
        <Menu
          style={css.raw({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            size: '32px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: 'full',
            color: 'text.subtle',
            backgroundColor: 'surface.default',
            transition: 'common',
            _hover: {
              backgroundColor: 'surface.subtle',
              color: 'text.default',
            },
          })}
          offset={8}
          placement="bottom-end"
          bind:open={themeMenuOpen}
        >
          {#snippet button()}
            <Icon icon={EclipseIcon} size={16} />
          {/snippet}

          {#each themeNames as name (name)}
            <MenuItem
              icon={themes[name].icon}
              onclick={() => {
                mixpanel.track('switch_theme', { old: theme.currentTheme, new: name, via: 'header' });
                theme.currentTheme = name;
              }}
            >
              {themes[name].label}

              {#if theme.currentTheme === name}
                <Icon style={css.raw({ marginLeft: 'auto', color: 'text.brand' })} icon={CheckIcon} size={14} />
              {/if}
            </MenuItem>
          {/each}
        </Menu>

        {#if query.data.me}
          <Menu>
            {#snippet button()}
              {#if query.data.me?.avatar}
                <Img
                  style={css.raw({ size: '32px', borderWidth: '1px', borderColor: 'border.subtle', borderRadius: 'full' })}
                  alt={`${query.data.me.name}의 아바타`}
                  image$key={query.data.me.avatar}
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

            <MenuItem href={env.PUBLIC_WEBSITE_URL} icon={HouseIcon} type="link">내 홈으로</MenuItem>
            <MenuItem
              icon={LogOutIcon}
              onclick={() => {
                mixpanel.track('logout', { via: 'header' });

                location.assign(
                  qs.stringifyUrl({
                    url: '/logout',
                    query: {
                      redirect_uri: page.url.href,
                    },
                  }),
                );
              }}
            >
              로그아웃
            </MenuItem>
          </Menu>
        {:else}
          <Button external href={authorizeUrl} size="sm" type="link" variant="primary">시작하기</Button>
        {/if}
      </div>
    </div>
  </header>

  <main class={flex({ flexDirection: 'column', flex: '1' })}>
    {@render children()}
  </main>
</div>
