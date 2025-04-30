<script lang="ts">
  import CreditCardIcon from '~icons/lucide/credit-card';
  import PanelTopIcon from '~icons/lucide/panel-top';
  import ShieldCheckIcon from '~icons/lucide/shield-check';
  import UserIcon from '~icons/lucide/user';
  import { replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import { Icon, Modal } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import AccountTab from './AccountTab.svelte';
  import BillingTab from './BillingTab.svelte';
  import IdentityTab from './IdentityTab.svelte';
  import SiteTab from './SiteTab.svelte';
  import type { Component } from 'svelte';
  import type { DashboardLayout_PreferenceModal_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_user;
  };

  type Tab = {
    path: string;
    label: string;
    icon: Component;
    component: Component<never>;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_user on User {
        id

        plan {
          id
        }

        ...DashboardLayout_PreferenceModal_AccountTab_user
        ...DashboardLayout_PreferenceModal_BillingTab_user
        ...DashboardLayout_PreferenceModal_IdentityTab_user
        ...DashboardLayout_PreferenceModal_SiteTab_user
      }
    `),
  );

  const tabs = [
    {
      path: '/preference/account',
      label: '계정',
      icon: UserIcon,
      component: AccountTab,
    },
    ...($user.plan
      ? [
          {
            path: '/preference/site',
            label: '사이트',
            icon: PanelTopIcon,
            component: SiteTab,
          },
        ]
      : []),
    {
      path: '/preference/identity',
      label: '인증',
      icon: ShieldCheckIcon,
      component: IdentityTab,
    },
    {
      path: '/preference/billing',
      label: '결제',
      icon: CreditCardIcon,
      component: BillingTab,
    },
  ] satisfies Tab[];

  const currentTab = $derived(tabs.find((tab) => tab.path === page.state.shallowRoute));
</script>

<Modal style={css.raw({ maxWidth: '1080px', height: 'full', maxHeight: '600px' })} onclose={() => history.back()} open={!!currentTab}>
  <div class={flex({ height: 'full' })}>
    <div class={css({ flex: 'none', paddingY: '28px', paddingX: '8px', width: '240px', backgroundColor: 'gray.50' })}>
      <nav class={flex({ direction: 'column', gap: '4px' })}>
        {#each tabs as { icon, path, label } (path)}
          <button
            class={flex({
              align: 'center',
              gap: '4px',
              borderRadius: '4px',
              padding: '8px',
              fontSize: '14px',
              color: 'gray.700',
              _hover: { backgroundColor: 'gray.200' },
              _selected: { color: 'gray.900', fontWeight: 'semibold', backgroundColor: 'gray.100' },
            })}
            aria-selected={currentTab?.path === path}
            onclick={() => {
              replaceState('', { shallowRoute: path });
            }}
            role="tab"
            type="button"
          >
            <Icon {icon} size={16} />
            <span>{label}</span>
          </button>
        {/each}
      </nav>
    </div>

    <div class={css({ paddingX: '24px', paddingY: '30px', width: 'full', overflowY: 'auto' })}>
      {#if currentTab}
        {@const Component = currentTab.component}

        <Component {$user} />
      {/if}
    </div>
  </div>
</Modal>
