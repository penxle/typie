import type { FloatingEditorZoomChromeAttachment } from './ui/FloatingEditorZoomControls.svelte';

export type EditorViewSurfaceLayout = {
  contentInset?: {
    left?: number;
    right?: number;
    top?: number;
  };
  visibleAreaTopInset?: number;
  contentMotion?: { fromX: number; duration: number; easing: string };
  floatingZoom?: {
    rightInset?: number;
    topInset?: number;
    layoutOriginOffset?: number;
    chromeAttachment?: FloatingEditorZoomChromeAttachment;
  };
};
