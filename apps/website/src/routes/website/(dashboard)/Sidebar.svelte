<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import mixpanel from 'mixpanel-browser';
  import { fly } from 'svelte/transition';
  import ChartNoAxesCombinedIcon from '~icons/lucide/chart-no-axes-combined';
  import CircleFadingArrowUpIcon from '~icons/lucide/circle-fading-arrow-up';
  import CogIcon from '~icons/lucide/cog';
  import FolderIcon from '~icons/lucide/folder';
  import PlusIcon from '~icons/lucide/plus';
  import SearchIcon from '~icons/lucide/search';
  import ShieldUserIcon from '~icons/lucide/shield-user';
  import { goto, pushState } from '$app/navigation';
  import { updated } from '$app/state';
  import FaviconDark from '$assets/logos/favicon-dark.svg?component';
  import FaviconLight from '$assets/logos/favicon-light.svg?component';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog } from '$lib/notification';
  import PreferenceModal from './@preference/PreferenceModal.svelte';
  import StatsModal from './@stats/StatsModal.svelte';
  import Notification from './Notification.svelte';
  import Posts from './Posts.svelte';
  import SidebarButton from './SidebarButton.svelte';
  import ThemeSwitch from './ThemeSwitch.svelte';
  import UserMenu from './UserMenu.svelte';
  import type { DashboardLayout_Sidebar_query, DashboardLayout_Sidebar_user } from '$graphql';

  type Props = {
    $query: DashboardLayout_Sidebar_query;
    $user: DashboardLayout_Sidebar_user;
  };

  let { $query: _query, $user: _user }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = fragment(
    _query,
    graphql(`
      fragment DashboardLayout_Sidebar_query on Query {
        announcements {
          id

          ...DashboardLayout_Announcements_postView
        }
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id
        role

        sites {
          id
          ...DashboardLayout_Posts_site
        }

        ...DashboardLayout_UserMenu_user
        ...DashboardLayout_Notification_user
        ...DashboardLayout_Posts_user
        ...DashboardLayout_PreferenceModal_user
      }
    `),
  );

  const createPost = graphql(`
    mutation DashboardLayout_Sidebar_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const app = getAppContext();
</script>

<aside
  class={flex({
    flexDirection: 'column',
    alignItems: 'center',
    gap: '24px',
    flexShrink: '0',
    paddingY: '12px',
    width: '64px',
  })}
>
  <a class={css({ flexShrink: '0', borderRadius: '8px', size: '32px', overflow: 'hidden' })} href="/home">
    <FaviconLight class={css({ size: 'full', _dark: { display: 'none' } })} />
    <FaviconDark class={css({ display: 'none', size: 'full', _dark: { display: 'block' } })} />
  </a>

  <button
    class={center({
      borderWidth: '1px',
      borderColor: 'border.strong',
      borderRadius: '8px',
      size: '32px',
      color: 'text.faint',
      backgroundColor: 'surface.subtle',
      boxShadow: 'small',
      transition: 'common',
      _hover: {
        color: 'text.subtle',
        backgroundColor: 'surface.muted',
        boxShadow: 'medium',
      },
    })}
    onclick={async () => {
      const resp = await createPost({
        siteId: $user.sites[0].id,
      });

      mixpanel.track('create_post', { via: 'sidebar' });

      await goto(`/${resp.entity.slug}`);
    }}
    type="button"
    use:tooltip={{ message: '새 포스트 생성', placement: 'right', offset: 12 }}
  >
    <Icon icon={PlusIcon} size={20} />
  </button>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <SidebarButton
      active={app.preference.current.postsExpanded === false ? app.state.postsOpen : app.preference.current.postsExpanded === 'open'}
      icon={FolderIcon}
      keys={['Mod', 'Shift', 'E']}
      label="내 포스트"
      onclick={() => {
        if (app.preference.current.postsExpanded === false) {
          app.state.postsOpen = !app.state.postsOpen;
        } else {
          app.preference.current.postsExpanded = app.preference.current.postsExpanded === 'open' ? 'closed' : 'open';
        }
      }}
    />
    <SidebarButton icon={SearchIcon} label="검색" onclick={() => (app.state.commandPaletteOpen = true)} />
    <Notification {$user} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <SidebarButton
      icon={ChartNoAxesCombinedIcon}
      label="통계"
      onclick={() => {
        app.state.statsOpen = true;
        mixpanel.track('open_stats_modal');
      }}
    />
  </div>

  <div class={css({ flexGrow: '1' })}></div>

  {#if updated.current}
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div in:fly={{ y: '4px', duration: 500 }}>
        <SidebarButton
          icon={CircleFadingArrowUpIcon}
          iconStyle={css.raw({
            color: 'text.brand',
            animationName: 'alarm',
            animationDuration: '2s',
            animationDelay: '500ms',
            animationIterationCount: 'infinite',
            animationTimingFunction: 'cubic-bezier(0.36, 0.07, 0.19, 0.97)',
          })}
          label="새로운 업데이트가 있어요"
          onclick={() => {
            Dialog.confirm({
              title: '새로운 업데이트가 있어요',
              message: `이용 가능한 업데이트가 있어요. 새로고침 후 이용해주세요.`,
              actionLabel: '새로고침',
              actionHandler: () => {
                location.reload();
              },
              cancelLabel: '나중에',
            });

            mixpanel.track('click_new_update_alert', { via: 'sidebar' });
          }}
        />
      </div>
    </div>
  {/if}

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    {#if $user.role === 'ADMIN'}
      <SidebarButton as="a" href="/admin" icon={ShieldUserIcon} label="어드민" />
    {/if}

    <!-- <Announcements $posts={$query.announcements} /> -->

    <!-- <SidebarButton as="a" href="https://help.typie.co" icon={CircleHelpIcon} label="도움말" rel="noopener noreferrer" target="_blank" /> -->

    <div class={center({ width: 'full' })}>
      <ThemeSwitch />
    </div>

    <SidebarButton
      icon={CogIcon}
      label="설정"
      onclick={() => {
        pushState('', { shallowRoute: '/preference/account' });
        mixpanel.track('open_preference_modal', { via: 'sidebar' });
      }}
    />
  </div>

  <UserMenu {$user} />
</aside>

<Posts $site={$user.sites[0]} {$user} />

<PreferenceModal {$user} />
<StatsModal />
