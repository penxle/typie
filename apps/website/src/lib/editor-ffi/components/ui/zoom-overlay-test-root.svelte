<script lang="ts">
  import ZoomOverlay from './ZoomOverlay.svelte';
  import type { Component } from 'svelte';
  import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';

  type Props = {
    initialEnabled?: boolean;
    initialZoom?: number;
    initialIndicatorZoom?: number;
    initialLandmark?: DocumentZoomLandmark | null;
  };

  type TestableProps = {
    enabled: boolean;
    displayZoom: number;
    indicatorZoom: number;
    landmark: DocumentZoomLandmark | null;
    atMinimum: boolean;
    atMaximum: boolean;
    toggleTargetLandmark: DocumentZoomLandmark | null;
    boundaryAttemptRequest: number;
    boundaryAttemptLandmark: DocumentZoomLandmark | null;
    scrollContainer: HTMLElement;
    editorViewSurface: HTMLElement;
    onZoomOut: () => void;
    onZoomIn: () => void;
    onToggleZoom: () => Promise<unknown>;
  };

  const TestableZoomOverlay = ZoomOverlay as unknown as Component<TestableProps>;

  let { initialEnabled = true, initialZoom = 1.25, initialIndicatorZoom = initialZoom, initialLandmark = null }: Props = $props();
  let enabled = $state(initialEnabled);
  let displayZoom = $state(initialZoom);
  let indicatorZoom = $state(initialIndicatorZoom);
  let landmark = $state<DocumentZoomLandmark | null>(initialLandmark);
  let boundaryAttemptRequest = $state(0);
  let boundaryAttemptLandmark = $state<DocumentZoomLandmark | null>(null);
  let editorViewSurface = $state<HTMLElement>();
  let scrollContainer = $state<HTMLElement>();

  const toggleTargetLandmark = $derived<DocumentZoomLandmark | null>(landmark === 'unit' ? 'fit-width' : 'unit');
  const noOp: () => void = () => null;

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
</script>

<div data-pane-id="zoom-overlay-test-pane">
  <div bind:this={editorViewSurface} style="position: relative; width: 300px">
    <div bind:this={scrollContainer}></div>
    {#if editorViewSurface && scrollContainer}
      <TestableZoomOverlay
        atMaximum={landmark === 'maximum'}
        atMinimum={landmark === 'minimum'}
        {boundaryAttemptLandmark}
        {boundaryAttemptRequest}
        {displayZoom}
        {editorViewSurface}
        {enabled}
        {indicatorZoom}
        {landmark}
        onToggleZoom={async () => null}
        onZoomIn={noOp}
        onZoomOut={noOp}
        {scrollContainer}
        {toggleTargetLandmark}
      />
    {/if}
  </div>
</div>
