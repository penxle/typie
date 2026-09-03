import { hoverIntent } from '@typie/ui/actions';
import { createStableContext } from '@typie/ui/context/stable';
import { smootherstep } from '@typie/ui/utils';
import { untrack } from 'svelte';
import { PaneChromeGeometry } from './zen-mode-pane-chrome-geometry';
import type { HoverIntentParameter } from '@typie/ui/actions';
import type { ActionReturn as SvelteActionReturn } from 'svelte/action';
import type { PaneChromeLane, PaneChromeSegment, PaneChromeSegmentGeometry } from './zen-mode-pane-chrome-geometry';

export type { PaneChromeLane, PaneChromeSegment, PaneChromeSegmentGeometry } from './zen-mode-pane-chrome-geometry';
export type PaneChromePhase = 'idle' | 'pending' | 'preview' | 'spot' | 'expanding' | 'held' | 'fading';
export type PaneChromeTone = 'transient' | 'engaged';
export type PaneChromeExpansionPace = 'standard' | 'adjacent';
type PaneChromePoint = { x: number; y: number };
type PaneChromeStablePhase = Exclude<PaneChromePhase, 'spot' | 'expanding' | 'fading'>;
type PaneChromeTransition =
  | { phase: PaneChromeStablePhase }
  | { phase: 'spot'; spot: PaneChromePoint; spotLane: PaneChromeLane }
  | {
      phase: 'expanding';
      spot: PaneChromePoint;
      spotLane: PaneChromeLane;
      expansionOrigin: PaneChromePoint;
      expansionTargets: Record<PaneChromeSegment, boolean>;
      expansionPace: PaneChromeExpansionPace;
      foregroundRevealStarted: Record<PaneChromeSegment, boolean>;
    }
  | { phase: 'fading'; spot: PaneChromePoint | null; spotLane: PaneChromeLane | null };

export type PaneChromeExpansionTiming = {
  spotEnterMs: number;
  spotSurfaceMs: number;
  foregroundDelayMs: number;
  backgroundExpandMs: number;
  totalMs: number;
};

export type PaneChromeHoldHandle = {
  hold(reason: string): void;
  release(reason: string): void;
};

export type PaneChromeAttachmentHandle = {
  hold(event?: PointerEvent): void;
  release(): void;
  discoverable(): boolean;
  attached(): boolean;
  pointer(): PaneChromePoint | null;
};

type PaneChromeOptions = {
  active: () => boolean;
  focused: () => boolean;
};

type ActionReturn = { destroy(): void };
type HoverIntentActionReturn = SvelteActionReturn<HoverIntentParameter>;

const INTENT_MS = 400;
const GAP_DWELL_MS = 500;
export const PANE_CHROME_FOREGROUND_DELAY_MS = 100;
export const PANE_CHROME_FOREGROUND_FADE_IN_MS = 180;
export const PANE_CHROME_FADE_OUT_MS = 400;
export const PANE_CHROME_BACKGROUND_EXPAND_MS = 900;
export const PANE_CHROME_EXPANSION_MS = 1000;
export const PANE_CHROME_FADE_WIDTH = 24;
export const PANE_CHROME_SPOT_RADIUS = 88;
export const PANE_CHROME_SPOT_MASK_INSET = 8;
export const PANE_CHROME_SPOT_EDGE_WIDTH = 48;
export const PANE_CHROME_EXPANSION_EASING = 'cubic-bezier(0.22, 1, 0.36, 1)';
export const PANE_CHROME_SINGLE_TOOLBAR_TOP_INSET = 78;
export const PANE_CHROME_BOUNDARY = 1;
const EXIT_GRACE_MS = 1500;

const STANDARD_EXPANSION_TIMING: PaneChromeExpansionTiming = {
  spotEnterMs: 500,
  spotSurfaceMs: 600,
  foregroundDelayMs: PANE_CHROME_FOREGROUND_DELAY_MS,
  backgroundExpandMs: PANE_CHROME_BACKGROUND_EXPAND_MS,
  totalMs: PANE_CHROME_EXPANSION_MS,
};
const ADJACENT_EXPANSION_TIMING: PaneChromeExpansionTiming = {
  spotEnterMs: 120,
  spotSurfaceMs: 200,
  foregroundDelayMs: 20,
  backgroundExpandMs: 520,
  totalMs: 560,
};

export const paneChromeExpansionTiming = (pace: PaneChromeExpansionPace): PaneChromeExpansionTiming =>
  pace === 'adjacent' ? ADJACENT_EXPANSION_TIMING : STANDARD_EXPANSION_TIMING;

const segments: readonly PaneChromeSegment[] = ['identity', 'actions', 'toolbar'];

