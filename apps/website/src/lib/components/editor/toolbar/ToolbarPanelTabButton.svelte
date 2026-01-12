<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import { getViewContext } from '../../../../routes/website/(dashboard)/[slug]/@split-view/context.svelte';
  import PlanUpgradeModal from '../../../../routes/website/(dashboard)/PlanUpgradeModal.svelte';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { AppPreference } from '@typie/ui/context';
  import type { Component } from 'svelte';
  import type { DocumentEditor_TopToolbar_PanelTabButton_user } from '$graphql';

  type Props = {
    tab: AppPreference['panelTabByViewId'][string];
    label: string;
    icon: Component;
    keys?: TooltipParameter['keys'];
    $user: DocumentEditor_TopToolbar_PanelTabButton_user;
    needSubscription?: boolean;
  };

  let { tab, label, icon, keys, $user: _user, needSubscription }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DocumentEditor_TopToolbar_PanelTabButton_user on User {
        id
        ...DashboardLayout_PlanUpgradeModal_user

        subscription {
          id
        }
      }
    `),
  );

  const needPlanUpgrade = $derived(needSubscription && !$user.subscription);

  let planUpgradeModalOpen = $state(false);

  const app = getAppContext();

  const splitViewId = getViewContext().id;

  const isExpanded = $derived(app.preference.current.panelExpandedByViewId[splitViewId]);
  const isTab = $derived(app.preference.current.panelTabByViewId[splitViewId] === tab);

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
</script>

<button
  class={center({
    flexDirection: 'column',
    gap: '4px',
    flexShrink: '0',
    borderRadius: '4px',
    width: toolbarSize === 'large' ? '48px' : '40px',
    minHeight: '24px',
    color: 'text.faint',
    transition: 'common',
    _hover: { backgroundColor: 'surface.subtle' },
    _expanded: { backgroundColor: 'surface.muted!', color: 'text.default' },
    _disabled: { opacity: '50' },
  })}
  aria-expanded={isExpanded && isTab}
  onclick={() => {
    if (needPlanUpgrade) {
      planUpgradeModalOpen = true;
      mixpanel.track('open_plan_upgrade_modal', { via: 'panel_tab_button', tab });
      return;
    }

    if (isExpanded) {
      if (isTab) {
        app.preference.current.panelExpandedByViewId = {
          ...app.preference.current.panelExpandedByViewId,
          [splitViewId]: false,
        };
        mixpanel.track('toggle_panel_expanded', { expanded: false });
      } else {
        app.preference.current.panelTabByViewId = {
          ...app.preference.current.panelTabByViewId,
          [splitViewId]: tab,
        };
        mixpanel.track('toggle_panel_tab', { tab });
      }
    } else {
      app.preference.current.panelExpandedByViewId = {
        ...app.preference.current.panelExpandedByViewId,
        [splitViewId]: true,
      };
      if (isTab) {
        mixpanel.track('toggle_panel_expanded', { expanded: true });
      } else {
        app.preference.current.panelTabByViewId = {
          ...app.preference.current.panelTabByViewId,
          [splitViewId]: tab,
        };
        mixpanel.track('toggle_panel_tab', { tab });
      }
    }
  }}
  type="button"
  use:tooltip={{
    message: toolbarSize === 'medium' ? label : undefined,
    keys: toolbarSize === 'medium' ? keys : undefined,
    arrow: false,
    delay: 1000,
  }}
>
  <Icon style={css.raw({ color: 'text.faint' })} {icon} size={20} />

  {#if toolbarSize === 'large'}
    <span class={css({ fontSize: '11px', whiteSpace: 'nowrap' })}>{label}</span>
  {/if}
</button>

{#if $user}
  <PlanUpgradeModal {$user} bind:open={planUpgradeModalOpen}>{label} 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.</PlanUpgradeModal>
{/if}
