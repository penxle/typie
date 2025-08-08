<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import CreditCardIcon from '~icons/lucide/credit-card';
  import FlaskConicalIcon from '~icons/lucide/flask-conical';
  import KeyboardIcon from '~icons/lucide/keyboard';
  import PanelTopIcon from '~icons/lucide/panel-top';
  import PencilIcon from '~icons/lucide/pencil';
  import ShieldCheckIcon from '~icons/lucide/shield-check';
  import UserIcon from '~icons/lucide/user';
  import { replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import { Icon, Modal } from '$lib/components';
  import AccountTab from './AccountTab.svelte';
  import BillingTab from './BillingTab.svelte';
  import EditorTab from './EditorTab.svelte';
  import IdentityTab from './IdentityTab.svelte';
  import LaboratoryTab from './LaboratoryTab.svelte';
  import ShortcutsTab from './ShortcutsTab.svelte';
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

        subscription {
          id
        }

        ...DashboardLayout_PreferenceModal_AccountTab_user
        ...DashboardLayout_PreferenceModal_BillingTab_user
        ...DashboardLayout_PreferenceModal_IdentityTab_user
        ...DashboardLayout_PreferenceModal_SiteTab_user
        ...DashboardLayout_PreferenceModal_ShortcutsTab_user
        ...DashboardLayout_PreferenceModal_EditorTab_user
        ...DashboardLayout_PreferenceModal_LaboratoryTab_user
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
    {
      path: '/preference/editor',
      label: '에디터',
      icon: PencilIcon,
      component: EditorTab,
    },
    ...($user.subscription
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
    {
      path: '/preference/laboratory',
      label: '실험실',
      icon: FlaskConicalIcon,
      component: LaboratoryTab,
    },
    {
      path: '/preference/shortcuts',
      label: '단축키',
      icon: KeyboardIcon,
      component: ShortcutsTab,
    },
  ] satisfies Tab[];

  const currentTab = $derived(tabs.find((tab) => tab.path === page.state.shallowRoute));
</script>

<Modal
  style={css.raw({ maxWidth: '900px', height: 'full', maxHeight: '600px', padding: '0' })}
  onclose={() => history.back()}
  open={!!currentTab}
>
  <div class={flex({ height: 'full' })}>
    <div
      class={css({
        flex: 'none',
        paddingY: '24px',
        paddingX: '12px',
        width: '200px',
        borderRightWidth: '1px',
        borderColor: 'border.subtle',
      })}
    >
      <nav class={flex({ direction: 'column', gap: '1px' })}>
        {#each tabs as { icon, path, label } (path)}
          <button
            class={flex({
              align: 'center',
              gap: '8px',
              borderRadius: '6px',
              paddingX: '10px',
              paddingY: '8px',
              fontSize: '13px',
              color: 'text.muted',
              transition: 'common',
              _hover: { backgroundColor: 'surface.subtle' },
              _selected: {
                color: 'text.default',
                fontWeight: 'medium',
                backgroundColor: 'surface.muted',
              },
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

    <div class={css({ paddingX: '40px', paddingY: '32px', width: 'full', overflowY: 'auto' })}>
      {#if currentTab}
        {@const Component = currentTab.component}

        <Component {$user} />
      {/if}
    </div>
  </div>
</Modal>
