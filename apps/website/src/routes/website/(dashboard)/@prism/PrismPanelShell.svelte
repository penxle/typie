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
  const panelVisible = $derived(app.state.prismAccess || app.preference.current.prismPanelOpen);
  const panelOpen = $derived(app.preference.current.prismPanelOpen);
  const panelInteractive = $derived(panelOpen && !app.preference.current.zenModeEnabled);
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

{#if panelVisible && !app.preference.current.zenModeEnabled}
  <div
    style:width={panelOpen ? `${width}px` : '0px'}
    style:transition-duration={previewWidth === null ? `${panelMotionDuration}ms` : '0ms'}
    style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
    class={css({ flexShrink: '0', height: 'full', transitionProperty: '[width]' })}
    aria-hidden="true"
    data-prism-panel-spacer
  ></div>
{/if}

<div
  style:width={`${width}px`}
  style:pointer-events={panelInteractive ? 'auto' : 'none'}
  style:transform={panelOpen ? 'scale(1)' : `scale(${PANEL_HIDDEN_SCALE})`}
  style:transition-duration={`${panelMotionDuration}ms`}
  style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
  class={css({
    position: 'absolute',
    top: '0',
    right: '0',
    bottom: '0',
    height: 'full',
    transformOrigin: 'center',
    transitionProperty: '[transform]',
    zIndex: 'panel',
  })}
  data-prism-panel-shell
  hidden={!panelVisible}
>
  <aside
    bind:this={panel}
    style:clip-path={panelOpen ? 'inset(0)' : 'inset(0 0 0 100%)'}
    style:transition-duration={`${panelMotionDuration}ms`}
    style:transition-timing-function={PRISM_VISIBILITY_MOTION.easing}
    class={flex({
      position: 'absolute',
      inset: '0',
      flexDirection: 'column',
      width: 'full',
      height: 'full',
      overflow: 'hidden',
      borderLeftWidth: '1px',
      borderColor: 'border.subtle',
      backgroundColor: 'surface.default',
      transitionProperty: '[clip-path]',
    })}
    inert={!panelInteractive}
  >
    {@render children()}
  </aside>

  <div
    style:transform="translateX(-50%)"
    class={css({
      position: 'absolute',
      top: '0',
      bottom: '0',
      left: '0',
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

{#if panelVisible}
  <div
    style:width={`${width}px`}
    style:clip-path={panelOpen ? 'inset(0)' : 'inset(0 0 0 100%)'}
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
      transitionProperty: '[clip-path, opacity]',
    })}
    aria-hidden="true"
  ></div>
{/if}
