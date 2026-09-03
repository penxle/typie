<script lang="ts">
  import { setupAppContext } from '@typie/ui/context';
  import EditorBreadcrumb from '$lib/editor-ffi/components/ui/EditorBreadcrumb.svelte';
  import { setupPane, setupPaneGroup } from './context.svelte';
  import PaneHeader from './PaneHeader.svelte';
  import PaneSkeleton from './PaneSkeleton.svelte';
  import { setupZenModePaneChrome } from './zen-mode-pane-chrome.svelte';
  import type { Pane } from './types';

  type Props = { headerWidth?: number; zenModeEnabled?: boolean };

  let { headerWidth = 420, zenModeEnabled = false }: Props = $props();

  const app = setupAppContext('pane-header-priority-test');
  app.preference.current.sidebarHidden = false;
  app.preference.current.zenModeEnabled = zenModeEnabled;
  app.preference.current.prismPanelOpen = false;

  const pane: Pane = { id: 'pane-header-test', type: 'pane', kind: 'entity', slug: 'pane-header-test' };
  const paneGroup = setupPaneGroup('pane-header-site', {
    userId: 'pane-header-priority-test',
    navigate: () => {
      // No navigation in this test host.
    },
    onSiteChange: () => {
      // The test stays on one site.
    },
  });
  paneGroup.state.current.root = pane;
  paneGroup.state.current.focusedPaneId = pane.id;
  setupPane(pane);
  setupZenModePaneChrome({ active: () => zenModeEnabled, focused: () => true });
</script>

<div style:width={`${headerWidth}px`} style="position: relative; height: 37px" data-pane-header-test-host>
  <PaneHeader placement={{ topLeft: true, topRight: true }}>
    <EditorBreadcrumb pathIdentity="folder/document" viewportId="pane-header-breadcrumb-test">
      <div style="width: 260px">Folder / Document with a long title</div>
    </EditorBreadcrumb>

    {#snippet scrollableActions()}
      {#each { length: 12 }, index}
        <button style="width: 24px; height: 24px; flex-shrink: 0" data-pane-header-scrollable-action={index} type="button">
          {index + 1}
        </button>
      {/each}
    {/snippet}

    {#snippet fixedActions()}
      {#each { length: 3 }, index}
        <button style="width: 24px; height: 24px; flex-shrink: 0" data-pane-header-fixed-action={index} type="button">
          {index + 1}
        </button>
      {/each}
    {/snippet}
  </PaneHeader>
</div>

<div style="width: 180px" data-pane-header-breadcrumb-test-host>
  <PaneHeader placement={{ topLeft: false, topRight: false }}>
    <EditorBreadcrumb pathIdentity="long-breadcrumb" viewportId="pane-header-breadcrumb-only-test">
      <div style="width: 260px">Folder / Document with a long title</div>
    </EditorBreadcrumb>
  </PaneHeader>
</div>

<div style="width: 420px; height: 240px" data-pane-header-skeleton-handoff-test-host>
  <PaneHeader placement={{ topLeft: true, topRight: true }}>
    <span>Loaded document</span>
  </PaneHeader>
  <PaneSkeleton contentInsetTop={78} headerPlacement={{ topLeft: true, topRight: true }} {pane} showHeader={false} />
</div>

<output data-sidebar-peek>{String(app.state.sidebarPeek)}</output>
<output data-sidebar-hidden>{String(app.preference.current.sidebarHidden)}</output>