const emptyFlags = (): Record<PaneChromeSegment, boolean> => ({ identity: false, actions: false, toolbar: false });
const EMPTY_FLAGS: Readonly<Record<PaneChromeSegment, boolean>> = Object.freeze(emptyFlags());
const transientTones = (): Record<PaneChromeSegment, PaneChromeTone> => ({
  identity: 'transient',
  actions: 'transient',
  toolbar: 'transient',
});

export const paneChromeRadialMask = (x: number, y: number, radiusVariable: string, opacityVariable: string): string => {
  const edgeStops = Array.from({ length: 9 }, (_, index) => {
    const progress = index / 8;
    const alpha = 1 - smootherstep(progress);
    return `rgb(0 0 0 / calc(${alpha} * var(${opacityVariable}))) calc(var(${radiusVariable}) - ${PANE_CHROME_SPOT_MASK_INSET + PANE_CHROME_SPOT_EDGE_WIDTH * (1 - progress)}px)`;
  });
  return `radial-gradient(circle at ${x}px ${y}px, rgb(0 0 0 / var(${opacityVariable})) 0, rgb(0 0 0 / var(${opacityVariable})) calc(var(${radiusVariable}) - ${PANE_CHROME_SPOT_MASK_INSET + PANE_CHROME_SPOT_EDGE_WIDTH}px), ${edgeStops.join(', ')})`;
};

export const paneChromeRevealTargets = (zone: PaneChromeSegment | 'gap'): readonly PaneChromeSegment[] =>
  zone === 'toolbar' ? segments : zone === 'gap' ? ['identity', 'actions'] : [zone];

const [getZenModePaneChrome, setZenModePaneChrome] = createStableContext<ZenModePaneChrome>('dashboard.ZenModePaneChrome');

export { getZenModePaneChrome };

export class ZenModePaneChrome {
  #options: PaneChromeOptions;
  #active = $state(false);
  #synced = false;
  #suppressStaleHolds = false;
  #coarsePointer = false;
  #geometry: PaneChromeGeometry;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperative hold registry; presentation state is mirrored separately
  #holds = new Map<string, number>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperative MutationObserver bookkeeping
  #menuRegistrations = new Map<HTMLElement, { observer: MutationObserver; segment: PaneChromeSegment; open: boolean }>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- every consumer owns an independent attachment lifetime
  #attachmentHolds = new Set<symbol>();
  #adjacentHoverIntent: HoverIntentActionReturn | undefined;
  #adjacentZone: PaneChromeSegment | 'gap' | null = null;
  #adjacentExpansionFrame: number | undefined;
  #intentTimer: ReturnType<typeof setTimeout> | undefined;
  #dwellTimer: ReturnType<typeof setTimeout> | undefined;
  #foregroundTimer: ReturnType<typeof setTimeout> | undefined;
  #expansionTimer: ReturnType<typeof setTimeout> | undefined;
  #hideTimer: ReturnType<typeof setTimeout> | undefined;
  #fadeTimer: ReturnType<typeof setTimeout> | undefined;
  #pendingZone: PaneChromeSegment | 'gap' | null = null;
  #pointerZone: PaneChromeSegment | 'gap' | null = null;
  #lastPointer: { x: number; y: number } | null = null;
  #preparedEntry: { zone: PaneChromeSegment; point: PaneChromePoint } | null = null;
  #transition = $state.raw<PaneChromeTransition>({ phase: 'idle' });
  shown = $state.raw(emptyFlags());
  tone = $state.raw(transientTones());
  pointer = $state.raw<PaneChromePoint | null>(null);
  topOcclusion = $state(0);
  layoutRevision = $state(0);

