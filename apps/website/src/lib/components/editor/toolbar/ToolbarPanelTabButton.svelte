<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { getPane, getPaneGroup } from '../../../../routes/website/(dashboard)/[slug]/@pane/context.svelte';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Component } from 'svelte';
  import type { PanelTab } from '../../../../routes/website/(dashboard)/[slug]/@pane/context.svelte';

  type Props = {
    tab: PanelTab;
    label: string;
    icon: Component;
    keys?: TooltipParameter['keys'];
  };

  let { tab, label, icon, keys }: Props = $props();

  const app = getAppContext();

  const paneId = getPane().id;
  const paneGroup = getPaneGroup();

  const isExpanded = $derived(paneGroup.state.current.panelExpandedByPaneId[paneId]);
  const isTab = $derived(paneGroup.state.current.panelTabByPaneId[paneId] === tab);

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
    if (isExpanded) {
      if (isTab) {
        paneGroup.state.current.panelExpandedByPaneId = {
          ...paneGroup.state.current.panelExpandedByPaneId,
          [paneId]: false,
        };
        mixpanel.track('toggle_panel_expanded', { expanded: false });
      } else {
        paneGroup.state.current.panelTabByPaneId = {
          ...paneGroup.state.current.panelTabByPaneId,
          [paneId]: tab,
        };
        mixpanel.track('toggle_panel_tab', { tab });
      }
    } else {
      paneGroup.state.current.panelExpandedByPaneId = {
        ...paneGroup.state.current.panelExpandedByPaneId,
        [paneId]: true,
      };
      if (isTab) {
        mixpanel.track('toggle_panel_expanded', { expanded: true });
      } else {
        paneGroup.state.current.panelTabByPaneId = {
          ...paneGroup.state.current.panelTabByPaneId,
          [paneId]: tab,
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
