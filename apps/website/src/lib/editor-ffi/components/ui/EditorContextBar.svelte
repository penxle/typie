<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { hoverIntent } from '@typie/ui/actions';
  import { VerticalDivider } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import {
    CONTEXT_BAR_FADE_IN_MS,
    CONTEXT_BAR_FADE_OUT_MS,
    CONTEXT_BAR_TRANSIENT_VISIBLE_MS,
    ContextBarVisibilityCoordinator,
    smootherstep,
  } from './editor-context-bar.svelte';
  import EditorContextBarPinControl from './EditorContextBarPinControl.svelte';
  import EditorContextBarSegment from './EditorContextBarSegment.svelte';
  import { TransientVisibilityState } from './transient-visibility.svelte';
  import type { HoverIntentParameter } from '@typie/ui/actions';
  import type { Snippet } from 'svelte';
  import type { ActionReturn } from 'svelte/action';
  import type { ContextBarPresentation, ContextBarSegmentPresentation } from './editor-context-bar.svelte';

  export type EditorContextBarSegmentRenderProps = {
    state: TransientVisibilityState;
    presentation: ContextBarSegmentPresentation;
  };

  type Props = {
    editorViewSurface: HTMLElement;
    showViewControlsOnPaneEntry: boolean;
    breadcrumb?: Snippet<[EditorContextBarSegmentRenderProps]>;
    viewControls: Snippet<[EditorContextBarSegmentRenderProps]>;
    interactiveViewControlsWhenHidden?: boolean;
    pinned?: boolean;
    onTopOcclusionChange?: (topOcclusion: number) => void;
    onPinnedChange?: (pinned: boolean) => void;
  };

  type SegmentGeometry = {
    left: number;
    right: number;
    top: number;
    bottom: number;
    width: number;
  };

  type LanePoint = { x: number; y: number };

  type LaneHoverPhase = 'idle' | 'pending' | 'preview' | 'spot' | 'expanding' | 'held';
  type ContextBarSegmentId = 'leading' | 'viewControls';

  const FADE_WIDTH = 24;
  const LANE_HOVER_INTENT_DELAY_MS = 400;
  const HIDDEN_SEGMENT_HOVER_INTENT_DELAY_MS = 400;
  const LANE_SPOT_DWELL_MS = 500;
  const LANE_SPOT_ENTER_MS = 500;
  const LANE_SPOT_SURFACE_TRANSITION_MS = 600;
  const LANE_BACKGROUND_EXPAND_MS = 900;
  const LANE_EXPANSION_TOTAL_MS = 1000;
  const LANE_FOREGROUND_REVEAL_DELAY_MS = 100;
  const LANE_FOREGROUND_REVEAL_MS = 900;
  const LANE_SPOT_RADIUS = 88;
  const LANE_ENGAGED_SPOT_RADIUS = 52;
  const LANE_ENGAGED_SPOT_STRENGTH = 0.85;
  const LANE_ENGAGED_SPOT_ENTER_DELAY_MS = 500;
  const LANE_SPOT_MASK_INSET = 8;
  const LANE_SPOT_EDGE_WIDTH = 48;
  const LANE_SPOT_SURFACE_STRENGTH = 0.7;
  const LANE_SPOT_EXPANSION_EASING = 'cubic-bezier(0.22, 1, 0.36, 1)';
  const LANE_HOLD_REASON = 'lane-hover';
  const PIN_HOLD_REASON = 'pinned';
  const MUTED_SURFACE = token('colors.surface.muted');
  const CONTINUOUS_SURFACE = token('colors.surface.default');
  const PAGINATED_SURFACE = token('colors.surface.subtle');
  const TRANSIENT_BASE = `color-mix(in srgb, ${CONTINUOUS_SURFACE} 50%, ${PAGINATED_SURFACE} 50%)`;
  const ACTIVE_SURFACE = `color-mix(in srgb, ${MUTED_SURFACE} 80%, ${TRANSIENT_BASE} 20%)`;
  const SUBTLE_BORDER = token('colors.border.subtle');
  const DEFAULT_BORDER = token('colors.border.default');
  const TRANSIENT_SURFACE = `color-mix(in srgb, ${TRANSIENT_BASE} 75%, transparent)`;
  const TRANSIENT_EDGE = `color-mix(in srgb, ${TRANSIENT_BASE} 40%, ${SUBTLE_BORDER} 60%)`;
  const ENGAGED_EDGE = `color-mix(in srgb, ${ACTIVE_SURFACE} 36%, ${DEFAULT_BORDER} 64%)`;

  let {
    editorViewSurface,
    showViewControlsOnPaneEntry,
    breadcrumb,
    viewControls,
    interactiveViewControlsWhenHidden = false,
    pinned = false,
    onTopOcclusionChange,
    onPinnedChange,
  }: Props = $props();

  const leadingState = new TransientVisibilityState();
  const viewControlsState = new TransientVisibilityState();
  const visibilityCoordinator = new ContextBarVisibilityCoordinator();
  let lane = $state<HTMLElement>();
  let leadingElement = $state<HTMLElement>();
  let viewControlsElement = $state<HTMLElement>();
  let leadingGeometry = $state<SegmentGeometry>();
  let viewControlsGeometry = $state<SegmentGeometry>();
  let laneWidth = $state(0);
  let laneHeight = $state(0);
  let laneHoverPhase = $state<LaneHoverPhase>('idle');
  let pointerInLane = $state(false);
  let pointerInLaneGap = $state(false);
  let lanePointerX = $state(0);
  let lanePointerY = $state(0);
  let laneExpansionOriginX = $state(0);
  let laneExpansionOriginY = $state(0);
  let laneForegroundRevealStarted = $state(false);
  let laneExpansionMaskedSegments = $state<ContextBarSegmentId[]>([]);
  let fadingTransientSegments = $state<ContextBarSegmentId[]>([]);
  let transientExitPresentation = $state<ContextBarPresentation>();
  let laneHoverIntent: ActionReturn<HoverIntentParameter> | undefined;
  let laneDwellTimer: ReturnType<typeof setTimeout> | undefined;
  let laneForegroundRevealTimer: ReturnType<typeof setTimeout> | undefined;
  let laneExpansionTimer: ReturnType<typeof setTimeout> | undefined;
  let hiddenSegmentIntentTimer: ReturnType<typeof setTimeout> | undefined;
  let hiddenSegmentIntentTarget: ContextBarSegmentId | undefined;
  let hiddenSegmentEngaged: ContextBarSegmentId | undefined;
  let transientExitTimer: ReturnType<typeof setTimeout> | undefined;
  const fadingTransientTimers = new SvelteMap<ContextBarSegmentId, ReturnType<typeof setTimeout>>();
  let initialRevealComplete = false;
  let previousPinned: boolean | undefined;

  let presentation = $state<ContextBarPresentation>({
    unified: false,
    leading: { visible: false, tone: 'transient' },
    viewControls: { visible: false, tone: 'transient' },
  });
  let previousPresentation = presentation;
  const surfaceMaskPresentation = $derived(transientExitPresentation ?? presentation);

  const unifiedPairVisible = $derived(presentation.unified);
  const unifiedSurfaceVisible = $derived(unifiedPairVisible && laneHoverPhase !== 'expanding');
  const laneSpotVisible = $derived(laneHoverPhase === 'preview' || laneHoverPhase === 'spot' || laneHoverPhase === 'expanding');
  const hasEngagedSegment = $derived(
    (presentation.leading.visible && presentation.leading.tone === 'engaged') ||
      (presentation.viewControls.visible && presentation.viewControls.tone === 'engaged'),
  );
  const laneEngagedSpotVisible = $derived(pointerInLane && (laneSpotVisible || laneHoverPhase === 'held' || hasEngagedSegment));
  const laneSpotSurfaceStrength = $derived(
    laneHoverPhase === 'expanding' ? 1 : laneHoverPhase === 'preview' || laneHoverPhase === 'spot' ? LANE_SPOT_SURFACE_STRENGTH : 0,
  );
  const laneSpotOpacityTransitionMs = $derived(unifiedSurfaceVisible ? 0 : LANE_SPOT_ENTER_MS);
  const laneEngagedSpotEnterDelayMs = $derived(pinned ? 0 : LANE_ENGAGED_SPOT_ENTER_DELAY_MS);
  const laneSpotSurfaceTransitionMs = $derived(
    unifiedSurfaceVisible
      ? 0
      : laneHoverPhase === 'expanding'
        ? LANE_SPOT_SURFACE_TRANSITION_MS
        : laneHoverPhase === 'preview' || laneHoverPhase === 'spot'
          ? LANE_SPOT_ENTER_MS
          : CONTEXT_BAR_FADE_OUT_MS,
  );
  const laneExpansionRadius = $derived(Math.hypot(laneWidth, laneHeight) + FADE_WIDTH + LANE_SPOT_MASK_INSET);
  const laneSpotRadius = $derived(laneHoverPhase === 'expanding' ? laneExpansionRadius : LANE_SPOT_RADIUS);
  const laneForegroundRevealRadius = $derived(laneForegroundRevealStarted ? laneExpansionRadius : LANE_SPOT_RADIUS);
  const laneTransientSpotX = $derived(laneHoverPhase === 'expanding' ? laneExpansionOriginX : lanePointerX);
  const laneTransientSpotY = $derived(laneHoverPhase === 'expanding' ? laneExpansionOriginY : lanePointerY);
  const laneBlurMask = $derived.by(() => laneBlurMasks().join(', '));
  const laneSurfaceMask = $derived.by(() => laneSurfaceMasks().join(', '));
  const laneSurfaceMaskComposite = $derived(laneHoverPhase === 'preview' && fadingTransientSegments.length === 0 ? 'intersect' : 'add');
  const laneBlurVisible = $derived(
    laneSpotVisible ||
      fadingTransientSegments.length > 0 ||
      (unifiedSurfaceVisible && (presentation.leading.tone === 'transient' || presentation.viewControls.tone === 'transient')) ||
      (!unifiedPairVisible &&
        ((presentation.leading.visible && presentation.leading.tone === 'transient') ||
          (presentation.viewControls.visible && presentation.viewControls.tone === 'transient'))),
  );
  const hasVisibleSegment = $derived(presentation.leading.visible || presentation.viewControls.visible);
  const topOcclusion = $derived(hasVisibleSegment || laneSpotVisible || transientExitPresentation ? laneHeight : 0);
  const laneBlurRadius = $derived.by(() => {
    if (!laneBlurVisible) return 0;
    if (!hasVisibleSegment && (laneHoverPhase === 'preview' || laneHoverPhase === 'spot')) return 1.5;
    return 2;
  });
  const laneBlurTransitionMs = $derived(
    laneHoverPhase === 'expanding'
      ? LANE_BACKGROUND_EXPAND_MS
      : laneHoverPhase === 'preview' || laneHoverPhase === 'spot'
        ? LANE_SPOT_ENTER_MS
        : transientExitPresentation
          ? CONTEXT_BAR_FADE_OUT_MS
          : 320,
  );

  function toneColor(tone: ContextBarSegmentPresentation['tone']): string {
    return tone === 'engaged' ? ACTIVE_SURFACE : TRANSIENT_SURFACE;
  }

  function edgeColor(tone: ContextBarSegmentPresentation['tone']): string {
    return tone === 'engaged' ? ENGAGED_EDGE : TRANSIENT_EDGE;
  }

  function maskStops(side: 'leading' | 'trailing'): string {
    const samples = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = side === 'leading' ? 1 - smootherstep(progress) : smootherstep(progress);
      const position = side === 'leading' ? `calc(100% - ${FADE_WIDTH * (1 - progress)}px)` : `${FADE_WIDTH * progress}px`;
      return `rgb(0 0 0 / ${alpha}) ${position}`;
    });
    if (side === 'leading') samples.unshift(`black 0`, `black calc(100% - ${FADE_WIDTH}px)`);
    else samples.push(`black ${FADE_WIDTH}px`, 'black 100%');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function radialMask(
    x: number,
    y: number,
    radiusVariable = '--context-bar-lane-spot-radius',
    opacityVariable = '--context-bar-lane-spot-opacity',
  ): string {
    const edgeStops = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = 1 - smootherstep(progress);
      return `rgb(0 0 0 / calc(${alpha} * var(${opacityVariable}))) calc(var(${radiusVariable}) - ${LANE_SPOT_MASK_INSET + LANE_SPOT_EDGE_WIDTH * (1 - progress)}px)`;
    });
    return `radial-gradient(circle at ${x}px ${y}px, rgb(0 0 0 / var(${opacityVariable})) 0, rgb(0 0 0 / var(${opacityVariable})) calc(var(${radiusVariable}) - ${LANE_SPOT_MASK_INSET + LANE_SPOT_EDGE_WIDTH}px), ${edgeStops.join(', ')})`;
  }

  function laneSpotMask(opacityVariable: string): string {
    return radialMask(laneTransientSpotX, laneTransientSpotY, '--context-bar-lane-spot-radius', opacityVariable);
  }

  function engagedSpotMask(): string {
    const stops = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      return `rgb(0 0 0 / ${LANE_ENGAGED_SPOT_STRENGTH * (1 - smootherstep(progress))}) ${LANE_ENGAGED_SPOT_RADIUS * progress}px`;
    });
    return `radial-gradient(circle at ${lanePointerX}px ${lanePointerY}px, ${stops.join(', ')})`;
  }

  function unifiedTransientMask(): string {
    if (!leadingGeometry || !viewControlsGeometry) return 'linear-gradient(transparent, transparent)';
    const hasTransientSegment =
      surfaceMaskPresentation.leading.tone === 'transient' ||
      surfaceMaskPresentation.viewControls.tone === 'transient' ||
      fadingTransientSegments.length > 0;
    if (hasTransientSegment) return 'linear-gradient(black, black)';

    const leadingEngaged = surfaceMaskPresentation.leading.tone === 'engaged' && !fadingTransientSegments.includes('leading');
    const viewControlsEngaged =
      surfaceMaskPresentation.viewControls.tone === 'engaged' && !fadingTransientSegments.includes('viewControls');
    if (!leadingEngaged && !viewControlsEngaged) return 'linear-gradient(black, black)';

    const width = viewControlsGeometry.right - leadingGeometry.left;
    const leadingRight = leadingGeometry.right - leadingGeometry.left;
    const viewControlsLeft = viewControlsGeometry.left - leadingGeometry.left;
    const positions = [0, width];
    const addPosition = (position: number) => {
      if (!positions.includes(position)) positions.push(position);
    };
    if (leadingEngaged) {
      for (let index = 0; index <= 8; index += 1) addPosition(Math.min(width, leadingRight + (FADE_WIDTH * index) / 8));
    }
    if (viewControlsEngaged) {
      for (let index = 0; index <= 8; index += 1) addPosition(Math.max(0, viewControlsLeft - FADE_WIDTH + (FADE_WIDTH * index) / 8));
    }

    const stops = positions
      .toSorted((left, right) => left - right)
      .map((position) => {
        const leadingAlpha = leadingEngaged ? smootherstep((position - leadingRight) / FADE_WIDTH) : 1;
        const viewControlsAlpha = viewControlsEngaged ? 1 - smootherstep((position - (viewControlsLeft - FADE_WIDTH)) / FADE_WIDTH) : 1;
        return `rgb(0 0 0 / ${Math.min(leadingAlpha, viewControlsAlpha)}) ${position}px`;
      });
    return `linear-gradient(to right, ${stops.join(', ')})`;
  }

  function segmentLaneMask(segment: ContextBarSegmentId, transientOnly: boolean): string | undefined {
    const geometry = segment === 'leading' ? leadingGeometry : viewControlsGeometry;
    const segmentPresentation = surfaceMaskPresentation[segment];
    if (
      !geometry ||
      !segmentPresentation.visible ||
      (transientOnly && segmentPresentation.tone !== 'transient' && !fadingTransientSegments.includes(segment)) ||
      laneExpansionMaskedSegments.includes(segment)
    ) {
      return;
    }

    const samples = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = segment === 'leading' ? 1 - smootherstep(progress) : smootherstep(progress);
      const position = segment === 'leading' ? geometry.right + FADE_WIDTH * progress : geometry.left - FADE_WIDTH * (1 - progress);
      return `rgb(0 0 0 / ${alpha}) ${position}px`;
    });
    if (segment === 'leading') samples.push('transparent 100%');
    else samples.unshift('transparent 0');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function loneEngagedSegment(): ContextBarSegmentId | undefined {
    const visibleSegments = (['leading', 'viewControls'] as const).filter((segment) => presentation[segment].visible);
    const segment = visibleSegments.length === 1 ? visibleSegments[0] : undefined;
    if (!segment) return;
    const state = segmentState(segment);
    return state.hovered || state.focused ? segment : undefined;
  }

  function outsideEngagedSegmentMask(segment: ContextBarSegmentId): string {
    const geometry = segment === 'leading' ? leadingGeometry : viewControlsGeometry;
    if (!geometry) return 'linear-gradient(transparent, transparent)';
    const samples = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = segment === 'leading' ? smootherstep(progress) : 1 - smootherstep(progress);
      const position = segment === 'leading' ? geometry.right + FADE_WIDTH * progress : geometry.left - FADE_WIDTH * (1 - progress);
      return `rgb(0 0 0 / ${alpha}) ${position}px`;
    });
    if (segment === 'leading') samples.push('black 100%');
    else samples.unshift('black 0');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function laneBlurMasks(): string[] {
    const masks = [laneSpotMask('--context-bar-lane-spot-opacity')];
    const hasTransientSegment =
      surfaceMaskPresentation.leading.tone === 'transient' ||
      surfaceMaskPresentation.viewControls.tone === 'transient' ||
      fadingTransientSegments.length > 0;
    if (unifiedSurfaceVisible || transientExitPresentation?.unified) {
      if (hasTransientSegment) masks.push('linear-gradient(black, black)');
      return masks;
    }

    const includeEngagedSegments = laneSpotVisible;
    const leadingMask = segmentLaneMask('leading', !includeEngagedSegments);
    const viewControlsMask = segmentLaneMask('viewControls', !includeEngagedSegments);
    if (leadingMask) masks.push(leadingMask);
    if (viewControlsMask) masks.push(viewControlsMask);
    return masks;
  }

  function laneSurfaceMasks(): string[] {
    const masks = [laneSpotMask('--context-bar-lane-spot-surface-opacity')];
    if (laneHoverPhase === 'preview') {
      const engagedSegment = loneEngagedSegment();
      const fadingMask = engagedSegment ? segmentLaneMask(engagedSegment, true) : undefined;
      masks.push(fadingMask ?? (engagedSegment ? outsideEngagedSegmentMask(engagedSegment) : 'linear-gradient(transparent, transparent)'));
      return masks;
    }
    if (unifiedSurfaceVisible || transientExitPresentation?.unified) {
      masks.push(unifiedTransientMask());
      return masks;
    }
    const leadingMask = segmentLaneMask('leading', true);
    const viewControlsMask = segmentLaneMask('viewControls', true);
    if (leadingMask) masks.push(leadingMask);
    if (viewControlsMask) masks.push(viewControlsMask);
    return masks;
  }

  function segmentRevealMask(segment: ContextBarSegmentId, geometry: SegmentGeometry | undefined): string {
    if (!geometry || laneHoverPhase !== 'expanding' || !laneExpansionMaskedSegments.includes(segment)) return 'none';
    return radialMask(
      laneExpansionOriginX - geometry.left,
      laneExpansionOriginY,
      '--context-bar-segment-reveal-radius',
      '--context-bar-segment-reveal-opacity',
    );
  }

  function syncGeometry(): void {
    if (!lane || !viewControlsElement) return;
    const laneRect = lane.getBoundingClientRect();
    laneWidth = laneRect.width;
    laneHeight = laneRect.height;
    const relativeGeometry = (element: HTMLElement): SegmentGeometry => {
      const rect = element.getBoundingClientRect();
      return {
        left: rect.left - laneRect.left,
        right: rect.right - laneRect.left,
        top: rect.top - laneRect.top,
        bottom: rect.bottom - laneRect.top,
        width: rect.width,
      };
    };
    leadingGeometry = leadingElement ? relativeGeometry(leadingElement) : undefined;
    viewControlsGeometry = relativeGeometry(viewControlsElement);
  }

  function segmentState(segment: ContextBarSegmentId): TransientVisibilityState {
    return segment === 'leading' ? leadingState : viewControlsState;
  }

  function clearHiddenSegmentIntent(): void {
    clearTimeout(hiddenSegmentIntentTimer);
    hiddenSegmentIntentTimer = undefined;
    hiddenSegmentIntentTarget = undefined;
  }

  function retainTransientDuringEngagement(segment: ContextBarSegmentId): void {
    clearTimeout(fadingTransientTimers.get(segment));
    if (prefersReducedMotion.current) {
      fadingTransientTimers.delete(segment);
      fadingTransientSegments = fadingTransientSegments.filter((candidate) => candidate !== segment);
      return;
    }

    if (!fadingTransientSegments.includes(segment)) fadingTransientSegments = [...fadingTransientSegments, segment];
    fadingTransientTimers.set(
      segment,
      setTimeout(() => {
        fadingTransientTimers.delete(segment);
        fadingTransientSegments = fadingTransientSegments.filter((candidate) => candidate !== segment);
      }, CONTEXT_BAR_FADE_IN_MS),
    );
  }

  function clearFadingTransientSegments(): void {
    for (const timer of fadingTransientTimers.values()) clearTimeout(timer);
    fadingTransientTimers.clear();
    fadingTransientSegments = [];
  }

  function clearTransientSurfaceExit(): void {
    clearTimeout(transientExitTimer);
    transientExitTimer = undefined;
    transientExitPresentation = undefined;
  }

  function startTransientSurfaceExit(previous: ContextBarPresentation): void {
    clearTransientSurfaceExit();
    if (prefersReducedMotion.current) return;
    transientExitPresentation = previous;
    transientExitTimer = setTimeout(() => {
      transientExitTimer = undefined;
      transientExitPresentation = undefined;
    }, CONTEXT_BAR_FADE_OUT_MS);
  }

  function releaseHiddenSegmentHover(): void {
    if (!hiddenSegmentEngaged) return;
    const state = segmentState(hiddenSegmentEngaged);
    hiddenSegmentEngaged = undefined;
    state.setHovered(false);
    state.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
  }

  function engageHiddenSegment(segment: ContextBarSegmentId): void {
    clearHiddenSegmentIntent();
    hiddenSegmentEngaged = segment;
    segmentState(segment).setHovered(true);
  }

  function geometryContainsPoint(geometry: SegmentGeometry | undefined, point: LanePoint): boolean {
    return (
      geometry !== undefined &&
      point.x >= geometry.left &&
      point.x <= geometry.right &&
      point.y >= geometry.top &&
      point.y <= geometry.bottom
    );
  }

  function segmentAtPoint(point: LanePoint): ContextBarSegmentId | undefined {
    if (geometryContainsPoint(leadingGeometry, point)) return 'leading';
    if (geometryContainsPoint(viewControlsGeometry, point)) return 'viewControls';
  }

  function laneHoverIntentParameter(intentEnabled: boolean): HoverIntentParameter {
    return {
      delay: LANE_HOVER_INTENT_DELAY_MS,
      intentEnabled,
      samples: 1,
      onIntent: () => {
        if (pointerInLaneGap && laneHoverPhase === 'pending') startLaneSpot();
      },
    };
  }

  function setLaneHoverIntentEnabled(enabled: boolean): void {
    laneHoverIntent?.update?.(laneHoverIntentParameter(enabled));
  }

  function clearLaneTimers(): void {
    clearTimeout(laneDwellTimer);
    clearTimeout(laneForegroundRevealTimer);
    clearTimeout(laneExpansionTimer);
    setLaneHoverIntentEnabled(false);
    laneDwellTimer = undefined;
    laneForegroundRevealTimer = undefined;
    laneExpansionTimer = undefined;
    laneForegroundRevealStarted = false;
  }

  function holdLane(): void {
    clearLaneTimers();
    laneHoverPhase = 'held';
    laneExpansionMaskedSegments = [];
    leadingState.hold(LANE_HOLD_REASON);
    viewControlsState.hold(LANE_HOLD_REASON);
  }

  function stopLaneInteraction(linger: boolean, pointerStillInLane = false): void {
    const wasHeld = laneHoverPhase === 'held';
    pointerInLane = pointerStillInLane;
    pointerInLaneGap = false;
    clearLaneTimers();
    laneHoverPhase = 'idle';
    laneExpansionMaskedSegments = [];
    leadingState.release(LANE_HOLD_REASON);
    viewControlsState.release(LANE_HOLD_REASON);
    if (!wasHeld || !linger) return;
    leadingState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    viewControlsState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
  }

  function settlePinnedPresentation(): void {
    clearHiddenSegmentIntent();
    releaseHiddenSegmentHover();
    clearLaneTimers();
    clearFadingTransientSegments();
    clearTransientSurfaceExit();
    laneHoverPhase = 'idle';
    pointerInLaneGap = false;
    laneExpansionMaskedSegments = [];
    leadingState.release(LANE_HOLD_REASON);
    viewControlsState.release(LANE_HOLD_REASON);
    leadingState.hold(PIN_HOLD_REASON);
    viewControlsState.hold(PIN_HOLD_REASON);
  }

  function startLaneSpot(): void {
    clearLaneTimers();
    laneExpansionMaskedSegments = [];
    laneHoverPhase = 'spot';
    laneDwellTimer = setTimeout(() => {
      laneDwellTimer = undefined;
      if (prefersReducedMotion.current) {
        holdLane();
        return;
      }
      startLaneExpansion();
    }, LANE_SPOT_DWELL_MS);
  }

  function startLanePreview(): void {
    clearLaneTimers();
    laneExpansionMaskedSegments = [];
    laneHoverPhase = 'preview';
  }

  function startLaneExpansion(): void {
    clearLaneTimers();
    if (prefersReducedMotion.current) {
      holdLane();
      return;
    }
    laneExpansionOriginX = lanePointerX;
    laneExpansionOriginY = lanePointerY;
    laneExpansionMaskedSegments = (['leading', 'viewControls'] as const).filter((segment) => !presentation[segment].visible);
    laneHoverPhase = 'expanding';
    leadingState.hold(LANE_HOLD_REASON);
    viewControlsState.hold(LANE_HOLD_REASON);
    laneForegroundRevealTimer = setTimeout(() => {
      laneForegroundRevealTimer = undefined;
      laneForegroundRevealStarted = true;
    }, LANE_FOREGROUND_REVEAL_DELAY_MS);
    laneExpansionTimer = setTimeout(() => {
      laneExpansionTimer = undefined;
      holdLane();
    }, LANE_EXPANSION_TOTAL_MS);
  }

  function startLaneIntent(): void {
    if (laneHoverPhase !== 'idle') return;
    laneHoverPhase = 'pending';
    setLaneHoverIntentEnabled(true);
  }

  function lanePoint(event: PointerEvent): LanePoint | undefined {
    if (!lane) return;
    const rect = lane.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    if (x < 0 || x > rect.width || y < 0 || y > rect.height) return;
    return { x, y };
  }

  function isLaneGap(point: LanePoint): boolean {
    if (!leadingGeometry || !viewControlsGeometry || viewControlsGeometry.left <= leadingGeometry.right) return false;
    return point.x > leadingGeometry.right && point.x < viewControlsGeometry.left;
  }

  function handlePanePointerMove(event: PointerEvent): void {
    if (!leadingElement || event.pointerType === 'touch') {
      stopLaneInteraction(false);
      clearHiddenSegmentIntent();
      releaseHiddenSegmentHover();
      return;
    }

    const point = lanePoint(event);
    if (!point) {
      stopLaneInteraction(true);
      clearHiddenSegmentIntent();
      releaseHiddenSegmentHover();
      return;
    }

    pointerInLane = true;
    lanePointerX = point.x;
    lanePointerY = point.y;

    if (isLaneGap(point)) {
      pointerInLaneGap = true;
      clearHiddenSegmentIntent();
      releaseHiddenSegmentHover();

      if (laneHoverPhase === 'expanding') {
        return;
      }
      if (laneHoverPhase === 'preview') {
        startLaneSpot();
        return;
      }
      if (laneHoverPhase === 'held' || presentation.unified) {
        holdLane();
      } else if (laneHoverPhase === 'idle') {
        if (presentation.leading.visible || presentation.viewControls.visible) startLaneSpot();
        else startLaneIntent();
      }
      return;
    }

    const segment = segmentAtPoint(point);
    if (segment && loneEngagedSegment() === segment) {
      pointerInLaneGap = false;
      if (laneHoverPhase !== 'preview') startLanePreview();
      return;
    }

    if (segment && (laneHoverPhase === 'spot' || laneHoverPhase === 'expanding' || laneHoverPhase === 'held')) {
      pointerInLaneGap = false;
      if (hiddenSegmentEngaged === segment) return;
      if (hiddenSegmentEngaged) releaseHiddenSegmentHover();
      if (presentation[segment].visible) {
        clearHiddenSegmentIntent();
        return;
      }

      const sibling: ContextBarSegmentId = segment === 'leading' ? 'viewControls' : 'leading';
      if (presentation[sibling].visible) engageHiddenSegment(segment);
      return;
    }

    stopLaneInteraction(true, true);
    if (!segment) {
      clearHiddenSegmentIntent();
      releaseHiddenSegmentHover();
      return;
    }
    if (hiddenSegmentEngaged === segment) return;
    if (hiddenSegmentEngaged) releaseHiddenSegmentHover();

    const ownVisible = presentation[segment].visible;
    if (ownVisible) {
      clearHiddenSegmentIntent();
      return;
    }

    const sibling: ContextBarSegmentId = segment === 'leading' ? 'viewControls' : 'leading';
    if (presentation[sibling].visible) {
      engageHiddenSegment(segment);
      return;
    }

    if (hiddenSegmentIntentTarget === segment) return;
    clearHiddenSegmentIntent();
    hiddenSegmentIntentTarget = segment;
    hiddenSegmentIntentTimer = setTimeout(() => {
      hiddenSegmentIntentTimer = undefined;
      hiddenSegmentIntentTarget = undefined;
      engageHiddenSegment(segment);
    }, HIDDEN_SEGMENT_HOVER_INTENT_DELAY_MS);
  }

  function handlePanePointerLeave(): void {
    stopLaneInteraction(true);
    clearHiddenSegmentIntent();
    releaseHiddenSegmentHover();
  }

  $effect(() => {
    const nextPinned = pinned;
    untrack(() => {
      if (nextPinned) {
        settlePinnedPresentation();
      } else {
        leadingState.release(PIN_HOLD_REASON);
        viewControlsState.release(PIN_HOLD_REASON);
        if (previousPinned) {
          leadingState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
          viewControlsState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
        }
      }
      previousPinned = nextPinned;
    });
  });

  $effect(() => {
    const nextPresentation = visibilityCoordinator.resolve({
      leading: leadingState.activity,
      viewControls: viewControlsState.activity,
    });
    const hadTransientSurface =
      (previousPresentation.leading.visible && previousPresentation.leading.tone === 'transient') ||
      (previousPresentation.viewControls.visible && previousPresentation.viewControls.tone === 'transient');
    const nextHidden = !nextPresentation.leading.visible && !nextPresentation.viewControls.visible;
    if (hadTransientSurface && nextHidden) startTransientSurfaceExit(previousPresentation);
    else if (!nextHidden) clearTransientSurfaceExit();

    for (const segment of ['leading', 'viewControls'] as const) {
      if (
        previousPresentation[segment].visible &&
        previousPresentation[segment].tone === 'transient' &&
        nextPresentation[segment].visible &&
        nextPresentation[segment].tone === 'engaged'
      ) {
        retainTransientDuringEngagement(segment);
      }
    }
    if (laneHoverPhase === 'expanding') {
      const nextMaskedSegments = laneExpansionMaskedSegments.filter((segment) => nextPresentation[segment].tone !== 'engaged');
      if (nextMaskedSegments.length !== laneExpansionMaskedSegments.length) laneExpansionMaskedSegments = nextMaskedSegments;
    }
    previousPresentation = nextPresentation;
    presentation = nextPresentation;
  });

  $effect(() => {
    if (laneHoverPhase !== 'expanding' && laneHoverPhase !== 'held' && pointerInLaneGap && presentation.unified) holdLane();
  });

  $effect(() => {
    if (laneHoverPhase === 'expanding' && prefersReducedMotion.current) holdLane();
  });

  $effect(() => {
    const currentLane = lane;
    const currentLeading = leadingElement;
    const currentViewControls = viewControlsElement;
    if (!currentLane || !currentViewControls) return;
    const observer = new ResizeObserver(syncGeometry);
    observer.observe(currentLane);
    observer.observe(currentViewControls);
    if (currentLeading) observer.observe(currentLeading);
    syncGeometry();
    return () => observer.disconnect();
  });

  $effect(() => {
    if (initialRevealComplete || (leadingGeometry?.width ?? 0) <= 0 || (viewControlsGeometry?.width ?? 0) <= 0) return;
    initialRevealComplete = true;
    leadingState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    viewControlsState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
  });

  $effect(() => {
    const target = editorViewSurface.closest<HTMLElement>('[data-pane-id]') ?? editorViewSurface;
    const currentLaneHoverIntent = hoverIntent(target, laneHoverIntentParameter(false)) ?? undefined;
    laneHoverIntent = currentLaneHoverIntent;
    const handlePointerEnter = (event: PointerEvent) => {
      if (event.pointerType === 'touch') return;
      leadingState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
      if (showViewControlsOnPaneEntry) viewControlsState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    };
    target.addEventListener('pointerenter', handlePointerEnter);
    target.addEventListener('pointerleave', handlePanePointerLeave);
    target.addEventListener('pointermove', handlePanePointerMove);
    return () => {
      currentLaneHoverIntent?.destroy?.();
      if (laneHoverIntent === currentLaneHoverIntent) laneHoverIntent = undefined;
      target.removeEventListener('pointerenter', handlePointerEnter);
      target.removeEventListener('pointerleave', handlePanePointerLeave);
      target.removeEventListener('pointermove', handlePanePointerMove);
    };
  });

  $effect(() => {
    const nextTopOcclusion = topOcclusion;
    untrack(() => onTopOcclusionChange?.(nextTopOcclusion));
  });

  $effect(() => {
    return () => {
      onTopOcclusionChange?.(0);
      clearHiddenSegmentIntent();
      clearFadingTransientSegments();
      clearTransientSurfaceExit();
      stopLaneInteraction(false);
      leadingState.destroy();
      viewControlsState.destroy();
    };
  });