  constructor(options: PaneChromeOptions) {
    this.#options = options;
    this.#geometry = new PaneChromeGeometry(() => {
      this.layoutRevision += 1;
      this.#measureOcclusion();
    });
  }

  get phase(): PaneChromePhase {
    return this.#transition.phase;
  }

  get foreground(): Readonly<Record<PaneChromeSegment, boolean>> {
    return this.phase === 'fading' ? EMPTY_FLAGS : this.shown;
  }

  get spot(): PaneChromePoint | null {
    return this.#transition.phase === 'spot' || this.#transition.phase === 'expanding' || this.#transition.phase === 'fading'
      ? this.#transition.spot
      : null;
  }

  get spotLane(): PaneChromeLane | null {
    return this.#transition.phase === 'spot' || this.#transition.phase === 'expanding' || this.#transition.phase === 'fading'
      ? this.#transition.spotLane
      : null;
  }

  get expansionOrigin(): PaneChromePoint | null {
    return this.#transition.phase === 'expanding' ? this.#transition.expansionOrigin : null;
  }

  get expansionTargets(): Readonly<Record<PaneChromeSegment, boolean>> {
    return this.#transition.phase === 'expanding' ? this.#transition.expansionTargets : EMPTY_FLAGS;
  }

  get expansionPace(): PaneChromeExpansionPace {
    return this.#transition.phase === 'expanding' ? this.#transition.expansionPace : 'standard';
  }

  get foregroundRevealStarted(): Readonly<Record<PaneChromeSegment, boolean>> {
    return this.#transition.phase === 'expanding' ? this.#transition.foregroundRevealStarted : EMPTY_FLAGS;
  }

  sync(): void {
    const active = this.#options.active();
    const focused = this.#options.focused();

    if (!active) {
      this.#active = false;
      this.#suppressStaleHolds = false;
      this.#preparedEntry = null;
      this.#reset();
      this.#synced = true;
      return;
    }

    const entered = this.#synced && !this.#active;
    const preparedEntry = entered ? this.#preparedEntry : null;
    if (entered) this.#preparedEntry = null;
    this.#active = true;
    this.#synced = true;
    if (entered) this.#suppressStaleHolds = true;
    this.#measureOcclusion();

    if (this.#coarsePointer) {
      if (focused) this.#revealAll(false);
      else this.#reset();
      return;
    }

    if (entered) {
      if (preparedEntry) this.#revealEntry(preparedEntry.zone, preparedEntry.point);
      else this.#revealEntryPointer();
    }
  }

  // eslint-disable-next-line unicorn/consistent-class-member-order -- bound actions follow lifecycle synchronization for readability
  registerRoot = (node: HTMLElement): ActionReturn => {
    const geometryRegistration = this.#geometry.registerRoot(node);
    const adjacentHoverIntent = hoverIntent(node, this.#adjacentHoverIntentParameter(false)) ?? undefined;
    this.#adjacentHoverIntent = adjacentHoverIntent;

    const media = window.matchMedia('(hover: none), (pointer: coarse)');
    const syncPointer = () => {
      this.#coarsePointer = media.matches;
      this.sync();
    };
    syncPointer();
    media.addEventListener('change', syncPointer);

    return {
      destroy: () => {
        media.removeEventListener('change', syncPointer);
        this.#cancelAdjacentIntent();
        adjacentHoverIntent?.destroy?.();
        geometryRegistration.destroy?.();
        if (this.#adjacentHoverIntent === adjacentHoverIntent) this.#adjacentHoverIntent = undefined;
      },
    };
  };

  registerHeaderLane = (node: HTMLElement): ActionReturn => {
    const geometryRegistration = this.#geometry.registerHeaderLane(node);
    this.#refreshPointerAfterRegistration();
    return {
      destroy: () => geometryRegistration.destroy?.(),
    };
  };

  registerToolbarLane = (node: HTMLElement): ActionReturn => {
    const geometryRegistration = this.#geometry.registerToolbarLane(node);
    this.#observeMenus(node, 'toolbar');
    this.#refreshPointerAfterRegistration();
    return {
      destroy: () => {
        geometryRegistration.destroy?.();
        this.#unobserveMenus(node);
      },
    };
  };

  registerSegment(segment: Exclude<PaneChromeSegment, 'toolbar'>, node: HTMLElement): ActionReturn {
    const geometryRegistration = this.#geometry.registerSegment(segment, node);
    this.#observeMenus(node, segment);
    this.#refreshPointerAfterRegistration();
    return {
      destroy: () => {
        geometryRegistration.destroy?.();
        this.#unobserveMenus(node);
      },
    };
  }

  handlePointerMove(event: PointerEvent): void {
    if (event.pointerType === 'touch') return;
    const excluded = event.target instanceof Element && event.target.closest('[data-pane-chrome-reveal-exclusion]') !== null;
    this.#lastPointer = excluded ? null : { x: event.clientX, y: event.clientY };
    if (!this.#active || this.#coarsePointer) return;
    if (excluded) {
      this.#pointerZone = null;
      this.pointer = null;
      this.#reconcileHoverHolds(null);
      this.#syncTones();
      this.#cancelIntent();
      if (this.#anythingShown()) this.#scheduleHide();
      return;
    }
    const zone = this.#zoneAt(event.clientX, event.clientY);
    this.#suppressStaleHolds = false;
    this.#pointerZone = zone;
    this.pointer = zone === null ? null : { x: event.clientX, y: event.clientY };
    this.#reconcileHoverHolds(zone);
    this.#syncTones();
    if (zone === null) {
      this.#cancelIntent();
      if (this.#anythingShown()) this.#scheduleHide();
      return;
    }

    this.#clearHide();
    if (this.phase === 'fading' && this.#anythingShown()) {
      this.#cancelFade();
      this.#showStablePhase('held');
      this.#measureOcclusion();
    }
    const targets = paneChromeRevealTargets(zone);
    if (this.#adjacentExpansionFrame !== undefined) {
      if (targets.every((segment) => this.shown[segment])) {
        this.#cancelIntent();
      } else {
        this.#adjacentZone = zone;
        this.#pendingZone = zone;
        this.#showSpot({ x: event.clientX, y: event.clientY }, zone === 'toolbar' ? 'toolbar' : 'header');
        this.#measureOcclusion();
      }
      return;
    }
    if (this.phase === 'spot' && this.#pendingZone === zone && (zone === 'gap' || zone === 'toolbar')) {
      this.#moveSpot({ x: event.clientX, y: event.clientY });
      return;
    }
    if (targets.every((segment) => this.shown[segment])) {
      this.#cancelIntent();
      return;
    }
    if (this.#pendingZone === zone) return;
    if (this.phase !== 'fading' && this.#anythingShown() && targets.some((segment) => !this.shown[segment])) {
      this.#startAdjacentIntent(zone);
      return;
    }
    this.#startIntent(zone, event.clientX, event.clientY);
  }

  handlePointerLeave(): void {
    this.#lastPointer = null;
    if (!this.#active || this.#coarsePointer) return;
    this.#suppressStaleHolds = false;
    this.#pointerZone = null;
    this.pointer = null;
    this.#reconcileHoverHolds(null);
    this.#syncTones();
    this.#cancelIntent();
    this.#scheduleHide();
  }

  prepareEntryReveal(segment: PaneChromeSegment, event: Pick<PointerEvent, 'clientX' | 'clientY'>): void {
    if (this.#options.active()) return;
    this.#preparedEntry = { zone: segment, point: { x: event.clientX, y: event.clientY } };
  }

  hold(segment: PaneChromeSegment, reason: string): void {
    if (!this.#active || this.#suppressStaleHolds) return;
    const key = `${segment}:${reason}`;
    this.#holds.set(key, (this.#holds.get(key) ?? 0) + 1);
    this.#clearHide();
    if (this.phase !== 'expanding') {
      if (segment === 'toolbar') this.#revealAll(true);
      else this.#reveal([segment], true);
    }
    this.#syncTones();
  }

  release(segment: PaneChromeSegment, reason: string): void {
    const key = `${segment}:${reason}`;
    const count = this.#holds.get(key) ?? 0;
    if (count <= 1) this.#holds.delete(key);
    else this.#holds.set(key, count - 1);
    this.#syncTones();
    this.#scheduleHide();
  }

  segmentHandle(segment: PaneChromeSegment): PaneChromeHoldHandle {
    return {
      hold: (reason) => this.hold(segment, reason),
      release: (reason) => this.release(segment, reason),
    };
  }

  attachmentHandle(): PaneChromeAttachmentHandle {
    const owner = Symbol('pane-chrome-attachment');
    return {
      hold: (event) => this.#holdAttachment(owner, event),
      release: () => this.#releaseAttachment(owner),
      discoverable: () => this.#attachmentDiscoverable(),
      attached: () => this.#attachmentAttached(),
      pointer: () => this.#lastPointer,
    };
  }

  isShown(segment: PaneChromeSegment): boolean {
    return this.shown[segment] || !this.#options.active();
  }

  isSurfaceVisible(segment: PaneChromeSegment): boolean {
    return (this.shown[segment] && this.phase !== 'fading') || !this.#options.active();
  }

  isForegroundVisible(segment: PaneChromeSegment): boolean {
    return this.foreground[segment] || !this.#options.active();
  }

  isForegroundRevealStarted(segment: PaneChromeSegment): boolean {
    return this.foregroundRevealStarted[segment];
  }

  isInteractive(segment: PaneChromeSegment): boolean {
    if (!this.#options.active()) return true;
    if (!this.shown[segment] || this.phase === 'fading') return false;
    return this.phase !== 'expanding' || !this.expansionTargets[segment] || this.foregroundRevealStarted[segment];
  }

  isHeaderGapInteractive(): boolean {
    if (!this.#options.active()) return true;
    return this.phase !== 'expanding' && this.phase !== 'fading' && this.shown.identity && this.shown.actions;
  }

  isHeaderLaneInteractive(): boolean {
    if (!this.#options.active()) return true;
    if (this.phase === 'fading') return false;
    if (this.phase === 'expanding' && (this.expansionTargets.identity || this.expansionTargets.actions)) {
      return (
        (this.expansionTargets.identity && this.foregroundRevealStarted.identity) ||
        (this.expansionTargets.actions && this.foregroundRevealStarted.actions)
      );
    }
    return this.isInteractive('identity') && this.isInteractive('actions');
  }

  headerLaneInteractionClip(): string {
    if (!this.#options.active()) return 'none';
    if (this.phase === 'expanding' && (this.expansionTargets.identity || this.expansionTargets.actions)) {
      const origin = this.expansionOriginInLane('header');
      if (!origin) return 'circle(0px at 0 0)';
      return `circle(${this.foregroundRevealRadius('header')}px at ${origin.x}px ${origin.y}px)`;
    }
    return this.isInteractive('identity') && this.isInteractive('actions') ? 'none' : 'circle(0px at 0 0)';
  }

  isLaneSurfaceVisible(lane: PaneChromeLane): boolean {
    if (!this.#options.active()) return true;
    if (this.phase === 'fading') return false;
    if (lane === 'toolbar') {
      return this.spotLane === 'toolbar' || this.expansionTargets.toolbar || this.shown.toolbar;
    }
    return (
      this.spotLane === 'header' ||
      this.expansionTargets.identity ||
      this.expansionTargets.actions ||
      this.shown.identity ||
      this.shown.actions
    );
  }

  spotInLane(lane: PaneChromeLane): { x: number; y: number } | null {
    void this.layoutRevision;
    if (this.spotLane !== lane) return null;
    const spot = this.spot;
    return spot ? this.#geometry.pointInLane(spot, lane) : null;
  }

  pointerInLane(lane: PaneChromeLane): { x: number; y: number } | null {
    void this.layoutRevision;
    const pointer = this.pointer;
    if (!pointer) return null;
    const pointerLane = this.#pointerZone === null ? null : this.#pointerZone === 'toolbar' ? 'toolbar' : 'header';
    if (pointerLane !== lane) return null;
    return this.#geometry.pointInLane(pointer, lane);
  }

  get floatingZoomTopInset(): number {
    void this.layoutRevision;
    return this.#geometry.floatingChromeInset();
  }

  get floatingZoomInset(): number {
    void this.layoutRevision;
    if (this.phase === 'fading') return 0;
    if (this.shown.toolbar) return this.floatingZoomTopInset;
    if (this.shown.identity || this.shown.actions) return this.topOcclusion;
    return 0;
  }

  get headerInset(): number {
    void this.layoutRevision;
    if (!this.#active || this.phase === 'fading' || (!this.shown.identity && !this.shown.actions)) return 0;
    return this.#geometry.headerInset();
  }

  expansionOriginInLane(lane: PaneChromeLane): { x: number; y: number } | null {
    void this.layoutRevision;
    const origin = this.expansionOrigin;
    return origin ? this.#geometry.pointInLane(origin, lane) : null;
  }

  laneSize(lane: PaneChromeLane): { width: number; height: number } {
    void this.layoutRevision;
    return this.#geometry.laneSize(lane);
  }

  segmentGeometry(segment: PaneChromeSegment): PaneChromeSegmentGeometry | undefined {
    void this.layoutRevision;
    return this.#geometry.segmentGeometry(segment);
  }

  foregroundMask(segment: PaneChromeSegment): string {
    const geometry = this.segmentGeometry(segment);
    const lane: PaneChromeLane = segment === 'toolbar' ? 'toolbar' : 'header';
    const origin = this.expansionOriginInLane(lane);
    if (!geometry || !origin || this.phase !== 'expanding' || !this.expansionTargets[segment]) return 'none';
    return paneChromeRadialMask(
      origin.x - geometry.left,
      origin.y,
      '--zen-pane-chrome-foreground-radius',
      '--zen-pane-chrome-foreground-opacity',
    );
  }

  foregroundClip(segment: PaneChromeSegment): string {
    const geometry = this.segmentGeometry(segment);
    const lane: PaneChromeLane = segment === 'toolbar' ? 'toolbar' : 'header';
    const origin = this.expansionOriginInLane(lane);
    if (!geometry || !origin || this.phase !== 'expanding' || !this.expansionTargets[segment]) return 'none';
    return `circle(${this.foregroundRevealRadius(lane)}px at ${origin.x - geometry.left}px ${origin.y}px)`;
  }

  foregroundRevealRadius(lane: PaneChromeLane): number {
    const { width, height } = this.laneSize(lane);
    const revealStarted =
      lane === 'toolbar'
        ? this.expansionTargets.toolbar && this.foregroundRevealStarted.toolbar
        : (this.expansionTargets.identity && this.foregroundRevealStarted.identity) ||
          (this.expansionTargets.actions && this.foregroundRevealStarted.actions);
    return revealStarted && this.phase === 'expanding'
      ? Math.hypot(width, height) + PANE_CHROME_FADE_WIDTH + PANE_CHROME_SPOT_MASK_INSET
      : PANE_CHROME_SPOT_RADIUS;
  }

  #showStablePhase(phase: PaneChromeStablePhase): void {
    this.#transition = { phase };
  }

  #showSpot(spot: PaneChromePoint, spotLane: PaneChromeLane): void {
    this.#transition = { phase: 'spot', spot, spotLane };
  }

  #moveSpot(spot: PaneChromePoint): void {
    if (this.#transition.phase !== 'spot') return;
    this.#transition = { ...this.#transition, spot };
  }

  #zoneAt(x: number, y: number): PaneChromeSegment | 'gap' | null {
    return this.#geometry.zoneAt(x, y);
  }

  #revealEntryPointer(): void {
    const point = this.#lastPointer;
    if (!point) return;
    const zone = this.#zoneAt(point.x, point.y);
    if (zone === null) return;
    this.#revealEntry(zone, point);
  }

  #revealEntry(zone: PaneChromeSegment | 'gap', point: PaneChromePoint): void {
    this.#pointerZone = zone;
    this.pointer = point;
    this.#reconcileHoverHolds(zone);
    this.#syncTones();
    this.#reveal(paneChromeRevealTargets(zone), true);
  }

  #startIntent(zone: PaneChromeSegment | 'gap', x: number, y: number): void {
    const origin = this.pointer ?? { x, y };
    if (this.phase === 'pending' && this.#intentTimer) {
      this.#pendingZone = zone;
      return;
    }
    if (this.phase === 'spot' && this.#dwellTimer) {
      this.#pendingZone = zone;
      if (zone === 'identity' || zone === 'actions') {
        clearTimeout(this.#dwellTimer);
        this.#dwellTimer = undefined;
        this.#reveal(paneChromeRevealTargets(zone), false);
      } else {
        this.#showSpot(origin, zone === 'toolbar' ? 'toolbar' : 'header');
        this.#measureOcclusion();
      }
      return;
    }

    this.#cancelIntent();
    this.#pendingZone = zone;
    this.#showStablePhase('pending');
    this.#intentTimer = setTimeout(() => {
      this.#intentTimer = undefined;
      const currentZone = this.#pendingZone;
      if (currentZone === null) return;
      if (currentZone === 'gap' || currentZone === 'toolbar') {
        const currentOrigin = this.pointer ?? origin;
        this.#showSpot(currentOrigin, currentZone === 'toolbar' ? 'toolbar' : 'header');
        this.#measureOcclusion();
        this.#dwellTimer = setTimeout(() => {
          this.#dwellTimer = undefined;
          const dwellZone = this.#pendingZone;
          if (dwellZone === null) return;
          this.#startExpansion(paneChromeRevealTargets(dwellZone));
        }, GAP_DWELL_MS);
      } else {
        this.#reveal(paneChromeRevealTargets(currentZone), false);
      }
    }, INTENT_MS);
  }

  #adjacentHoverIntentParameter(intentEnabled: boolean): HoverIntentParameter {
    return {
      delay: INTENT_MS,
      intentEnabled,
      samples: 1,
      onIntent: (event) => this.#startAdjacentSpot(event),
    };
  }

  #startAdjacentIntent(zone: PaneChromeSegment | 'gap'): void {
    this.#adjacentZone = zone;
    const intent = this.#adjacentHoverIntent;
    if (!intent?.update) {
      const pointer = this.pointer;
      if (pointer) this.#startAdjacentSpot({ clientX: pointer.x, clientY: pointer.y });
      return;
    }
    intent.update(this.#adjacentHoverIntentParameter(true));
  }

  #startAdjacentSpot(event: Pick<PointerEvent, 'clientX' | 'clientY'>): void {
    const zone = this.#adjacentZone;
    if (zone === null) return;
    const targets = paneChromeRevealTargets(zone);
    if (targets.every((segment) => this.shown[segment])) {
      this.#cancelAdjacentIntent();
      return;
    }

    this.#adjacentHoverIntent?.update?.(this.#adjacentHoverIntentParameter(false));
    this.#pendingZone = zone;
    this.#showSpot(this.pointer ?? { x: event.clientX, y: event.clientY }, zone === 'toolbar' ? 'toolbar' : 'header');
    this.#measureOcclusion();
    this.#adjacentExpansionFrame = requestAnimationFrame(() => {
      this.#adjacentExpansionFrame = undefined;
      const currentZone = this.#pendingZone;
      this.#pendingZone = null;
      this.#adjacentZone = null;
      if (currentZone === null) return;
      this.#startExpansion(paneChromeRevealTargets(currentZone), 'adjacent');
    });
  }

  #attachmentDiscoverable(): boolean {
    return this.#active && this.phase !== 'fading' && this.shown.toolbar;
  }

  #attachmentAttached(): boolean {
    return this.#active && this.phase !== 'fading' && this.floatingZoomInset > 0;
  }

  #holdAttachment(owner: symbol, event?: PointerEvent): void {
    if (!this.#active || this.#coarsePointer || event?.pointerType === 'touch') return;
    if (event) this.#lastPointer = { x: event.clientX, y: event.clientY };
    this.#attachmentHolds.add(owner);
    if (!this.#anythingShown()) return;
    this.#cancelIntent();
    if (this.phase === 'fading') this.#cancelFade();
    this.#pointerZone = null;
    this.pointer = null;
    this.#reconcileHoverHolds(null);
    this.#syncTones();
    this.#clearHide();
    if (this.phase !== 'expanding') this.#showStablePhase('held');
    this.#measureOcclusion();
  }

  #releaseAttachment(owner: symbol): void {
    if (!this.#attachmentHolds.delete(owner)) return;
    this.#pointerZone = null;
    this.pointer = null;
    this.#reconcileHoverHolds(null);
    this.#syncTones();
    this.#scheduleHide();
  }

  #revealAll(held: boolean): void {
    this.#reveal([...segments], held);
  }

  #reveal(targets: readonly PaneChromeSegment[], held: boolean): void {
    this.#cancelIntent();
    this.#cancelFade();
    this.shown = { ...this.shown, ...Object.fromEntries(targets.map((segment) => [segment, true])) };
    this.#showStablePhase(held || this.#attachmentHolds.size > 0 ? 'held' : 'preview');
    this.#syncTones();
    this.#measureOcclusion();
  }

  #startExpansion(targets: readonly PaneChromeSegment[], pace: PaneChromeExpansionPace = 'standard'): void {
    if (this.#reducedMotion()) {
      this.#reveal(targets, true);
      return;
    }
    this.#cancelFade();
    const hiddenTargets = targets.filter((segment) => !this.shown[segment]);
    if (hiddenTargets.length === 0) {
      this.#reveal(targets, true);
      return;
    }
    const targetFlags = { ...emptyFlags(), ...Object.fromEntries(hiddenTargets.map((segment) => [segment, true])) };
    const timing = paneChromeExpansionTiming(pace);
    const origin = this.spot;
    const spotLane = this.spotLane;
    if (!origin || !spotLane) {
      this.#reveal(targets, true);
      return;
    }
    this.#pendingZone = null;
    this.shown = { ...this.shown, ...Object.fromEntries(targets.map((segment) => [segment, true])) };
    this.#transition = {
      phase: 'expanding',
      spot: origin,
      spotLane,
      expansionOrigin: origin,
      expansionTargets: targetFlags,
      expansionPace: pace,
      foregroundRevealStarted: emptyFlags(),
    };
    this.#syncTones();
    this.#measureOcclusion();

    this.#foregroundTimer = setTimeout(() => {
      this.#foregroundTimer = undefined;
      if (this.#transition.phase === 'expanding') {
        this.#transition = { ...this.#transition, foregroundRevealStarted: targetFlags };
      }
    }, timing.foregroundDelayMs);
    this.#expansionTimer = setTimeout(() => {
      this.#expansionTimer = undefined;
      if (this.phase !== 'expanding') return;
      this.#showStablePhase('held');
    }, timing.totalMs);
  }

  #scheduleHide(): void {
    if (
      !this.#active ||
      this.#coarsePointer ||
      this.#holds.size > 0 ||
      this.#hasOpenMenu() ||
      this.#attachmentHolds.size > 0 ||
      !this.#anythingPresented()
    )
      return;
    if (this.#hideTimer) return;
    this.#hideTimer = setTimeout(() => {
      this.#hideTimer = undefined;
      if (this.#holds.size > 0 || this.#hasOpenMenu() || this.#attachmentHolds.size > 0) return;
      this.#beginFade();
    }, EXIT_GRACE_MS);
  }

  #beginFade(): void {
    const fadingSpot = this.spot;
    const fadingSpotLane = this.spotLane;
    this.#showStablePhase(this.#anythingShown() ? 'held' : 'idle');
    this.#cancelIntent();
    this.#transition = { phase: 'fading', spot: fadingSpot, spotLane: fadingSpotLane };
    this.#measureOcclusion();
    this.#fadeTimer = setTimeout(
      () => {
        this.#fadeTimer = undefined;
        this.shown = emptyFlags();
        this.#showStablePhase('idle');
        this.#measureOcclusion();
      },
      this.#reducedMotion() ? 0 : PANE_CHROME_FADE_OUT_MS,
    );
  }

  #observeMenus(node: HTMLElement, segment: PaneChromeSegment): void {
    const sync = () => {
      const expanded = node.querySelector('[aria-haspopup][aria-expanded="true"]') !== null;
      const registration = this.#menuRegistrations.get(node);
      const previous = registration?.open ?? false;
      if (expanded === previous) return;
      if (registration) registration.open = expanded;
      if (expanded) {
        this.#clearHide();
        if (segment === 'toolbar') this.#revealAll(true);
        else this.#reveal([segment], true);
        this.#syncTones();
      } else {
        this.#syncTones();
        this.#scheduleHide();
      }
    };
    const observer = new MutationObserver(sync);
    observer.observe(node, { subtree: true, attributes: true, attributeFilter: ['aria-expanded'] });
    this.#menuRegistrations.set(node, { observer, segment, open: false });
    sync();
  }

  #unobserveMenus(node: HTMLElement): void {
    const registration = this.#menuRegistrations.get(node);
    registration?.observer.disconnect();
    this.#menuRegistrations.delete(node);
    if (registration?.open) {
      this.#syncTones();
      this.#scheduleHide();
    }
  }

  #measureOcclusion(): void {
    if (!this.#active || this.phase === 'fading') {
      this.topOcclusion = 0;
      return;
    }
    this.topOcclusion = this.#geometry.visibleChromeInset(this.shown.identity || this.shown.actions, this.shown.toolbar);
  }

  #syncTones(): void {
    this.tone = Object.fromEntries(
      segments.map((segment) => {
        const held = [...this.#holds.keys()].some((key) => key.startsWith(`${segment}:`)) || this.#segmentHasOpenMenu(segment);
        return [segment, held || this.#pointerZone === segment ? 'engaged' : 'transient'];
      }),
    ) as Record<PaneChromeSegment, PaneChromeTone>;
  }

  #reconcileHoverHolds(zone: PaneChromeSegment | 'gap' | null): void {
    for (const segment of segments) {
      if (zone !== segment) this.#holds.delete(`${segment}:hover`);
    }
  }

  #anythingShown(): boolean {
    return segments.some((segment) => this.shown[segment]);
  }

  #anythingPresented(): boolean {
    return this.#anythingShown() || this.spot !== null;
  }

  #hasOpenMenu(): boolean {
    return [...this.#menuRegistrations.values()].some(({ open }) => open);
  }

  #segmentHasOpenMenu(segment: PaneChromeSegment): boolean {
    return [...this.#menuRegistrations.values()].some((registration) => registration.segment === segment && registration.open);
  }

  #refreshPointerAfterRegistration(): void {
    if (!this.#active || !this.#lastPointer || this.#coarsePointer) return;
    this.#revealEntryPointer();
  }

  #clearHide(): void {
    if (this.#hideTimer) clearTimeout(this.#hideTimer);
    this.#hideTimer = undefined;
  }

  #cancelIntent(): void {
    this.#cancelAdjacentIntent();
    if (this.#intentTimer) clearTimeout(this.#intentTimer);
    if (this.#dwellTimer) clearTimeout(this.#dwellTimer);
    this.#intentTimer = undefined;
    this.#dwellTimer = undefined;
    this.#pendingZone = null;
    if (this.phase === 'pending') this.#showStablePhase(this.#anythingShown() ? 'held' : 'idle');
    if (this.spot !== null && !this.#anythingShown()) this.#beginFade();
  }

  #cancelAdjacentIntent(): void {
    const hadExpansionFrame = this.#adjacentExpansionFrame !== undefined;
    if (this.#adjacentExpansionFrame !== undefined) cancelAnimationFrame(this.#adjacentExpansionFrame);
    this.#adjacentExpansionFrame = undefined;
    this.#adjacentZone = null;
    this.#adjacentHoverIntent?.update?.(this.#adjacentHoverIntentParameter(false));
    if (!hadExpansionFrame) return;
    this.#pendingZone = null;
    if (this.phase === 'spot') this.#showStablePhase(this.#anythingShown() ? 'held' : 'idle');
    this.#measureOcclusion();
  }

  #cancelFade(): void {
    this.#clearHide();
    if (this.#fadeTimer) clearTimeout(this.#fadeTimer);
    this.#fadeTimer = undefined;
    if (this.#foregroundTimer) clearTimeout(this.#foregroundTimer);
    this.#foregroundTimer = undefined;
    if (this.#expansionTimer) clearTimeout(this.#expansionTimer);
    this.#expansionTimer = undefined;
    if (this.#transition.phase === 'expanding') {
      this.#transition = { ...this.#transition, foregroundRevealStarted: emptyFlags() };
    }
  }

  #reset(): void {
    this.#cancelIntent();
    this.#cancelFade();
    if (this.#foregroundTimer) clearTimeout(this.#foregroundTimer);
    this.#foregroundTimer = undefined;
    this.#holds.clear();
    this.#attachmentHolds.clear();
    this.shown = emptyFlags();
    this.tone = transientTones();
    this.#pointerZone = null;
    this.pointer = null;
    this.#showStablePhase('idle');
    this.topOcclusion = 0;
  }

  destroy(): void {
    this.#active = false;
    this.#reset();
    for (const { observer } of this.#menuRegistrations.values()) observer.disconnect();
    this.#menuRegistrations.clear();
  }

  #reducedMotion(): boolean {
    return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  }
}

export const setupZenModePaneChrome = (options: PaneChromeOptions): ZenModePaneChrome => {
  const chrome = new ZenModePaneChrome(options);
  setZenModePaneChrome(chrome);
  chrome.sync();
  $effect(() => {
    options.active();
    options.focused();
    untrack(() => chrome.sync());
  });
  $effect(() => () => chrome.destroy());
  return chrome;
};
