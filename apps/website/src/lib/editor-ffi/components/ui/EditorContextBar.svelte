<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { hoverIntent } from '@typie/ui/actions';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { SvelteMap } from 'svelte/reactivity';
  import {
    CONTEXT_BAR_FADE_IN_MS,
    CONTEXT_BAR_FADE_OUT_MS,
    CONTEXT_BAR_TRANSIENT_VISIBLE_MS,
    ContextBarVisibilityCoordinator,
    EditorContextBarSegmentState,
    smootherstep,
  } from './editor-context-bar.svelte';
  import EditorContextBarSegment from './EditorContextBarSegment.svelte';
  import type { HoverIntentParameter } from '@typie/ui/actions';
  import type { Snippet } from 'svelte';
  import type { ActionReturn } from 'svelte/action';
  import type { ContextBarPresentation, ContextBarSegmentPresentation } from './editor-context-bar.svelte';

  export type EditorContextBarSegmentRenderProps = {
    state: EditorContextBarSegmentState;
    presentation: ContextBarSegmentPresentation;
  };

  type Props = {
    editorViewSurface: HTMLElement;
    showViewControlsOnPaneEntry: boolean;
    breadcrumb?: Snippet<[EditorContextBarSegmentRenderProps]>;
    viewControls: Snippet<[EditorContextBarSegmentRenderProps]>;
    interactiveViewControlsWhenHidden?: boolean;
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
  type ContextBarSegmentId = 'breadcrumb' | 'viewControls';

  const FADE_WIDTH = 24;
  const LANE_HOVER_INTENT_DELAY_MS = 400;
  const HIDDEN_SEGMENT_HOVER_INTENT_DELAY_MS = 400;
  const LANE_SPOT_DWELL_MS = 500;
  const LANE_SPOT_ENTER_MS = 500;
  const LANE_SPOT_SURFACE_TRANSITION_MS = 600;
  const LANE_SPOT_EXPAND_MS = 1000;
  const LANE_SPOT_RADIUS = 88;
  const LANE_SPOT_MASK_INSET = 8;
  const LANE_SPOT_EDGE_WIDTH = 48;
  const LANE_SPOT_SURFACE_STRENGTH = 0.7;
  const LANE_SPOT_EXPANSION_EASING = 'cubic-bezier(0.22, 1, 0.36, 1)';
  const LANE_HOLD_REASON = 'lane-hover';
  const ACTIVE_SURFACE = token('colors.surface.muted');
  const TRANSIENT_BASE = token('colors.surface.subtle');
  const SUBTLE_BORDER = token('colors.border.subtle');
  const DEFAULT_BORDER = token('colors.border.default');
  const TRANSIENT_SURFACE = `color-mix(in srgb, ${TRANSIENT_BASE} 80%, transparent)`;
  const TRANSIENT_EDGE = `color-mix(in srgb, ${TRANSIENT_BASE} 40%, ${SUBTLE_BORDER} 60%)`;
  const ENGAGED_EDGE = `color-mix(in srgb, ${ACTIVE_SURFACE} 36%, ${DEFAULT_BORDER} 64%)`;

  let {
    editorViewSurface,
    showViewControlsOnPaneEntry,
    breadcrumb,
    viewControls,
    interactiveViewControlsWhenHidden = false,
  }: Props = $props();

  const breadcrumbState = new EditorContextBarSegmentState();
  const viewControlsState = new EditorContextBarSegmentState();
  const visibilityCoordinator = new ContextBarVisibilityCoordinator();
  let lane = $state<HTMLElement>();
  let breadcrumbElement = $state<HTMLElement>();
  let viewControlsElement = $state<HTMLElement>();
  let breadcrumbGeometry = $state<SegmentGeometry>();
  let viewControlsGeometry = $state<SegmentGeometry>();
  let laneWidth = $state(0);
  let laneHeight = $state(0);
  let laneHoverPhase = $state<LaneHoverPhase>('idle');
  let pointerInLaneGap = $state(false);
  let lanePointerX = $state(0);
  let lanePointerY = $state(0);
  let laneExpansionMaskedSegments = $state<ContextBarSegmentId[]>([]);
  let fadingTransientSegments = $state<ContextBarSegmentId[]>([]);
  let laneHoverIntent: ActionReturn<HoverIntentParameter> | undefined;
  let laneDwellTimer: ReturnType<typeof setTimeout> | undefined;
  let laneExpansionTimer: ReturnType<typeof setTimeout> | undefined;
  let hiddenSegmentIntentTimer: ReturnType<typeof setTimeout> | undefined;
  let hiddenSegmentIntentTarget: ContextBarSegmentId | undefined;
  let hiddenSegmentEngaged: ContextBarSegmentId | undefined;
  const fadingTransientTimers = new SvelteMap<ContextBarSegmentId, ReturnType<typeof setTimeout>>();
  let initialRevealComplete = false;

  let presentation = $state<ContextBarPresentation>({
    unified: false,
    breadcrumb: { visible: false, tone: 'transient' },
    viewControls: { visible: false, tone: 'transient' },
  });
  let previousPresentation = presentation;

  const unifiedPairVisible = $derived(presentation.unified);
  const unifiedSurfaceVisible = $derived(unifiedPairVisible && laneHoverPhase !== 'expanding');
  const laneSpotVisible = $derived(laneHoverPhase === 'preview' || laneHoverPhase === 'spot' || laneHoverPhase === 'expanding');
  const laneSpotSurfaceStrength = $derived(
    laneHoverPhase === 'expanding' ? 1 : laneHoverPhase === 'preview' || laneHoverPhase === 'spot' ? LANE_SPOT_SURFACE_STRENGTH : 0,
  );
  const laneSpotOpacityTransitionMs = $derived(unifiedSurfaceVisible ? 0 : LANE_SPOT_ENTER_MS);
  const laneSpotSurfaceTransitionMs = $derived(
    unifiedSurfaceVisible
      ? 0
      : laneHoverPhase === 'expanding'
        ? LANE_SPOT_SURFACE_TRANSITION_MS
        : laneHoverPhase === 'preview' || laneHoverPhase === 'spot'
          ? LANE_SPOT_ENTER_MS
          : CONTEXT_BAR_FADE_OUT_MS,
  );
  const laneSpotRadius = $derived(
    laneHoverPhase === 'expanding' ? Math.hypot(laneWidth, laneHeight) + FADE_WIDTH + LANE_SPOT_MASK_INSET : LANE_SPOT_RADIUS,
  );
  const laneBlurMask = $derived.by(() => laneBlurMasks().join(', '));
  const laneSurfaceMask = $derived.by(() => laneSurfaceMasks().join(', '));
  const laneSurfaceMaskComposite = $derived(laneHoverPhase === 'preview' && fadingTransientSegments.length === 0 ? 'intersect' : 'add');
  const laneBlurVisible = $derived(
    laneSpotVisible ||
      fadingTransientSegments.length > 0 ||
      (unifiedSurfaceVisible && (presentation.breadcrumb.tone === 'transient' || presentation.viewControls.tone === 'transient')) ||
      (!unifiedPairVisible &&
        ((presentation.breadcrumb.visible && presentation.breadcrumb.tone === 'transient') ||
          (presentation.viewControls.visible && presentation.viewControls.tone === 'transient'))),
  );
  const hasVisibleSegment = $derived(presentation.breadcrumb.visible || presentation.viewControls.visible);
  const laneBlurRadius = $derived.by(() => {
    if (!laneBlurVisible) return 0;
    if (!hasVisibleSegment && (laneHoverPhase === 'preview' || laneHoverPhase === 'spot')) return 1.5;
    return 3;
  });
  const laneBlurTransitionMs = $derived(
    laneHoverPhase === 'expanding'
      ? LANE_SPOT_EXPAND_MS
      : laneHoverPhase === 'preview' || laneHoverPhase === 'spot'
        ? LANE_SPOT_ENTER_MS
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

  function radialMask(x: number, y: number, opacityVariable = '--context-bar-lane-spot-opacity'): string {
    const edgeStops = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = 1 - smootherstep(progress);
      return `rgb(0 0 0 / calc(${alpha} * var(${opacityVariable}))) calc(var(--context-bar-lane-spot-radius) - ${LANE_SPOT_MASK_INSET + LANE_SPOT_EDGE_WIDTH * (1 - progress)}px)`;
    });
    return `radial-gradient(circle at ${x}px ${y}px, rgb(0 0 0 / var(${opacityVariable})) 0, rgb(0 0 0 / var(${opacityVariable})) calc(var(--context-bar-lane-spot-radius) - ${LANE_SPOT_MASK_INSET + LANE_SPOT_EDGE_WIDTH}px), ${edgeStops.join(', ')})`;
  }

  function laneSpotMask(opacityVariable: string): string {
    return radialMask(lanePointerX, lanePointerY, opacityVariable);
  }

  function unifiedTransientMask(): string {
    if (!breadcrumbGeometry || !viewControlsGeometry) return 'linear-gradient(transparent, transparent)';
    const hasTransientSegment =
      presentation.breadcrumb.tone === 'transient' || presentation.viewControls.tone === 'transient' || fadingTransientSegments.length > 0;
    if (hasTransientSegment) return 'linear-gradient(black, black)';

    const breadcrumbEngaged = presentation.breadcrumb.tone === 'engaged' && !fadingTransientSegments.includes('breadcrumb');
    const viewControlsEngaged = presentation.viewControls.tone === 'engaged' && !fadingTransientSegments.includes('viewControls');
    if (!breadcrumbEngaged && !viewControlsEngaged) return 'linear-gradient(black, black)';

    const width = viewControlsGeometry.right - breadcrumbGeometry.left;
    const breadcrumbRight = breadcrumbGeometry.right - breadcrumbGeometry.left;
    const viewControlsLeft = viewControlsGeometry.left - breadcrumbGeometry.left;
    const positions = [0, width];
    const addPosition = (position: number) => {
      if (!positions.includes(position)) positions.push(position);
    };
    if (breadcrumbEngaged) {
      for (let index = 0; index <= 8; index += 1) addPosition(Math.min(width, breadcrumbRight + (FADE_WIDTH * index) / 8));
    }
    if (viewControlsEngaged) {
      for (let index = 0; index <= 8; index += 1) addPosition(Math.max(0, viewControlsLeft - FADE_WIDTH + (FADE_WIDTH * index) / 8));
    }

    const stops = positions
      .toSorted((left, right) => left - right)
      .map((position) => {
        const breadcrumbAlpha = breadcrumbEngaged ? smootherstep((position - breadcrumbRight) / FADE_WIDTH) : 1;
        const viewControlsAlpha = viewControlsEngaged ? 1 - smootherstep((position - (viewControlsLeft - FADE_WIDTH)) / FADE_WIDTH) : 1;
        return `rgb(0 0 0 / ${Math.min(breadcrumbAlpha, viewControlsAlpha)}) ${position}px`;
      });
    return `linear-gradient(to right, ${stops.join(', ')})`;
  }

  function segmentLaneMask(segment: ContextBarSegmentId, transientOnly: boolean): string | undefined {
    const geometry = segment === 'breadcrumb' ? breadcrumbGeometry : viewControlsGeometry;
    const segmentPresentation = presentation[segment];
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
      const alpha = segment === 'breadcrumb' ? 1 - smootherstep(progress) : smootherstep(progress);
      const position = segment === 'breadcrumb' ? geometry.right + FADE_WIDTH * progress : geometry.left - FADE_WIDTH * (1 - progress);
      return `rgb(0 0 0 / ${alpha}) ${position}px`;
    });
    if (segment === 'breadcrumb') samples.push('transparent 100%');
    else samples.unshift('transparent 0');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function loneEngagedSegment(): ContextBarSegmentId | undefined {
    const visibleSegments = (['breadcrumb', 'viewControls'] as const).filter((segment) => presentation[segment].visible);
    const segment = visibleSegments.length === 1 ? visibleSegments[0] : undefined;
    if (!segment) return;
    const state = segmentState(segment);
    return state.hovered || state.focused ? segment : undefined;
  }

  function outsideEngagedSegmentMask(segment: ContextBarSegmentId): string {
    const geometry = segment === 'breadcrumb' ? breadcrumbGeometry : viewControlsGeometry;
    if (!geometry) return 'linear-gradient(transparent, transparent)';
    const samples = Array.from({ length: 9 }, (_, index) => {
      const progress = index / 8;
      const alpha = segment === 'breadcrumb' ? smootherstep(progress) : 1 - smootherstep(progress);
      const position = segment === 'breadcrumb' ? geometry.right + FADE_WIDTH * progress : geometry.left - FADE_WIDTH * (1 - progress);
      return `rgb(0 0 0 / ${alpha}) ${position}px`;
    });
    if (segment === 'breadcrumb') samples.push('black 100%');
    else samples.unshift('black 0');
    return `linear-gradient(to right, ${samples.join(', ')})`;
  }

  function laneBlurMasks(): string[] {
    const masks = [laneSpotMask('--context-bar-lane-spot-opacity')];
    const hasTransientSegment =
      presentation.breadcrumb.tone === 'transient' || presentation.viewControls.tone === 'transient' || fadingTransientSegments.length > 0;
    if (unifiedSurfaceVisible) {
      if (hasTransientSegment) masks.push('linear-gradient(black, black)');
      return masks;
    }

    const includeEngagedSegments = laneSpotVisible;
    const breadcrumbMask = segmentLaneMask('breadcrumb', !includeEngagedSegments);
    const viewControlsMask = segmentLaneMask('viewControls', !includeEngagedSegments);
    if (breadcrumbMask) masks.push(breadcrumbMask);
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
    if (unifiedSurfaceVisible) {
      masks.push(unifiedTransientMask());
      return masks;
    }
    const breadcrumbMask = segmentLaneMask('breadcrumb', true);
    const viewControlsMask = segmentLaneMask('viewControls', true);
    if (breadcrumbMask) masks.push(breadcrumbMask);
    if (viewControlsMask) masks.push(viewControlsMask);
    return masks;
  }

  function segmentRevealMask(segment: ContextBarSegmentId, geometry: SegmentGeometry | undefined): string {
    if (!geometry || laneHoverPhase !== 'expanding' || !laneExpansionMaskedSegments.includes(segment)) return 'none';
    return radialMask(lanePointerX - geometry.left, lanePointerY);
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
    breadcrumbGeometry = breadcrumbElement ? relativeGeometry(breadcrumbElement) : undefined;
    viewControlsGeometry = relativeGeometry(viewControlsElement);
  }

  function segmentState(segment: ContextBarSegmentId): EditorContextBarSegmentState {
    return segment === 'breadcrumb' ? breadcrumbState : viewControlsState;
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
    if (geometryContainsPoint(breadcrumbGeometry, point)) return 'breadcrumb';
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
    clearTimeout(laneExpansionTimer);
    setLaneHoverIntentEnabled(false);
    laneDwellTimer = undefined;
    laneExpansionTimer = undefined;
  }

  function holdLane(): void {
    clearLaneTimers();
    laneHoverPhase = 'held';
    laneExpansionMaskedSegments = [];
    breadcrumbState.hold(LANE_HOLD_REASON);
    viewControlsState.hold(LANE_HOLD_REASON);
  }

  function stopLaneInteraction(linger: boolean): void {
    const wasHeld = laneHoverPhase === 'held';
    pointerInLaneGap = false;
    clearLaneTimers();
    laneHoverPhase = 'idle';
    laneExpansionMaskedSegments = [];
    breadcrumbState.release(LANE_HOLD_REASON);
    viewControlsState.release(LANE_HOLD_REASON);
    if (!wasHeld || !linger) return;
    breadcrumbState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    viewControlsState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
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
    laneExpansionMaskedSegments = (['breadcrumb', 'viewControls'] as const).filter((segment) => !presentation[segment].visible);
    laneHoverPhase = 'expanding';
    breadcrumbState.hold(LANE_HOLD_REASON);
    viewControlsState.hold(LANE_HOLD_REASON);
    laneExpansionTimer = setTimeout(() => {
      laneExpansionTimer = undefined;
      holdLane();
    }, LANE_SPOT_EXPAND_MS);
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
    if (!breadcrumbGeometry || !viewControlsGeometry || viewControlsGeometry.left <= breadcrumbGeometry.right) return false;
    return point.x > breadcrumbGeometry.right && point.x < viewControlsGeometry.left;
  }

  function handlePanePointerMove(event: PointerEvent): void {
    if (!breadcrumbElement || event.pointerType === 'touch') {
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

    if (isLaneGap(point)) {
      pointerInLaneGap = true;
      lanePointerX = point.x;
      lanePointerY = point.y;
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
        if (presentation.breadcrumb.visible || presentation.viewControls.visible) startLaneSpot();
        else startLaneIntent();
      }
      return;
    }

    const segment = segmentAtPoint(point);
    if (segment && loneEngagedSegment() === segment) {
      pointerInLaneGap = false;
      lanePointerX = point.x;
      lanePointerY = point.y;
      if (laneHoverPhase !== 'preview') startLanePreview();
      return;
    }

    stopLaneInteraction(true);
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

    const sibling: ContextBarSegmentId = segment === 'breadcrumb' ? 'viewControls' : 'breadcrumb';
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
    const nextPresentation = visibilityCoordinator.resolve({
      breadcrumb: breadcrumb ? breadcrumbState.activity : { transient: false, hovered: false, focused: false, holds: [] },
      viewControls: viewControlsState.activity,
    });
    for (const segment of ['breadcrumb', 'viewControls'] as const) {
      if (
        previousPresentation[segment].visible &&
        previousPresentation[segment].tone === 'transient' &&
        nextPresentation[segment].visible &&
        nextPresentation[segment].tone === 'engaged'
      ) {
        retainTransientDuringEngagement(segment);
      }
    }
    previousPresentation = nextPresentation;
    presentation = nextPresentation;
  });

  $effect(() => {
    if (laneHoverPhase !== 'expanding' && laneHoverPhase !== 'held' && pointerInLaneGap && presentation.unified) holdLane();
  });

  $effect(() => {
    const currentLane = lane;
    const currentBreadcrumb = breadcrumbElement;
    const currentViewControls = viewControlsElement;
    if (!currentLane || !currentViewControls) return;
    const observer = new ResizeObserver(syncGeometry);
    observer.observe(currentLane);
    observer.observe(currentViewControls);
    if (currentBreadcrumb) observer.observe(currentBreadcrumb);
    syncGeometry();
    return () => observer.disconnect();
  });

  $effect(() => {
    const breadcrumbReady = !breadcrumb || (breadcrumbGeometry?.width ?? 0) > 0;
    if (initialRevealComplete || !breadcrumbReady || (viewControlsGeometry?.width ?? 0) <= 0) return;
    initialRevealComplete = true;
    if (breadcrumb) breadcrumbState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    viewControlsState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
  });

  $effect(() => {
    const target = editorViewSurface.closest<HTMLElement>('[data-pane-id]') ?? editorViewSurface;
    const currentLaneHoverIntent = hoverIntent(target, laneHoverIntentParameter(false)) ?? undefined;
    laneHoverIntent = currentLaneHoverIntent;
    const handlePointerEnter = (event: PointerEvent) => {
      if (event.pointerType === 'touch') return;
      if (breadcrumb) breadcrumbState.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
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
    return () => {
      clearHiddenSegmentIntent();
      clearFadingTransientSegments();
      stopLaneInteraction(false);
      breadcrumbState.destroy();
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
    : `--context-bar-lane-spot-opacity ${laneSpotOpacityTransitionMs}ms ease-out, --context-bar-lane-spot-surface-opacity ${laneSpotSurfaceTransitionMs}ms ease-out, --context-bar-lane-spot-radius ${LANE_SPOT_EXPAND_MS}ms ${LANE_SPOT_EXPANSION_EASING}`}
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
    class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
    aria-hidden="true"
    data-context-bar-full-lane={unifiedSurfaceVisible}
    data-context-bar-lane-spot
    data-context-bar-lane-surface
    data-context-bar-spot-surface-strength={laneSpotSurfaceStrength}
    data-context-bar-spot-visible={laneSpotVisible}
    data-context-bar-spot-x={lanePointerX}
    data-context-bar-spot-y={lanePointerY}
  >
    <div style:background={TRANSIENT_EDGE} class={css({ position: 'absolute', right: '0', bottom: '0', left: '0', height: '1px' })}></div>
  </div>

  {#if breadcrumb}
    <div style:mask-image={segmentRevealMask('breadcrumb', breadcrumbGeometry)} class={css({ minWidth: '0', flex: '[0 1 auto]' })}>
      <EditorContextBarSegment
        id="breadcrumb"
        presentation={presentation.breadcrumb}
        state={breadcrumbState}
        bind:element={breadcrumbElement}
      >
        {@render breadcrumb({ state: breadcrumbState, presentation: presentation.breadcrumb })}
      </EditorContextBarSegment>
    </div>
  {/if}

  <div style:mask-image={segmentRevealMask('viewControls', viewControlsGeometry)} class={css({ flex: 'none', marginLeft: 'auto' })}>
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

  {#each [{ id: 'breadcrumb', side: 'leading', geometry: breadcrumbGeometry, presentation: presentation.breadcrumb }, { id: 'view-controls', side: 'trailing', geometry: viewControlsGeometry, presentation: presentation.viewControls }] as surface (surface.id)}
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
</style>
