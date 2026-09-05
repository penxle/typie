<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { pointerCapture } from '@typie/ui/actions';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import { untrack } from 'svelte';
  import { PRISM_VISIBILITY_MOTION, reducedMotion } from './lib/motion.ts';
  import { PRISM_PANEL_MAX, PRISM_PANEL_MIN } from './prism-panel.ts';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
  };

  type ResizeSession = {
    startX: number;
    startWidth: number;
  };

  let { children }: Props = $props();

  const app = getAppContext();
  const PANEL_HIDDEN_SCALE = 0.96;
  const PANEL_HIDDEN_SCRIM_OPACITY = 0.9;
  const panelOpen = $derived(app.preference.current.prismPanelOpen);
  const panelInteractive = $derived(panelOpen);
  const panelMotionDuration = reducedMotion() ? 0 : PRISM_VISIBILITY_MOTION.duration;
  let panel = $state<HTMLElement>();
  let previewWidth = $state<number | null>(null);
  const width = $derived(clamp(previewWidth ?? app.preference.current.prismPanelWidth, PRISM_PANEL_MIN, PRISM_PANEL_MAX));

  const updateResize = (session: ResizeSession, event: PointerEvent) => {
    previewWidth = clamp(session.startWidth + Math.round(session.startX - event.clientX), PRISM_PANEL_MIN, PRISM_PANEL_MAX);
  };

  $effect(() => {
    if (panelInteractive) return;

    untrack(() => {
      const focused = document.activeElement;
      if (focused instanceof HTMLElement && panel?.contains(focused)) focused.blur();
    });
  });
</script>

<div
  style:width={panelOpen ? `${width}px` : '0px'}
  style:transition-duration={previewWidth === null ? `${panelMotionDuration}ms` : '0ms'}
  style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
  class={css({ flexShrink: '0', height: 'full', transitionProperty: '[width]' })}
  aria-hidden="true"
  data-prism-panel-spacer
></div>

<div
  style:width={`${width}px`}
  style:--prism-panel-hidden={panelOpen ? '0%' : '100%'}
  style:pointer-events="none"
  style:transition-duration={`${panelMotionDuration}ms`}
  style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
  class={css({
    position: 'absolute',
    top: '0',
    right: '0',
    bottom: '0',
    height: 'full',
    zIndex: 'panel',
    transitionProperty: '[--prism-panel-hidden]',
  })}
  data-prism-panel-shell
  data-zen-mode-closing-surface
>
  <div
    style:clip-path="inset(0 0 0 calc(var(--prism-panel-hidden) + 1px))"
    style:pointer-events={panelInteractive ? 'auto' : 'none'}
    class={css({ position: 'absolute', inset: '0', overflow: 'hidden' })}
    data-prism-panel-reveal
  >
    <aside
      bind:this={panel}
      style:width={`${width}px`}
      style:transform={panelOpen ? 'scale(1)' : `scale(${PANEL_HIDDEN_SCALE})`}
      style:transition-duration={`${panelMotionDuration}ms`}
      style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
      class={flex({
        position: 'absolute',
        top: '0',
        right: '0',
        bottom: '0',
        flexDirection: 'column',
        height: 'full',
        overflow: 'hidden',
        backgroundColor: 'surface.default',
        transformOrigin: 'center',
        transitionProperty: '[transform]',
      })}
      inert={!panelInteractive}
    >
      {@render children()}
    </aside>

    <div
      style:width={`${width}px`}
      style:opacity={panelOpen ? 0 : PANEL_HIDDEN_SCRIM_OPACITY}
      style:transition-duration={`${panelMotionDuration}ms`}
      style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
      class={css({
        position: 'absolute',
        top: '0',
        right: '0',
        bottom: '0',
        zIndex: 'panel',
        pointerEvents: 'none',
        backgroundColor: 'surface.default',
        filter: '[brightness(0.9)]',
        transitionProperty: '[opacity]',
      })}
      aria-hidden="true"
    ></div>
  </div>

  <div
    style:width={`${width}px`}
    style:transform="translateX(var(--prism-panel-hidden))"
    class={css({
      position: 'absolute',
      top: '0',
      bottom: '0',
      left: '0',
      boxSizing: 'border-box',
      borderLeftWidth: '1px',
      borderColor: 'border.subtle',
      zIndex: 'panel',
      pointerEvents: 'none',
    })}
    data-prism-panel-edge
  >
    <div
      style:pointer-events={panelInteractive ? 'auto' : 'none'}
      style:transform="translateX(-50%)"
      class={css({
        position: 'absolute',
        top: '0',
        bottom: '0',
        left: '[-1px]',
        display: 'flex',
        justifyContent: 'center',
        width: '8px',
        cursor: 'col-resize',
        zIndex: '4',
        _hoverAfter: {
          content: '""',
          display: 'block',
          borderRadius: '4px',
          height: 'full',
          width: '2px',
          backgroundColor: 'border.strong',
          opacity: '50',
        },
      })}
      data-prism-panel-resize-handle
      use:pointerCapture={{
        start: (event): ResizeSession | null => {
          if (!event.isPrimary || event.button !== 0) return null;
          event.preventDefault();
          const session = { startX: event.clientX, startWidth: width };
          previewWidth = session.startWidth;
          return session;
        },
        move: updateResize,
        end: (session, event) => {
          updateResize(session, event);
          if (previewWidth !== null) app.preference.current.prismPanelWidth = previewWidth;
          previewWidth = null;
        },
        cancel: () => {
          previewWidth = null;
        },
      }}
    ></div>
  </div>
</div>

<style>
  @property --prism-panel-hidden {
    syntax: '<percentage>';
    inherits: true;
    initial-value: 100%;
  }
</style>
