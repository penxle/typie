<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import HelpCircleIcon from '~icons/lucide/help-circle';
  import LogOutIcon from '~icons/lucide/log-out';
  import SettingsIcon from '~icons/lucide/settings';
  import UsersIcon from '~icons/lucide/users';
  import { pushState } from '$app/navigation';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { DashboardLayout_Profile_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_Profile_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_Profile_user on User {
        id
        name
        email

        avatar {
          id
          ...Img_image
        }

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  let open = $state(false);
</script>

<Menu placement="bottom-start" bind:open>
  {#snippet button()}
    <div
      class={flex({
        alignItems: 'center',
        gap: '12px',
        paddingX: '8px',
        paddingY: '6px',
        borderRadius: '6px',
        backgroundColor: 'surface.subtle',
        cursor: 'pointer',
        transition: 'common',
        _hover: {
          backgroundColor: 'surface.muted',
        },
      })}
    >
      <Img
        style={css.raw({
          size: '20px',
          borderRadius: 'full',
        })}
        alt={user.data.name}
        image$key={user.data.avatar}
        size={24}
      />

      <span class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', truncate: true })}>
        {user.data.name}
      </span>

      <Icon style={css.raw({ color: 'text.faint' })} icon={ChevronDownIcon} size={16} />
    </div>
  {/snippet}

  <MenuItem
    icon={SettingsIcon}
    onclick={() => {
      pushState('', { shallowRoute: '/preference/profile' });
      mixpanel.track('open_preference_modal', { via: 'profile_menu' });
      open = false;
    }}
  >
    설정
  </MenuItem>

  {#if user.data.subscription}
    <MenuItem external href="https://typie.link/community" icon={UsersIcon} type="link">타이피 커뮤니티</MenuItem>
  {/if}

  <MenuItem external href="https://typie.link/help" icon={HelpCircleIcon} type="link">고객센터</MenuItem>

  <MenuItem
    icon={LogOutIcon}
    onclick={() => {
      mixpanel.track('logout', { via: 'profile_menu' });

      location.href = qs.stringifyUrl({
        url: `${env.PUBLIC_AUTH_URL}/logout`,
        query: {
          redirect_uri: env.PUBLIC_WEBSITE_URL,
        },
      });
    }}
    variant="danger"
  >
    로그아웃
  </MenuItem>
</Menu>
