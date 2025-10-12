<script lang="ts">
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import { getSplitViewContext } from './[slug]/@split-view/context.svelte';
  import type { DashboardLayout_Shortcuts_query } from '$graphql';

  type Props = {
    $query: DashboardLayout_Shortcuts_query;
  };

  let { $query: _query }: Props = $props();

  const app = getAppContext();
  const splitView = getSplitViewContext();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = fragment(
    _query,
    graphql(`
      fragment DashboardLayout_Shortcuts_query on Query {
        me @required {
          id
        }
      }
    `),
  );

  const handleKeydown = async (event: KeyboardEvent) => {
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.code === 'KeyM') {
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

    if ((event.ctrlKey || event.metaKey) && event.code === 'KeyF') {
      if (!app.state.current) return;

      event.preventDefault();
      if (splitView.state.current.focusedViewId) {
        app.state.findReplaceOpenByViewId[splitView.state.current.focusedViewId] = true;
      }

      return;
    }

    if (event.code === 'Escape') {
      if (splitView.state.current.focusedViewId && app.state.findReplaceOpenByViewId[splitView.state.current.focusedViewId]) {
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
