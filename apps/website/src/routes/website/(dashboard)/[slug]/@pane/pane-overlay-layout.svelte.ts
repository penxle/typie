import { prefersReducedMotion } from '@typie/ui/state';
import { PANE_CHROME_FADE_OUT_MS, PANE_CHROME_SINGLE_TOOLBAR_TOP_INSET } from './zen-mode-pane-chrome.svelte';
import type { ZenModePaneChrome } from './zen-mode-pane-chrome.svelte';

type PaneOverlayLayoutOptions = {
  active: () => boolean;
  chrome: ZenModePaneChrome;
};

export const setupPaneOverlayLayout = ({ active, chrome }: PaneOverlayLayoutOptions) => {
  let motionArmed = $state(false);

  $effect(() => {
    motionArmed = false;
    if (!active()) return;
    const frame = requestAnimationFrame(() => (motionArmed = true));
    return () => cancelAnimationFrame(frame);
  });

  return {
    get contentTopInset(): number {
      return active() ? Math.max(chrome.floatingZoomTopInset, PANE_CHROME_SINGLE_TOOLBAR_TOP_INSET) : 0;
    },
    get headerInset(): number {
      return active() ? chrome.headerInset : 0;
    },
    get topInset(): number {
      return active() ? chrome.floatingZoomInset : 0;
    },
    get visibleAreaTopInset(): number {
      return active() ? chrome.topOcclusion : 0;
    },
    get motionDuration(): number {
      return chrome.phase === 'fading' ? PANE_CHROME_FADE_OUT_MS : 280;
    },
    get positionTransition(): string {
      return !motionArmed || !active() || prefersReducedMotion.current
        ? 'none'
        : 'top var(--editor-pane-overlay-motion-duration, 280ms) var(--editor-pane-overlay-motion-easing, cubic-bezier(0.22, 1, 0.36, 1))';
    },
  };
};
