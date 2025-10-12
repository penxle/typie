<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import CreditCardIcon from '~icons/lucide/credit-card';
  import FlaskConicalIcon from '~icons/lucide/flask-conical';
  import GemIcon from '~icons/lucide/gem';
  import GiftIcon from '~icons/lucide/gift';
  import KeyboardIcon from '~icons/lucide/keyboard';
  import LayoutIcon from '~icons/lucide/layout';
  import PencilIcon from '~icons/lucide/pencil';
  import ShieldIcon from '~icons/lucide/shield';
  import TypeIcon from '~icons/lucide/type';
  import UserIcon from '~icons/lucide/user';
  import { replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import BillingTab from './BillingTab.svelte';
  import EditorTab from './EditorTab.svelte';
  import FontTab from './FontTab.svelte';
  import InterfaceTab from './InterfaceTab.svelte';
  import LaboratoryTab from './LaboratoryTab.svelte';
  import PlanTab from './PlanTab.svelte';
  import ProfileTab from './ProfileTab.svelte';
  import ReferralTab from './ReferralTab.svelte';
  import SecurityTab from './SecurityTab.svelte';
  import ShortcutsTab from './ShortcutsTab.svelte';
  import type { Component } from 'svelte';
  import type { DashboardLayout_PreferenceModal_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_user;
  };

  type Tab = {
    path: string;
    label: string;
    icon: Component;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    component: Component<any>;
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

        ...DashboardLayout_PreferenceModal_ProfileTab_user
        ...DashboardLayout_PreferenceModal_SecurityTab_user
        ...DashboardLayout_PreferenceModal_EditorTab_user
        ...DashboardLayout_PreferenceModal_InterfaceTab_user
        ...DashboardLayout_PreferenceModal_FontTab_user
        ...DashboardLayout_PreferenceModal_PlanTab_user
        ...DashboardLayout_PreferenceModal_BillingTab_user
        ...DashboardLayout_PreferenceModal_ReferralTab_user
        ...DashboardLayout_PreferenceModal_LaboratoryTab_user
        ...DashboardLayout_PreferenceModal_ShortcutsTab_user
      }
    `),
  );

  type TabGroup = {
    label: string;
    tabs: Tab[];
  };

  const tabGroups: TabGroup[] = [
    {
      label: '계정',
      tabs: [
        {
          path: '/preference/profile',
          label: '프로필',
          icon: UserIcon,
          component: ProfileTab,
        },
        {
          path: '/preference/security',
          label: '보안',
          icon: ShieldIcon,
          component: SecurityTab,
        },
      ],
    },
    {
      label: '환경',
      tabs: [
        {
          path: '/preference/interface',
          label: '인터페이스',
          icon: LayoutIcon,
          component: InterfaceTab,
        },
        {
          path: '/preference/editor',
          label: '에디터',
          icon: PencilIcon,
          component: EditorTab,
        },
        {
          path: '/preference/font',
          label: '폰트',
          icon: TypeIcon,
          component: FontTab,
        },
      ],
    },
    {
      label: '구독',
      tabs: [
        {
          path: '/preference/plan',
          label: '플랜',
          icon: GemIcon,
          component: PlanTab,
        },
        {
          path: '/preference/billing',
          label: '결제',
          icon: CreditCardIcon,
          component: BillingTab,
        },
        {
          path: '/preference/referral',
          label: '초대',
          icon: GiftIcon,
          component: ReferralTab,
        },
      ],
    },
    {
      label: '고급',
      tabs: [
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
      ],
    },
  ];

  const tabs = tabGroups.flatMap((group) => group.tabs);

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
        overflowY: 'auto',
      })}
    >
      <nav class={flex({ direction: 'column', gap: '16px' })}>
        {#each tabGroups as group, groupIndex (group.label)}
          <div>
            {#if groupIndex > 0}
              <div class={css({ height: '1px', backgroundColor: 'border.subtle', marginBottom: '16px' })}></div>
            {/if}

            <div class={css({ paddingX: '10px', paddingY: '4px', marginBottom: '4px' })}>
              <h3
                class={css({
                  fontSize: '11px',
                  fontWeight: 'semibold',
                  color: 'text.disabled',
                  textTransform: 'uppercase',
                  letterSpacing: '[0.05em]',
                })}
              >
                {group.label}
              </h3>
            </div>

            <div class={flex({ direction: 'column', gap: '1px' })}>
              {#each group.tabs as { icon, path, label } (path)}
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
            </div>
          </div>
        {/each}
      </nav>
    </div>

    <div class={css({ paddingX: '40px', paddingY: '32px', width: 'full', overflowY: 'auto' })}>
      {#if currentTab}
        {@const Component = currentTab.component}

        <!-- @ts-expect-error Each tab component accepts a specific fragment type derived from DashboardLayout_PreferenceModal_user -->
        <Component {$user} />
      {/if}
    </div>
  </div>
</Modal>
