<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { getAppContext } from '@typie/ui/context';
  import { runEscapeStack } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { IS_MAC } from '$lib/editor-ffi/constants';
  import { graphql } from '$mearie';
  import { getPaneGroup } from './[slug]/@pane/context.svelte';
  import type { DashboardLayout_Shortcuts_query$key } from '$mearie';

  type Props = {
    query$key: DashboardLayout_Shortcuts_query$key;
  };

  let { query$key }: Props = $props();

  const app = getAppContext();
  const paneGroup = getPaneGroup();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = createFragment(
    graphql(`
      fragment DashboardLayout_Shortcuts_query on Query {
        me @required {
          id
        }
      }
    `),
    () => query$key,
  );

  const handleKeydown = async (event: KeyboardEvent) => {
    if (app.state.prismAccess && (IS_MAC ? event.metaKey : event.ctrlKey) && !event.shiftKey && event.code === 'KeyE') {
      event.preventDefault();
      const next = !app.preference.current.prismPanelOpen;
      app.preference.current.prismPanelOpen = next;
      mixpanel.track(next ? 'open_prism_panel' : 'close_prism_panel', { via: 'shortcut' });
      return;
    }

    if ((IS_MAC ? event.metaKey : event.ctrlKey) && event.shiftKey && event.code === 'KeyM') {
      if (!app.state.current) return;

      event.preventDefault();

      app.preference.current.zenModeEnabled = !app.preference.current.zenModeEnabled;

      if (app.preference.current.zenModeEnabled) {
        mixpanel.track('zen_mode_enabled', { via: 'shortcut' });
      } else {
        mixpanel.track('zen_mode_disabled', { via: 'shortcut' });
      }

      return;
    }

    if ((IS_MAC ? event.metaKey : event.ctrlKey) && event.code === 'KeyF') {
      if (!app.state.current) return;

      event.preventDefault();
      if (paneGroup.state.current.focusedPaneId) {
        paneGroup.findReplaceOpenByPaneId[paneGroup.state.current.focusedPaneId] = true;
      }

      return;
    }

    if (event.code === 'Escape') {
      if (event.isComposing || event.defaultPrevented) return;

      if (runEscapeStack()) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }

      if (paneGroup.state.current.focusedPaneId && paneGroup.findReplaceOpenByPaneId[paneGroup.state.current.focusedPaneId]) {
        return;
      }

      if (app.preference.current.zenModeEnabled) {
        event.preventDefault();

        app.preference.current.zenModeEnabled = false;
        mixpanel.track('zen_mode_disabled', { via: 'esc' });

        return;
      }
    }
  };
</script>

<svelte:window onkeydown={handleKeydown} />