</script>

<div
  bind:this={lane}
  style:--context-bar-lane-spot-opacity={laneSpotVisible ? 1 : 0}
  style:--context-bar-lane-spot-surface-opacity={laneSpotSurfaceStrength}
  style:--context-bar-lane-spot-radius={`${laneSpotRadius}px`}
  style:transition={prefersReducedMotion.current
    ? 'none'
    : `--context-bar-lane-spot-opacity ${laneSpotOpacityTransitionMs}ms ease-out, --context-bar-lane-spot-surface-opacity ${laneSpotSurfaceTransitionMs}ms ease-out, --context-bar-lane-spot-radius ${LANE_BACKGROUND_EXPAND_MS}ms ${LANE_SPOT_EXPANSION_EASING}`}
  class={css({
    position: 'absolute',
    top: '0',
    right: '0',
    left: '0',
    zIndex: '5',
    isolation: 'isolate',
    display: 'flex',
    alignItems: 'flex-start',
    pointerEvents: 'none',
  })}
  data-context-bar-lane-phase={laneHoverPhase}
  data-context-bar-unified={presentation.unified}
  data-editor-context-bar
  role="presentation"
>
  <div
    style:backdrop-filter={`blur(${laneBlurRadius}px)`}
    style:mask-image={laneBlurMask}
    style:mask-composite="add"
    style:transition={prefersReducedMotion.current
      ? 'none'
      : `backdrop-filter ${laneBlurTransitionMs}ms ${laneHoverPhase === 'expanding' ? LANE_SPOT_EXPANSION_EASING : 'ease-out'}`}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-2]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-context-bar-blur-layer
    data-context-bar-blur-radius={laneBlurRadius}
  ></div>
  <div
    style:background={TRANSIENT_SURFACE}
    style:mask-image={laneSurfaceMask}
    style:mask-composite={laneSurfaceMaskComposite}
    style:opacity={laneBlurVisible ? '1' : '0'}
    style:transition={prefersReducedMotion.current ? 'none' : `opacity ${laneBlurVisible ? 0 : CONTEXT_BAR_FADE_OUT_MS}ms ease-out`}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-context-bar-full-lane={unifiedSurfaceVisible}
    data-context-bar-lane-spot
    data-context-bar-lane-surface
    data-context-bar-spot-surface-strength={laneSpotSurfaceStrength}
    data-context-bar-spot-visible={laneSpotVisible}
    data-context-bar-spot-x={laneTransientSpotX}
    data-context-bar-spot-y={laneTransientSpotY}
  >
    <div style:background={TRANSIENT_EDGE} class={css({ position: 'absolute', right: '0', bottom: '0', left: '0', height: '1px' })}></div>
  </div>

  <div
    style:background={ACTIVE_SURFACE}
    style:mask-image={engagedSpotMask()}
    style:opacity={laneEngagedSpotVisible ? '1' : '0'}
    style:transition={prefersReducedMotion.current
      ? 'none'
      : laneEngagedSpotVisible
        ? `opacity ${LANE_SPOT_ENTER_MS}ms ease-out ${laneEngagedSpotEnterDelayMs}ms`
        : `opacity ${CONTEXT_BAR_FADE_OUT_MS}ms ease-out`}
    class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-context-bar-engaged-spot
    data-context-bar-spot-visible={laneEngagedSpotVisible}
    data-context-bar-spot-x={lanePointerX}
    data-context-bar-spot-y={lanePointerY}
  ></div>

  {#each [{ id: 'leading', side: 'leading', geometry: leadingGeometry, presentation: presentation.leading }, { id: 'view-controls', side: 'trailing', geometry: viewControlsGeometry, presentation: presentation.viewControls }] as surface (surface.id)}
    {#if surface.geometry}
      {@const leading = surface.side === 'leading'}
      {@const surfaceVisible = surface.presentation.visible && surface.presentation.tone === 'engaged'}
      <div
        style:left={`${surface.geometry.left - (leading ? 0 : FADE_WIDTH)}px`}
        style:width={`${surface.geometry.width + FADE_WIDTH}px`}
        style:opacity={surfaceVisible ? '1' : '0'}
        style:background-color={toneColor(surface.presentation.tone)}
        style:mask-image={maskStops(surface.side as 'leading' | 'trailing')}
        style:transition={prefersReducedMotion.current
          ? 'none'
          : `opacity ${surfaceVisible ? CONTEXT_BAR_FADE_IN_MS : CONTEXT_BAR_FADE_OUT_MS}ms ease-out, background-color ${CONTEXT_BAR_FADE_IN_MS}ms ease-out`}
        class={css({ position: 'absolute', top: '0', bottom: '0', zIndex: '[-1]', pointerEvents: 'none' })}
        aria-hidden="true"
        data-context-bar-surface={surface.id}
      >
        <div
          style:background-color={edgeColor(surface.presentation.tone)}
          class={css({ position: 'absolute', right: '0', bottom: '0', left: '0', height: '1px' })}
        ></div>
      </div>
    {/if}
  {/each}

  <div
    style:--context-bar-segment-reveal-radius={`${laneForegroundRevealRadius}px`}
    style:--context-bar-segment-reveal-opacity={laneForegroundRevealStarted ? 1 : 0}
    style:mask-image={segmentRevealMask('leading', leadingGeometry)}
    style:transition={prefersReducedMotion.current || !laneExpansionMaskedSegments.includes('leading')
      ? 'none'
      : `--context-bar-segment-reveal-radius ${LANE_FOREGROUND_REVEAL_MS}ms ${LANE_SPOT_EXPANSION_EASING}`}
    class={css({ minWidth: '0', flex: '[0 1 auto]' })}
  >
    <EditorContextBarSegment id="leading" presentation={presentation.leading} state={leadingState} bind:element={leadingElement}>
      <div class={css({ display: 'flex', alignItems: 'center', minWidth: '0', height: '32px', paddingLeft: '16px' })}>
        <EditorContextBarPinControl onToggle={() => onPinnedChange?.(!pinned)} {pinned} />
        {#if breadcrumb}
          <span class={css({ display: 'flex', alignItems: 'center', flex: 'none', marginLeft: '8px' })} data-context-bar-pin-divider>
            <VerticalDivider style={css.raw({ height: '12px' })} />
          </span>
          <div class={css({ minWidth: '0', flex: '[0 1 auto]' })}>
            {@render breadcrumb({ state: leadingState, presentation: presentation.leading })}
          </div>
        {/if}
      </div>
    </EditorContextBarSegment>
  </div>

  <div
    style:--context-bar-segment-reveal-radius={`${laneForegroundRevealRadius}px`}
    style:--context-bar-segment-reveal-opacity={laneForegroundRevealStarted ? 1 : 0}
    style:mask-image={segmentRevealMask('viewControls', viewControlsGeometry)}
    style:transition={prefersReducedMotion.current || !laneExpansionMaskedSegments.includes('viewControls')
      ? 'none'
      : `--context-bar-segment-reveal-radius ${LANE_FOREGROUND_REVEAL_MS}ms ${LANE_SPOT_EXPANSION_EASING}`}
    class={css({ flex: 'none', marginLeft: 'auto' })}
  >
    <EditorContextBarSegment
      id="view-controls"
      interactiveWhenHidden={interactiveViewControlsWhenHidden}
      presentation={presentation.viewControls}
      state={viewControlsState}
      bind:element={viewControlsElement}
    >
      {@render viewControls({ state: viewControlsState, presentation: presentation.viewControls })}
    </EditorContextBarSegment>
  </div>
</div>

<style>
  @property --context-bar-lane-spot-radius {
    syntax: '<length>';
    inherits: true;
    initial-value: 88px;
  }

  @property --context-bar-lane-spot-opacity {
    syntax: '<number>';
    inherits: true;
    initial-value: 0;
  }

  @property --context-bar-lane-spot-surface-opacity {
    syntax: '<number>';
    inherits: true;
    initial-value: 0;
  }

  @property --context-bar-segment-reveal-radius {
    syntax: '<length>';
    inherits: true;
    initial-value: 88px;
  }

  @property --context-bar-segment-reveal-opacity {
    syntax: '<number>';
    inherits: true;
    initial-value: 0;
  }
</style>
