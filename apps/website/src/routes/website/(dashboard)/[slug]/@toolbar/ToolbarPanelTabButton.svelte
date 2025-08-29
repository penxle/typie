<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import PlanUpgradeModal from '../../PlanUpgradeModal.svelte';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { AppPreference } from '@typie/ui/context';
  import type { Component } from 'svelte';

  type Props = {
    tab: AppPreference['panelTab'];
    label: string;
    icon: Component;
    keys?: TooltipParameter['keys'];
    needPlanUpgrade?: boolean;
  };

  let { tab, label, icon, keys, needPlanUpgrade }: Props = $props();

  let planUpgradeModalOpen = $state(false);

  const app = getAppContext();

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
</script>

<button
  class={center({
    flexDirection: 'column',
    gap: '4px',
    flexShrink: '0',
    borderRadius: '4px',
    width: '40px',
    minHeight: '24px',
    color: 'text.faint',
    transition: 'common',
    _hover: { backgroundColor: 'surface.subtle' },
    _expanded: { backgroundColor: 'surface.muted!', color: 'text.default' },
  })}
  aria-expanded={app.preference.current.panelExpanded && app.preference.current.panelTab === tab}
  onclick={() => {
    if (needPlanUpgrade) {
      planUpgradeModalOpen = true;
      return;
    }

    if (app.preference.current.panelExpanded) {
      if (app.preference.current.panelTab === tab) {
        app.preference.current.panelExpanded = false;
        mixpanel.track('toggle_panel_expanded', { expanded: false });
      } else {
        app.preference.current.panelTab = tab;
        mixpanel.track('toggle_panel_tab', { tab: app.preference.current.panelTab });
      }
    } else {
      app.preference.current.panelExpanded = true;
      if (app.preference.current.panelTab === tab) {
        mixpanel.track('toggle_panel_expanded', { expanded: true });
      } else {
        app.preference.current.panelTab = tab;
        mixpanel.track('toggle_panel_tab', { tab: app.preference.current.panelTab });
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
    <span class={css({ fontSize: '11px' })}>{label}</span>
  {/if}
</button>

<PlanUpgradeModal bind:open={planUpgradeModalOpen} />
