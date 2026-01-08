<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import SettingsIcon from '~icons/lucide/settings';
  import { replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import GeneralTab from './GeneralTab.svelte';
  import type { Component } from 'svelte';
  import type { DashboardLayout_SiteSettingsModal_site, DashboardLayout_SiteSettingsModal_user } from '$graphql';

  type Props = {
    $site: DashboardLayout_SiteSettingsModal_site;
    $user: DashboardLayout_SiteSettingsModal_user;
  };

  type Tab = {
    path: string;
    label: string;
    icon: Component;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    component: Component<any>;
  };

  let { $site: _site, $user: _user }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_site on Site {
        id

        ...DashboardLayout_SiteSettingsModal_GeneralTab_site
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_user on User {
        id

        ...DashboardLayout_SiteSettingsModal_GeneralTab_user
      }
    `),
  );

  const tabs: Tab[] = [
    {
      path: '/site-settings/general',
      label: '일반',
      icon: SettingsIcon,
      component: GeneralTab,
    },
  ];

  const currentTab = $derived(
    tabs.find((tab) => page.state.shallowRoute?.startsWith('/site-settings') && tab.path === page.state.shallowRoute),
  );
  const open = $derived(page.state.shallowRoute?.startsWith('/site-settings') ?? false);

  $effect(() => {
    if (open && !currentTab) {
      replaceState('', { shallowRoute: '/site-settings/general' });
    }
  });
</script>

<Modal style={css.raw({ maxWidth: '900px', height: 'full', maxHeight: '600px', padding: '0' })} onclose={() => history.back()} {open}>
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
        <div>
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
              사이트
            </h3>
          </div>

          <div class={flex({ direction: 'column', gap: '1px' })}>
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
          </div>
        </div>
      </nav>
    </div>

    <div class={css({ paddingX: '40px', paddingY: '32px', width: 'full', overflowY: 'auto' })}>
      {#if currentTab}
        {@const Component = currentTab.component}

        <!-- @ts-expect-error Each tab component accepts a specific fragment type -->
        <Component {$site} {$user} />
      {/if}
    </div>
  </div>
</Modal>
