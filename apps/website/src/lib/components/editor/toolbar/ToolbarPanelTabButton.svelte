<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { getViewContext } from '../../../../routes/website/(dashboard)/[slug]/@split-view/context.svelte';
  import type { TooltipParameter } from '@typie/ui/actions';
  import type { AppPreference } from '@typie/ui/context';
  import type { Component } from 'svelte';

  type Props = {
    tab: AppPreference['panelTabByViewId'][string];
    label: string;
    icon: Component;
    keys?: TooltipParameter['keys'];
    disabled?: boolean;
  };

  let { tab, label, icon, keys, disabled = false }: Props = $props();

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
  {disabled}
  onclick={() => {
    if (disabled) return;

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
