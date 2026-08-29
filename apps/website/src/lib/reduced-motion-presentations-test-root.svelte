<script lang="ts">
  import { Scrollbar } from '@typie/ui/components';
  import { setupAppContext } from '@typie/ui/context';
  import { setupPaneGroup } from '../routes/website/(dashboard)/[slug]/@pane/context.svelte';
  import PaneSkeleton from '../routes/website/(dashboard)/[slug]/@pane/PaneSkeleton.svelte';
  import EditorScrollbar from './editor-ffi/components/Scrollbar.svelte';
  import { setupEditorContext } from './editor-ffi/editor.svelte';
  import type { Pane } from '../routes/website/(dashboard)/[slug]/@pane/types';
  import type { Editor } from './editor-ffi/editor.svelte';
  import type { EditorScrollScope } from './editor-ffi/scroll.svelte';

  const app = setupAppContext('reduced-motion-presentations-test');
  app.preference.current.defaultPrimaryToolbar = 'insert';

  const paneGroup = setupPaneGroup('reduced-motion-presentations-test', {
    userId: 'reduced-motion-presentations-test',
    navigate: () => null,
    onSiteChange: () => null,
  });
  const entityPane: Pane = { id: 'entity-pane', type: 'pane', kind: 'entity', slug: 'document' };
  const homePane: Pane = { id: 'home-pane', type: 'pane', kind: 'home' };
  paneGroup.state.current.toolbarExpandedByPaneId[entityPane.id] = false;

  const editorContext = setupEditorContext();
  let sharedScrollContainer = $state<HTMLDivElement>();
  let editorScrollContainer = $state<HTMLDivElement>();

  $effect(() => {
    if (!editorScrollContainer) return;
    editorContext.editor = {
      scrollContainerEl: editorScrollContainer,
      rootAttrs: { layout_mode: { type: 'continuous', max_width: 600 } },
      pageSizes: [],
      pageEls: {},
      presentationGeometryRevision: 0,
      safeDisplayZoom: () => 1,
    } as unknown as Editor;
    editorContext.scroll = {
      cancel: () => null,
      lastScrollRevision: 1,
      lastScrollWasAuto: false,
    } as unknown as EditorScrollScope;
  });
</script>

<div style="position: relative; width: 120px; height: 160px" data-testid="shared-scrollbar">
  <div
    bind:this={sharedScrollContainer}
    id="reduced-motion-shared-scroll"
    style="width: 100%; height: 100%; overflow-y: auto; scrollbar-width: none"
  >
    <div style="height: 480px"></div>
  </div>
  <Scrollbar
    controls="reduced-motion-shared-scroll"
    label="공유 스크롤바"
    orientation="vertical"
    scrollContainer={sharedScrollContainer}
    size="md"
  />
</div>

<div style="position: relative; width: 120px; height: 160px" data-testid="editor-scrollbar">
  <div bind:this={editorScrollContainer} style="width: 100%; height: 100%; overflow-y: auto; scrollbar-width: none">
    <div style="height: 480px"></div>
  </div>
  <EditorScrollbar />
</div>

<div style="width: 800px; height: 800px" data-testid="entity-skeleton">
  <PaneSkeleton pane={entityPane} />
</div>

<div style="width: 800px; height: 800px" data-testid="home-skeleton">
  <PaneSkeleton pane={homePane} />
</div>
