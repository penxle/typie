<script lang="ts">
  import ChartNoAxesCombinedIcon from '~icons/lucide/chart-no-axes-combined';
  import CircleHelpIcon from '~icons/lucide/circle-help';
  import CogIcon from '~icons/lucide/cog';
  import FolderIcon from '~icons/lucide/folder';
  import PlusIcon from '~icons/lucide/plus';
  import SearchIcon from '~icons/lucide/search';
  import { goto, pushState } from '$app/navigation';
  import Favicon from '$assets/logos/favicon.svg?component';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PreferenceModal from './@preference/PreferenceModal.svelte';
  import StatsModal from './@stats/StatsModal.svelte';
  import Notification from './Notification.svelte';
  import Posts from './Posts.svelte';
  import SidebarButton from './SidebarButton.svelte';
  import UserMenu from './UserMenu.svelte';
  import type { DashboardLayout_Sidebar_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_Sidebar_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id

        sites {
          id
          ...DashboardLayout_Posts_site
        }

        ...DashboardLayout_UserMenu_user
        ...DashboardLayout_Notification_user
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
    <Favicon class={css({ size: 'full' })} />
  </a>

  <button
    class={center({
      borderWidth: '1px',
      borderColor: 'gray.300',
      borderRadius: '8px',
      size: '32px',
      color: 'gray.500',
      backgroundColor: 'gray.50',
      boxShadow: 'small',
      transition: 'common',
      _hover: {
        color: 'gray.700',
        backgroundColor: 'gray.100',
        boxShadow: 'medium',
      },
    })}
    onclick={async () => {
      const resp = await createPost({
        siteId: $user.sites[0].id,
      });

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
    <SidebarButton icon={ChartNoAxesCombinedIcon} label="통계" onclick={() => (app.state.statsOpen = true)} />
  </div>

  <div class={css({ flexGrow: '1' })}></div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <SidebarButton as="a" href="https://help.typie.co" icon={CircleHelpIcon} label="도움말" rel="noopener noreferrer" target="_blank" />
    <SidebarButton icon={CogIcon} label="설정" onclick={() => pushState('', { shallowRoute: '/preference/account' })} />
  </div>

  <UserMenu {$user} />
</aside>

<Posts $site={$user.sites[0]} />

<PreferenceModal {$user} />
<StatsModal />
