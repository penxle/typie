<script lang="ts">
  import { CONTEXT_BAR_TRANSIENT_VISIBLE_MS } from './editor-context-bar.svelte';
  import EditorBreadcrumb from './EditorBreadcrumb.svelte';
  import EditorContextBar from './EditorContextBar.svelte';
  import EditorZoomControls from './EditorZoomControls.svelte';
  import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';
  import type { EditorContextBarSegmentRenderProps } from './EditorContextBar.svelte';

  type Props = {
    mode?: 'zoom' | 'context-bar';
    initialEnabled?: boolean;
    initialZoom?: number;
    initialIndicatorZoom?: number;
    initialLandmark?: DocumentZoomLandmark | null;
    initialToggleTargetLandmark?: DocumentZoomLandmark | null;
    onZoomOut?: () => unknown;
    onZoomIn?: () => unknown;
    onToggleZoom?: () => Promise<unknown>;
    surfaceWidth?: number;
    breadcrumbWidth?: number;
    viewControlsWidth?: number;
    viewControlsPaneEntry?: boolean;
    withinPane?: boolean;
  };

  let {
    mode = 'zoom',
    initialEnabled = true,
    initialZoom = 1.25,
    initialIndicatorZoom = initialZoom,
    initialLandmark = null,
    initialToggleTargetLandmark,
    onZoomOut = () => null,
    onZoomIn = () => null,
    onToggleZoom = async () => null,
    surfaceWidth = 300,
    breadcrumbWidth = 120,
    viewControlsWidth = 124,
    viewControlsPaneEntry = true,
    withinPane = true,
  }: Props = $props();
  let enabled = $state(initialEnabled);
  let displayZoom = $state(initialZoom);
  let indicatorZoom = $state(initialIndicatorZoom);
  let landmark = $state<DocumentZoomLandmark | null>(initialLandmark);
  let boundaryAttemptRequest = $state(0);
  let boundaryAttemptLandmark = $state<DocumentZoomLandmark | null>(null);
  let editorViewSurface = $state<HTMLElement>();
  let scrollContainer = $state<HTMLElement>();
  let currentSurfaceWidth = $state(surfaceWidth);
  let currentBreadcrumbWidth = $state(breadcrumbWidth);
  let breadcrumbPathIdentity = $state('folder/document');

  const toggleTargetLandmark = $derived<DocumentZoomLandmark | null>(
    initialToggleTargetLandmark === undefined ? (landmark === 'unit' ? 'fit-width' : 'unit') : initialToggleTargetLandmark,
  );

  export function setZoom(nextDisplayZoom: number, nextIndicatorZoom: number, nextLandmark: DocumentZoomLandmark | null) {
    displayZoom = nextDisplayZoom;
    indicatorZoom = nextIndicatorZoom;
    landmark = nextLandmark;
  }

  export function setEnabled(nextEnabled: boolean) {
    enabled = nextEnabled;
  }

  export function requestBoundaryAttempt(nextLandmark: DocumentZoomLandmark) {
    boundaryAttemptLandmark = nextLandmark;
    boundaryAttemptRequest += 1;
  }

  export function setBreadcrumb(next: { contentWidth?: number; pathIdentity?: string }) {
    if (next.contentWidth !== undefined) currentBreadcrumbWidth = next.contentWidth;
    if (next.pathIdentity !== undefined) breadcrumbPathIdentity = next.pathIdentity;
  }

  export function setSurfaceWidth(nextWidth: number) {
    currentSurfaceWidth = nextWidth;
  }
</script>

<svelte:element this={withinPane ? 'div' : 'section'} data-pane-id={withinPane ? 'zoom-overlay-test-pane' : undefined}>
  <div data-testid="editor-toolbar"></div>
  <div bind:this={editorViewSurface} style:width={`${currentSurfaceWidth}px`} style="position: relative; height: 120px">
    <button data-testid="context-bar-underlay" type="button">본문</button>
    <div bind:this={scrollContainer} data-testid="editor-pane"></div>
    {#if mode === 'zoom' && editorViewSurface && scrollContainer}
      <EditorContextBar {editorViewSurface} interactiveViewControlsWhenHidden showViewControlsOnPaneEntry={enabled && landmark !== 'unit'}>
        {#snippet viewControls({ state, presentation }: EditorContextBarSegmentRenderProps)}
          <EditorZoomControls
            atMaximum={landmark === 'maximum'}
            atMinimum={landmark === 'minimum'}
            {boundaryAttemptLandmark}
            {boundaryAttemptRequest}
            {displayZoom}
            {enabled}
            {indicatorZoom}
            {landmark}
            {onToggleZoom}
            {onZoomIn}
            {onZoomOut}
            segment={state}
            {toggleTargetLandmark}
            visible={presentation.visible}
          />
        {/snippet}
      </EditorContextBar>
    {:else if mode === 'context-bar' && editorViewSurface}
      <EditorContextBar {editorViewSurface} showViewControlsOnPaneEntry={viewControlsPaneEntry}>
        {#snippet breadcrumb({ state }: EditorContextBarSegmentRenderProps)}
          <EditorBreadcrumb
            onPathChange={() => state.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS)}
            pathIdentity={breadcrumbPathIdentity}
            viewportId="editor-breadcrumb-test-viewport"
          >
            <div style:width={`${currentBreadcrumbWidth}px`} data-testid="breadcrumb-content">
              <button data-testid="show-breadcrumb" onclick={() => state.showTemporarily(1000)} type="button">경로</button>
              <button data-testid="hold-breadcrumb" onclick={() => state.hold('test')} type="button">유지</button>
              <button data-testid="release-breadcrumb" onclick={() => state.release('test')} type="button">해제</button>
            </div>
          </EditorBreadcrumb>
        {/snippet}
        {#snippet viewControls({ state }: EditorContextBarSegmentRenderProps)}
          <div style:width={`${viewControlsWidth}px`} data-testid="view-controls-content">
            <button data-testid="show-view-controls" onclick={() => state.showTemporarily(1000)} type="button">보기</button>
            <button data-testid="hold-view-controls" onclick={() => state.hold('test')} type="button">유지</button>
            <button data-testid="release-view-controls" onclick={() => state.release('test')} type="button">해제</button>
          </div>
        {/snippet}
      </EditorContextBar>
    {/if}
  </div>
</svelte:element>
