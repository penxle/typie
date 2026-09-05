<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { getPane, getPaneGroup } from '../../@pane/context.svelte';
  import { getDocumentPanelFocusReturn } from './focus-return.svelte';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { Component } from 'svelte';
  import type { PanelTab } from '../../@pane/context.svelte';

  type Props = {
    tab: PanelTab;
    label: string;
    icon: Component;
    keys?: TooltipParameter['keys'];
  };

  let { tab, label, icon, keys }: Props = $props();

  const paneId = getPane().id;
  const paneGroup = getPaneGroup();
  const focusReturn = getDocumentPanelFocusReturn();

  const isExpanded = $derived(paneGroup.state.current.panelExpandedByPaneId[paneId]);
  const isTab = $derived(paneGroup.state.current.panelTabByPaneId[paneId] === tab);
</script>

<button
  class={center({
    size: '24px',
    flexShrink: '0',
    borderRadius: '4px',
    color: 'text.muted',
    transition: 'common',
    _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
    _expanded: { color: 'text.default!', backgroundColor: 'surface.active', _hover: { backgroundColor: 'surface.active' } },
  })}
  aria-expanded={isExpanded && isTab}
  onclick={() => {
    if (isExpanded) {
      if (isTab) {
        paneGroup.state.current.panelExpandedByPaneId = {
          ...paneGroup.state.current.panelExpandedByPaneId,
          [paneId]: false,
        };
        focusReturn.restore();
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
  onpointerdown={(event) => event.preventDefault()}
  type="button"
  use:tooltip={{ message: label, keys }}
>
  <Icon {icon} size={16} />
</button>
