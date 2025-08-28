<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Component } from 'svelte';

  type Props = {
    tab: 'info' | 'settings';
    label: string;
    icon: Component;
    keys?: TooltipParameter['keys'];
  };

  let { tab, label, icon, keys }: Props = $props();

  const app = getAppContext();
</script>

<button
  class={center({
    borderRadius: '4px',
    width: '40px',
    height: '24px',
    color: 'text.faint',
    transition: 'common',
    _hover: { backgroundColor: 'surface.subtle' },
    _expanded: { backgroundColor: 'surface.muted!' },
  })}
  aria-expanded={app.preference.current.panelExpanded && app.preference.current.panelTab === tab}
  onclick={() => {
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
    message: label,
    keys,
    arrow: false,
    delay: 1000,
  }}
>
  <Icon style={css.raw({ color: 'text.faint' })} {icon} size={20} />
</button>
