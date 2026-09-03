<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { smootherstep } from '@typie/ui/utils';
  import {
    getZenModePaneChrome,
    PANE_CHROME_BOUNDARY,
    PANE_CHROME_EXPANSION_EASING,
    PANE_CHROME_FADE_OUT_MS,
    PANE_CHROME_FADE_WIDTH,
    PANE_CHROME_FOREGROUND_FADE_IN_MS,
    PANE_CHROME_SPOT_MASK_INSET,
    PANE_CHROME_SPOT_RADIUS,
    paneChromeExpansionTiming,
    paneChromeRadialMask,
  } from './zen-mode-pane-chrome.svelte';
  import type { PaneChromeLane, PaneChromeSegmentGeometry, PaneChromeTone } from './zen-mode-pane-chrome.svelte';

  type Props = { lane: PaneChromeLane; toolbarSeparatorOffsets?: number[] };
  type HeaderSegment = {
    id: 'identity' | 'actions';
    side: 'leading' | 'trailing';
    geometry: PaneChromeSegmentGeometry | undefined;
  };

  const SPOT_ENTER_MS = 500;
  const ENGAGED_SPOT_RADIUS = 52;
  const ENGAGED_EDGE_SPOT_RADIUS = 80;
  const ENGAGED_SPOT_STRENGTH = 0.85;
  const ENGAGED_SEGMENT_STRENGTH = 0.5;
  const ENGAGED_SPOT_ENTER_DELAY_MS = 500;
  const MUTED_SURFACE = token('colors.surface.muted');
  const CONTINUOUS_SURFACE = token('colors.surface.default');
  const PAGINATED_SURFACE = token('colors.surface.subtle');
  const TRANSIENT_BASE = `color-mix(in srgb, ${CONTINUOUS_SURFACE} 50%, ${PAGINATED_SURFACE} 50%)`;
  const ACTIVE_SURFACE = `color-mix(in srgb, ${MUTED_SURFACE} 80%, ${TRANSIENT_BASE} 20%)`;
  const TRANSIENT_SURFACE = `color-mix(in srgb, ${TRANSIENT_BASE} 75%, transparent)`;
  const ENGAGED_SEGMENT_SURFACE = `color-mix(in srgb, ${ACTIVE_SURFACE} ${ENGAGED_SEGMENT_STRENGTH * 100}%, transparent)`;
  const TRANSIENT_EDGE = `color-mix(in srgb, ${TRANSIENT_BASE} 40%, ${token('colors.border.subtle')} 60%)`;
  const ENGAGED_EDGE_BASE = `color-mix(in srgb, ${ACTIVE_SURFACE} 36%, ${token('colors.border.default')} 64%)`;
  const ENGAGED_EDGE = `color-mix(in srgb, ${ENGAGED_EDGE_BASE} ${ENGAGED_SEGMENT_STRENGTH * 100}%, transparent)`;

  let { lane, toolbarSeparatorOffsets = [] }: Props = $props();
  const chrome = getZenModePaneChrome();
  const expansionTiming = $derived(paneChromeExpansionTiming(chrome.phase === 'expanding' ? chrome.expansionPace : 'standard'));
  const laneSize = $derived(chrome.laneSize(lane));
  const pointer = $derived(chrome.pointerInLane(lane));
  const laneSpot = $derived(chrome.phase === 'expanding' ? chrome.expansionOriginInLane(lane) : chrome.spotInLane(lane));
  const headerSegments = $derived<HeaderSegment[]>([
    { id: 'identity', side: 'leading', geometry: chrome.segmentGeometry('identity') },
    { id: 'actions', side: 'trailing', geometry: chrome.segmentGeometry('actions') },
  ]);
  const headerUnified = $derived(chrome.shown.identity && chrome.shown.actions);
  const expansionApplies = $derived(
    lane === 'toolbar' ? chrome.expansionTargets.toolbar : chrome.expansionTargets.identity || chrome.expansionTargets.actions,
  );
  const spotVisible = $derived(laneSpot !== null && (chrome.phase === 'spot' || (chrome.phase === 'expanding' && expansionApplies)));
  const spotRadius = $derived(
    chrome.phase === 'expanding'
      ? Math.hypot(laneSize.width, laneSize.height) + PANE_CHROME_FADE_WIDTH + PANE_CHROME_SPOT_MASK_INSET
      : PANE_CHROME_SPOT_RADIUS,
  );
  const spotSurfaceStrength = $derived(chrome.phase === 'expanding' ? 1 : spotVisible ? 0.7 : 0);
  const headerVisible = $derived(
    chrome.phase !== 'fading' && (spotVisible || chrome.isSurfaceVisible('identity') || chrome.isSurfaceVisible('actions')),
  );
  const toolbarVisible = $derived(chrome.phase !== 'fading' && (spotVisible || chrome.isSurfaceVisible('toolbar')));
  const toolbarSurfaceMask = $derived(
    spotVisible ? spotMask('--zen-pane-chrome-spot-surface-opacity') : chrome.shown.toolbar ? 'linear-gradient(black, black)' : undefined,
  );
  const toolbarBlurMask = $derived(
    spotVisible ? spotMask('--zen-pane-chrome-spot-opacity') : chrome.shown.toolbar ? 'linear-gradient(black, black)' : undefined,
  );
  const headerSurfaceMasks = $derived.by(() => resolveHeaderMasks(true, '--zen-pane-chrome-spot-surface-opacity'));
  const headerBlurMasks = $derived.by(() => resolveHeaderMasks(true, '--zen-pane-chrome-spot-opacity'));
  const headerSurfaceMask = $derived(headerSurfaceMasks.join(', '));
  const headerBlurMask = $derived(headerBlurMasks.join(', '));
  const hasEngagedHeaderSegment = $derived(
    (chrome.shown.identity && chrome.tone.identity === 'engaged') || (chrome.shown.actions && chrome.tone.actions === 'engaged'),
  );
  const laneShown = $derived(lane === 'header' ? chrome.shown.identity || chrome.shown.actions : chrome.shown.toolbar);
  const laneVisible = $derived(lane === 'header' ? headerVisible : toolbarVisible);
  const laneSurfaceMask = $derived(lane === 'header' ? headerSurfaceMask : toolbarSurfaceMask);
  const laneBlurMask = $derived(lane === 'header' ? headerBlurMask : toolbarBlurMask);
  const laneEngagedSpotVisible = $derived(
    pointer !== null && (spotVisible || chrome.phase === 'held' || (lane === 'header' && hasEngagedHeaderSegment)),
  );
  const laneEngagedSpotTransition = $derived(
    pointer === null
      ? `opacity ${PANE_CHROME_FADE_OUT_MS}ms ease-out`
      : `opacity ${SPOT_ENTER_MS}ms ease-out ${chrome.phase === 'held' ? 0 : ENGAGED_SPOT_ENTER_DELAY_MS}ms`,
  );
  const laneSurfaceTransition = $derived(
    `opacity ${laneVisible ? 0 : PANE_CHROME_FADE_OUT_MS}ms ease-out${
      lane === 'header' ? `, mask-image ${chrome.phase === 'expanding' ? expansionTiming.spotSurfaceMs : 0}ms ease-out` : ''
    }`,
  );
  let engagedSpotX = $state(0);
  let engagedSpotY = $state(0);
  const toolbarSeparators = $derived.by(() =>
    toolbarSeparatorOffsets
      .filter((offset) => Number.isFinite(offset) && offset > 0 && offset < laneSize.height)
      .toSorted((left, right) => left - right),
  );
  const toolbarEngagedRowIndex = $derived.by(() => {
    let rowIndex = 0;
    for (const offset of toolbarSeparators) {
      if (engagedSpotY >= offset) rowIndex += 1;
    }
    return rowIndex;
  });
  const toolbarEngagedSurfaceClip = $derived.by(() => {
    if (toolbarSeparators.length === 0) return;
    const top = toolbarEngagedRowIndex === 0 ? 0 : toolbarSeparators[toolbarEngagedRowIndex - 1];
    const bottom = toolbarSeparators[toolbarEngagedRowIndex] ?? laneSize.height;
    return `inset(${top}px 0 ${Math.max(0, laneSize.height - bottom)}px 0)`;
  });

  $effect(() => {
    if (!pointer) return;
    engagedSpotX = pointer.x;
    engagedSpotY = pointer.y;
  });

  function toneSurface(tone: PaneChromeTone): string {
    return tone === 'engaged' ? ENGAGED_SEGMENT_SURFACE : TRANSIENT_SURFACE;
  }

  function toneEdge(tone: PaneChromeTone): string {
    return tone === 'engaged' ? ENGAGED_EDGE : TRANSIENT_EDGE;
  }

  function segmentLaneMask(segment: HeaderSegment, includeEngaged: boolean): string | undefined {
    const geometry = segment.geometry;
    if (!geometry || !chrome.shown[segment.id] || (!includeEngaged && chrome.tone[segment.id] === 'engaged')) return;
    const samples = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = segment.side === 'leading' ? 1 - smootherstep(progress) : smootherstep(progress);
      const position =
        segment.side === 'leading'
          ? geometry.right + PANE_CHROME_FADE_WIDTH * progress
          : geometry.left - PANE_CHROME_FADE_WIDTH * (1 - progress);
      return `rgb(0 0 0 / ${alpha}) ${position}px`;
    });
    if (segment.side === 'leading') samples.push('transparent 100%');
    else samples.unshift('transparent 0');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function spotMask(opacityVariable: string): string | undefined {
    if (!spotVisible || !laneSpot) return;
    return paneChromeRadialMask(laneSpot.x, laneSpot.y, '--zen-pane-chrome-spot-radius', opacityVariable);
  }

  function resolveHeaderMasks(includeEngaged: boolean, opacityVariable: string): string[] {
    const spot = spotMask(opacityVariable);
    if (chrome.phase === 'expanding') {
      if (headerUnified && !chrome.expansionTargets.identity && !chrome.expansionTargets.actions) {
        return ['linear-gradient(black, black)'];
      }
      return [
        spot,
        ...headerSegments
          .filter((segment) => !chrome.expansionTargets[segment.id])
          .map((segment) => segmentLaneMask(segment, includeEngaged)),
      ].filter((mask): mask is string => mask !== undefined);
    }
    if (headerUnified) return ['linear-gradient(black, black)'];
    return [spot, ...headerSegments.map((segment) => segmentLaneMask(segment, includeEngaged))].filter(
      (mask): mask is string => mask !== undefined,
    );
  }

  function edgeMask(side: HeaderSegment['side']): string {
    const samples = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = side === 'leading' ? 1 - smootherstep(progress) : smootherstep(progress);
      const position =
        side === 'leading' ? `calc(100% - ${PANE_CHROME_FADE_WIDTH * (1 - progress)}px)` : `${PANE_CHROME_FADE_WIDTH * progress}px`;
      return `rgb(0 0 0 / ${alpha}) ${position}`;
    });
    if (side === 'leading') samples.unshift('black 0', `black calc(100% - ${PANE_CHROME_FADE_WIDTH}px)`);
    else samples.push(`black ${PANE_CHROME_FADE_WIDTH}px`, 'black 100%');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function engagedPointerMask(radius: number, strength: number): string {
    const stops = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      return `rgb(0 0 0 / ${strength * (1 - smootherstep(progress))}) ${radius * progress}px`;
    });
    return `radial-gradient(circle at ${engagedSpotX}px ${engagedSpotY}px, ${stops.join(', ')})`;
  }

  function engagedSpotMask(): string {
    return engagedPointerMask(ENGAGED_SPOT_RADIUS, ENGAGED_SPOT_STRENGTH);
  }

  function engagedEdgeSpotMask(): string {
    return engagedPointerMask(ENGAGED_EDGE_SPOT_RADIUS, 1);
  }
