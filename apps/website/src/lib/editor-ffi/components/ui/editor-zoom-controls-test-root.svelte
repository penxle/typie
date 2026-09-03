<script lang="ts">
  import FloatingEditorZoomControls from './FloatingEditorZoomControls.svelte';
  import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';

  type Props = {
    initialZoom?: number;
    initialIndicatorZoom?: number;
    initialLandmark?: DocumentZoomLandmark | null;
    onZoomOut?: () => unknown;
    onToggleZoom?: () => unknown | Promise<unknown>;
    fixed?: boolean;
    revealOnHover?: boolean;
    requiresChrome?: boolean;
    topInset?: number;
  };

  let {
    initialZoom = 1.25,
    initialIndicatorZoom = initialZoom,
    initialLandmark = null,
    onZoomOut = () => null,
    onToggleZoom = () => null,
    fixed = true,
    revealOnHover = false,
    requiresChrome = false,
    topInset = 0,
  }: Props = $props();
  let displayZoom = $state(initialZoom);
  let indicatorZoom = $state(initialIndicatorZoom);
  let landmark = $state<DocumentZoomLandmark | null>(initialLandmark);
  let boundaryAttemptRequest = $state(0);
  let boundaryAttemptLandmark = $state<DocumentZoomLandmark | null>(null);
  let chromeReady = $state(!requiresChrome);
  let chromeAttached = $state(!requiresChrome);
  let pointer: { x: number; y: number } | null = null;
  const chromeAttachment = {
    hold: () => null,
    release: () => null,
    discoverable: () => chromeReady,
    attached: () => chromeAttached,
    pointer: () => pointer,
  };

  const toggleTargetLandmark = $derived<DocumentZoomLandmark>(landmark === 'unit' ? 'fit-width' : 'unit');

  export function setZoom(nextDisplayZoom: number, nextIndicatorZoom: number, nextLandmark: DocumentZoomLandmark | null) {
    displayZoom = nextDisplayZoom;
    indicatorZoom = nextIndicatorZoom;
    landmark = nextLandmark;
  }

  export function requestBoundaryAttempt(nextLandmark: DocumentZoomLandmark) {
    boundaryAttemptLandmark = nextLandmark;
    boundaryAttemptRequest += 1;
  }

  export function setChromeReady(ready: boolean) {
    chromeReady = ready;
    chromeAttached = ready;
  }
</script>

<svelte:window onpointermove={(event) => (pointer = { x: event.clientX, y: event.clientY })} />

<div data-pane-id="zoom-controls-test-pane">
  <div style:--editor-floating-zoom-top-inset={`${topInset}px`} style="position: relative; width: 300px; height: 120px">
    <FloatingEditorZoomControls
      chromeAttachment={requiresChrome ? chromeAttachment : undefined}
      controls={{
        atMaximum: landmark === 'maximum',
        atMinimum: landmark === 'minimum',
        boundaryAttemptLandmark,
        boundaryAttemptRequest,
        displayZoom,
        enabled: true,
        indicatorZoom,
        landmark,
        onToggleZoom,
        onZoomIn: () => null,
        onZoomOut,
        toggleTargetLandmark,
      }}
      {fixed}
      {revealOnHover}
      {topInset}
    />
  </div>
</div>