</script>

<div
  style:--zen-pane-chrome-spot-opacity={spotVisible ? 1 : 0}
  style:--zen-pane-chrome-spot-radius={`${spotRadius}px`}
  style:--zen-pane-chrome-spot-surface-opacity={spotSurfaceStrength}
  style:transition={prefersReducedMotion.current
    ? 'none'
    : `--zen-pane-chrome-spot-opacity ${chrome.phase === 'expanding' ? expansionTiming.spotEnterMs : SPOT_ENTER_MS}ms ease-out, --zen-pane-chrome-spot-surface-opacity ${chrome.phase === 'expanding' ? expansionTiming.spotSurfaceMs : SPOT_ENTER_MS}ms ease-out, --zen-pane-chrome-spot-radius ${expansionTiming.backgroundExpandMs}ms ${PANE_CHROME_EXPANSION_EASING}`}
  class={css({ position: 'absolute', inset: '0', isolation: 'isolate', pointerEvents: 'none' })}
  data-zen-pane-chrome-header-effects={lane === 'header' ? '' : undefined}
  data-zen-pane-chrome-phase={chrome.phase}
  data-zen-pane-chrome-toolbar-effects={lane === 'toolbar' ? '' : undefined}
  role="presentation"
>
  <div
    style:backdrop-filter={`blur(${laneVisible ? (spotVisible && !laneShown ? 1.5 : 2) : 0}px)`}
    style:mask-image={laneBlurMask || 'linear-gradient(transparent, transparent)'}
    style:mask-composite={lane === 'header' ? 'add' : undefined}
    style:transition={prefersReducedMotion.current
      ? 'none'
      : `backdrop-filter ${chrome.phase === 'expanding' ? expansionTiming.backgroundExpandMs : 320}ms ${
          chrome.phase === 'expanding' ? PANE_CHROME_EXPANSION_EASING : 'ease-out'
        }`}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-2]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-zen-pane-chrome-header-blur={lane === 'header' ? '' : undefined}
    data-zen-pane-chrome-toolbar-blur={lane === 'toolbar' ? '' : undefined}
  ></div>

  <div
    style:background={TRANSIENT_SURFACE}
    style:mask-image={laneSurfaceMask || 'linear-gradient(transparent, transparent)'}
    style:mask-composite={lane === 'header' ? 'add' : undefined}
    style:opacity={laneVisible ? '1' : '0'}
    style:transition={prefersReducedMotion.current ? 'none' : laneSurfaceTransition}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-zen-pane-chrome-header-full-lane={lane === 'header' ? headerUnified : undefined}
    data-zen-pane-chrome-header-lane-surface={lane === 'header' ? '' : undefined}
    data-zen-pane-chrome-spot-strength={spotSurfaceStrength}
    data-zen-pane-chrome-spot-x={laneSpot?.x}
    data-zen-pane-chrome-spot-y={laneSpot?.y}
    data-zen-pane-chrome-toolbar-surface={lane === 'toolbar' ? '' : undefined}
  >
    {#if lane === 'toolbar'}
      {#each toolbarSeparators as offset (offset)}
        <div
          style:background={TRANSIENT_EDGE}
          style:top={`${offset}px`}
          style:height={`${PANE_CHROME_BOUNDARY}px`}
          class={css({ position: 'absolute', right: '0', left: '0' })}
          data-zen-pane-chrome-toolbar-edge-boundary="separator"
        ></div>
      {/each}
    {/if}
    <div style:background={TRANSIENT_EDGE} class={css({ position: 'absolute', right: '0', bottom: '0', left: '0', height: '1px' })}></div>
  </div>

  <div
    style:background={ACTIVE_SURFACE}
    style:clip-path={lane === 'toolbar' ? toolbarEngagedSurfaceClip : undefined}
    style:mask-image={engagedSpotMask()}
    style:opacity={laneEngagedSpotVisible ? '1' : '0'}
    style:transition={prefersReducedMotion.current ? 'none' : laneEngagedSpotTransition}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-zen-pane-chrome-header-engaged-spot={lane === 'header' ? '' : undefined}
    data-zen-pane-chrome-toolbar-engaged-spot={lane === 'toolbar' ? '' : undefined}
  ></div>

  <div
    style:mask-image={engagedEdgeSpotMask()}
    style:opacity={laneEngagedSpotVisible ? '1' : '0'}
    style:transition={prefersReducedMotion.current ? 'none' : laneEngagedSpotTransition}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-zen-pane-chrome-engaged-edge-radius={ENGAGED_EDGE_SPOT_RADIUS}
    data-zen-pane-chrome-header-engaged-edge={lane === 'header' ? '' : undefined}
    data-zen-pane-chrome-toolbar-engaged-edge={lane === 'toolbar' ? '' : undefined}
  >
    {#if lane === 'header'}
      <div style:background={ENGAGED_EDGE} class={css({ position: 'absolute', right: '0', bottom: '0', left: '0', height: '1px' })}></div>
    {:else}
      {#if toolbarEngagedRowIndex === 0}
        <div
          style:background={ENGAGED_EDGE}
          style:height={`${PANE_CHROME_BOUNDARY}px`}
          class={css({ position: 'absolute', top: '[-1px]', right: '0', left: '0' })}
          data-zen-pane-chrome-toolbar-engaged-edge-boundary="top"
        ></div>
      {/if}
      {#each toolbarSeparators as offset, index (offset)}
        {#if toolbarEngagedRowIndex === index || toolbarEngagedRowIndex === index + 1}
          <div
            style:background={ENGAGED_EDGE}
            style:top={`${offset}px`}
            style:height={`${PANE_CHROME_BOUNDARY}px`}
            class={css({ position: 'absolute', right: '0', left: '0' })}
            data-zen-pane-chrome-toolbar-engaged-edge-boundary="separator"
          ></div>
        {/if}
      {/each}
      {#if toolbarEngagedRowIndex === toolbarSeparators.length}
        <div
          style:background={ENGAGED_EDGE}
          style:height={`${PANE_CHROME_BOUNDARY}px`}
          class={css({ position: 'absolute', right: '0', bottom: '0', left: '0' })}
          data-zen-pane-chrome-toolbar-engaged-edge-boundary="bottom"
        ></div>
      {/if}
    {/if}
  </div>

  {#if lane === 'header'}
    {#each headerSegments as segment (segment.id)}
      {#if segment.geometry}
        {@const leading = segment.side === 'leading'}
        {@const visible = chrome.isSurfaceVisible(segment.id) && chrome.tone[segment.id] === 'engaged'}
        <div
          style:left={leading ? '0' : undefined}
          style:right={leading ? undefined : '0'}
          style:width={`${segment.geometry.width + PANE_CHROME_FADE_WIDTH}px`}
          style:opacity={visible ? '1' : '0'}
          style:background-color={toneSurface(chrome.tone[segment.id])}
          style:mask-image={edgeMask(segment.side)}
          style:transition={prefersReducedMotion.current
            ? 'none'
            : `opacity ${visible ? PANE_CHROME_FOREGROUND_FADE_IN_MS : PANE_CHROME_FADE_OUT_MS}ms ease-out, background-color ${PANE_CHROME_FOREGROUND_FADE_IN_MS}ms ease-out`}
          class={css({ position: 'absolute', top: '0', bottom: '0', zIndex: '[-1]', pointerEvents: 'none' })}
          aria-hidden="true"
          data-zen-pane-chrome-header-surface={segment.id}
        >
          <div
            style:background-color={toneEdge(chrome.tone[segment.id])}
            class={css({ position: 'absolute', right: '0', bottom: '0', left: '0', height: '1px' })}
          ></div>
        </div>
      {/if}
    {/each}
  {/if}
</div>

<style>
  @property --zen-pane-chrome-spot-radius {
    syntax: '<length>';
    inherits: true;
    initial-value: 88px;
  }

  @property --zen-pane-chrome-spot-opacity {
    syntax: '<number>';
    inherits: true;
    initial-value: 0;
  }

  @property --zen-pane-chrome-spot-surface-opacity {
    syntax: '<number>';
    inherits: true;
    initial-value: 0;
  }

  @property --zen-pane-chrome-foreground-radius {
    syntax: '<length>';
    inherits: true;
    initial-value: 88px;
  }

  @property --zen-pane-chrome-foreground-opacity {
    syntax: '<number>';
    inherits: true;
    initial-value: 0;
  }
</style>
