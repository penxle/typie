import { tick } from 'svelte';
import {
  clampDocumentLayoutZoom,
  clampDocumentZoom,
  computeDocumentFitWidthZoom,
  computeDocumentZoomBounds,
  computeInitialDocumentZoom,
  RENDER_ZOOM_DEBOUNCE_MS,
  RENDER_ZOOM_MAX_COMMIT_DELAY_MS,
  RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS,
  RENDER_ZOOM_SCALE_RATIO_THRESHOLD,
  renderZoomForDisplay,
  resolveDirectDocumentZoom,
  zoomDiffers,
  zoomEquals,
} from '$lib/editor-ffi/zoom';
import { ZOOM_MAX_MOTION_SECONDS, ZoomMotion } from '$lib/editor-ffi/zoom-motion';
import type { ScrollViewport } from '@typie/ui/utils';
import type { Editor } from '$lib/editor-ffi/editor.svelte';
import type { DocumentZoomLayout } from '$lib/editor-ffi/zoom';

type ZoomAnchor = { page: number; x: number; y: number; focalX: number; focalY: number };
type DirectZoomKind = 'touch' | 'wheel';
type ZoomMotionPlayback = 'animated' | 'immediate';

type DirectZoomSession = {
  kind: DirectZoomKind;
  layoutKey: string;
  anchor: ZoomAnchor | null;
  rawZoom: number;
  snapZoom: number | null;
  lastTimestampMs: number;
};

type ActiveZoomMotion = {
  layoutKey: string;
  anchor: ZoomAnchor | null;
  motion: ZoomMotion;
  lastTimestampMs: number;
};

const NOMINAL_MOTION_FRAME_MS = 1000 / 60;
const DIRECT_SNAP_VELOCITY_THRESHOLD = 0.18;

type EditorZoomControllerOptions = {
  editor: Editor;
  layout: () => DocumentZoomLayout | null;
  viewportWidth: () => number;
  getScrollViewport: () => ScrollViewport | null | undefined;
  attachViewportAnchor?: (point: Pick<ZoomAnchor, 'page' | 'x' | 'y'>) => void;
};

export class EditorZoomController {
  static readonly WHEEL_RAW_ZOOM_RESET_MS = 32;
  static readonly KEYBOARD_ZOOM_STEP = 0.1;

  #initializedLayoutKey: string | null = null;
  #renderZoomTimer: ReturnType<typeof setTimeout> | null = null;
  #renderZoomMismatchStartedAt: number | null = null;
  #lastRenderZoomCommitAt = -Infinity;
  #wheelReleaseTimer: ReturnType<typeof setTimeout> | null = null;
  #directSession: DirectZoomSession | null = null;
  #activeMotion: ActiveZoomMotion | null = null;
  #cancelAnimationFrame: (() => void) | null = null;
  #applyQueue: Promise<unknown> = Promise.resolve();
  #options: EditorZoomControllerOptions;

  displayZoom = $state(1);
  renderZoom = $state(1);

  constructor(options: EditorZoomControllerOptions) {
    this.#options = options;
  }

  async #stepZoomByKeyboard(delta: number): Promise<void> {
    if (!this.#options.layout()) return;
    await this.#setZoomWithAnchor(this.displayZoom + delta, this.#createZoomAnchorFromViewportCenter());
  }

  #createZoomAnchorFromClient(clientX: number, clientY: number): ZoomAnchor | null {
    const viewport = this.#options.getScrollViewport();
    if (!viewport) return null;
    const resolved = this.#options.editor.clientToLocal(clientX, clientY);
    if (!resolved) return null;
    const rect = viewport.getRect();
    return { ...resolved, focalX: clientX - rect.left, focalY: clientY - rect.top };
  }

  #createZoomAnchorFromViewportCenter(): ZoomAnchor | null {
    const viewport = this.#options.getScrollViewport();
    if (!viewport) return null;
    const rect = viewport.getRect();
    return this.#createZoomAnchorFromClient(rect.left + (rect.right - rect.left) / 2, rect.top + (rect.bottom - rect.top) / 2);
  }

  #resolveFocalInViewport(clientX: number, clientY: number): { x: number; y: number } | null {
    const rect = this.#options.getScrollViewport()?.getRect();
    const x = rect ? clientX - rect.left : clientX;
    const y = rect ? clientY - rect.top : clientY;
    return Number.isFinite(x) && Number.isFinite(y) ? { x, y } : null;
  }

  async #setZoomWithAnchor(nextZoom: number, anchor: ZoomAnchor | null): Promise<void> {
    this.setZoom(nextZoom);
    await this.#syncZoomAnchor(anchor, this.displayZoom);
  }

  #clearWheelReleaseTimer(): void {
    if (this.#wheelReleaseTimer) {
      clearTimeout(this.#wheelReleaseTimer);
      this.#wheelReleaseTimer = null;
    }
  }

  #scheduleWheelRelease(): void {
    this.#clearWheelReleaseTimer();
    this.#wheelReleaseTimer = setTimeout(() => {
      this.#wheelReleaseTimer = null;
      void this.releaseDirectZoom('wheel');
    }, EditorZoomController.WHEEL_RAW_ZOOM_RESET_MS);
  }

  #clearRenderZoomTimer(): void {
    if (this.#renderZoomTimer) {
      clearTimeout(this.#renderZoomTimer);
      this.#renderZoomTimer = null;
    }
  }

  #commitLatestRenderZoom(now: number): void {
    this.#clearRenderZoomTimer();
    this.#renderZoomMismatchStartedAt = null;
    const nextRenderZoom = this.#options.layout() ? renderZoomForDisplay(this.displayZoom) : 1;
    if (zoomDiffers(this.renderZoom, nextRenderZoom)) {
      this.renderZoom = nextRenderZoom;
      this.#lastRenderZoomCommitAt = now;
    }
  }

  #scheduleRenderZoom(final = false): void {
    this.#clearRenderZoomTimer();
    const nextRenderZoom = this.#options.layout() ? renderZoomForDisplay(this.displayZoom) : 1;
    if (!zoomDiffers(this.renderZoom, nextRenderZoom)) {
      this.#renderZoomMismatchStartedAt = null;
      return;
    }
    const now = Date.now();
    this.#renderZoomMismatchStartedAt ??= now;
    const minimumIntervalDeadline = this.#lastRenderZoomCommitAt + RENDER_ZOOM_MIN_COMMIT_INTERVAL_MS;
    const quietDeadline = now + RENDER_ZOOM_DEBOUNCE_MS;
    const maximumDelayDeadline = this.#renderZoomMismatchStartedAt + RENDER_ZOOM_MAX_COMMIT_DELAY_MS;
    const scaleRatio = Math.max(this.displayZoom / this.renderZoom, this.renderZoom / this.displayZoom);
    const requestedDeadline =
      final || scaleRatio >= RENDER_ZOOM_SCALE_RATIO_THRESHOLD ? now : Math.min(quietDeadline, maximumDelayDeadline);
    const deadline = Math.max(requestedDeadline, minimumIntervalDeadline);
    const delay = Math.max(0, deadline - now);
    if (delay === 0) {
      this.#commitLatestRenderZoom(now);
    } else {
      this.#renderZoomTimer = setTimeout(() => this.#commitLatestRenderZoom(Date.now()), delay);
    }
  }

  async #syncZoomAnchor(anchor: ZoomAnchor | null, zoom: number, isCurrent: () => boolean = () => true): Promise<boolean> {
    if (!anchor) return isCurrent();
    const viewport = this.#options.getScrollViewport();
    if (!viewport) return false;
    const pageCount = this.#options.editor.pageSizes.length;
    if (pageCount === 0 || anchor.page < 0 || anchor.page >= pageCount) return false;
    await tick();
    if (!isCurrent()) return false;
    const pageEl = this.#options.editor.pageEls[anchor.page];
    if (!pageEl) return false;
    const pageRect = pageEl.getBoundingClientRect();
    const scrollRect = viewport.getRect();
    const deltaX = pageRect.left + anchor.x * zoom - (scrollRect.left + anchor.focalX);
    const deltaY = pageRect.top + anchor.y * zoom - (scrollRect.top + anchor.focalY);
    if (![deltaX, deltaY].every(Number.isFinite)) return false;
    if (Math.abs(deltaX) > 0.5 || Math.abs(deltaY) > 0.5) viewport.scrollBy(deltaX, deltaY);
    this.#options.attachViewportAnchor?.(anchor);
    return true;
  }

  #enqueueResolvedZoom(zoom: number, resolveAnchor: () => ZoomAnchor | null, isCurrent: () => boolean): Promise<boolean> {
    const apply = async (): Promise<boolean> => {
      if (!isCurrent()) return false;
      if (zoomDiffers(this.displayZoom, zoom)) this.displayZoom = zoom;
      this.#scheduleRenderZoom();
      return this.#syncZoomAnchor(resolveAnchor(), zoom, isCurrent);
    };
    const result = this.#applyQueue.then(apply);
    this.#applyQueue = result.catch(() => false);
    return result;
  }

  #stopMotion(): void {
    this.#cancelAnimationFrame?.();
    this.#cancelAnimationFrame = null;
    this.#activeMotion = null;
  }

  #cancelInteractiveMotion(): void {
    this.#clearWheelReleaseTimer();
    this.#directSession = null;
    this.#stopMotion();
  }

  async #finishOrRecover(session: DirectZoomSession, playback: ZoomMotionPlayback = 'animated'): Promise<void> {
    const layout = this.#options.layout();
    if (!layout || layoutKey(layout) !== session.layoutKey) {
      this.#scheduleRenderZoom(true);
      return;
    }
    const bounds = computeDocumentZoomBounds(layout);
    if (this.displayZoom >= bounds.min && this.displayZoom <= bounds.max) {
      this.#scheduleRenderZoom(true);
      return;
    }
    const motion = new ZoomMotion(this.displayZoom, bounds);
    const anchor = session.anchor ? { ...session.anchor } : null;
    const active: ActiveZoomMotion = {
      layoutKey: session.layoutKey,
      anchor,
      motion,
      lastTimestampMs: nowMilliseconds() - NOMINAL_MOTION_FRAME_MS,
    };
    this.#activeMotion = active;
    if (playback === 'immediate') {
      const frame = motion.advance(ZOOM_MAX_MOTION_SECONDS);
      await this.#enqueueResolvedZoom(
        frame.displayZoom,
        () => active.anchor,
        () => this.#activeMotion === active,
      );
      this.#finishMotion(active);
    } else {
      this.#scheduleMotionFrame();
    }
  }

  #scheduleMotionFrame(): void {
    this.#cancelAnimationFrame?.();
    this.#cancelAnimationFrame = requestZoomAnimationFrame((timestampMs) => {
      this.#cancelAnimationFrame = null;
      void this.#advanceMotion(timestampMs);
    });
  }

  async #advanceMotion(timestampMs: number): Promise<void> {
    const active = this.#activeMotion;
    if (!active) return;
    const layout = this.#options.layout();
    if (!layout || layoutKey(layout) !== active.layoutKey) {
      this.#finishMotion(active);
      return;
    }
    const frame = active.motion.advance(Math.max(0, timestampMs - active.lastTimestampMs) / 1000);
    active.lastTimestampMs = timestampMs;
    const applied = await this.#enqueueResolvedZoom(
      frame.displayZoom,
      () => active.anchor,
      () => this.#activeMotion === active,
    );
    if (!applied || this.#activeMotion !== active || frame.finished) {
      this.#finishMotion(active);
    } else {
      this.#scheduleMotionFrame();
    }
  }

  #finishMotion(active: ActiveZoomMotion): void {
    if (this.#activeMotion !== active) return;
    this.#stopMotion();
    this.#scheduleRenderZoom(true);
  }

  get hasActiveDirectZoom(): boolean {
    return this.#directSession !== null;
  }

  get hasActiveMotion(): boolean {
    return this.#activeMotion !== null;
  }

  destroy(): void {
    this.#cancelInteractiveMotion();
    this.#clearRenderZoomTimer();
    this.#renderZoomMismatchStartedAt = null;
  }

  setZoom(nextZoom: number, { commitRender = false } = {}): void {
    this.#cancelInteractiveMotion();
    this.#clearRenderZoomTimer();
    const layout = this.#options.layout();
    if (!layout) {
      if (zoomDiffers(this.displayZoom, 1)) this.displayZoom = 1;
      if (zoomDiffers(this.renderZoom, 1)) {
        this.renderZoom = 1;
        this.#lastRenderZoomCommitAt = Date.now();
      }
      this.#renderZoomMismatchStartedAt = null;
      return;
    }
    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : 1;
    const clamped = clampDocumentLayoutZoom({ zoom: nextZoom, layout, viewportWidth });
    if (zoomDiffers(this.displayZoom, clamped)) this.displayZoom = clamped;
    if (commitRender) this.#commitLatestRenderZoom(Date.now());
    else this.#scheduleRenderZoom();
  }

  syncInitialZoom(): void {
    const layout = this.#options.layout();
    const viewportWidth = this.#options.viewportWidth();
    if (!layout) {
      this.#initializedLayoutKey = null;
      this.setZoom(1, { commitRender: true });
      return;
    }
    if (viewportWidth <= 0) return;
    const key = layoutKey(layout);
    if (this.#initializedLayoutKey === key) return;
    this.#initializedLayoutKey = key;
    this.setZoom(computeInitialDocumentZoom(layout, viewportWidth), { commitRender: true });
  }

  clampCurrentZoomToBounds(): void {
    if (this.hasActiveDirectZoom || this.hasActiveMotion) return;
    const layout = this.#options.layout();
    if (!layout) return;
    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : 1;
    const clamped = clampDocumentLayoutZoom({ zoom: this.displayZoom, layout, viewportWidth });
    if (zoomDiffers(clamped, this.displayZoom)) this.setZoom(clamped, { commitRender: true });
  }

  beginDirectZoom(kind: DirectZoomKind, clientX: number, clientY: number, timestampMs: number): boolean {
    const layout = this.#options.layout();
    const focal = this.#resolveFocalInViewport(clientX, clientY);
    if (!layout || !focal) return false;
    this.#cancelInteractiveMotion();
    this.#directSession = {
      kind,
      layoutKey: layoutKey(layout),
      anchor: this.#createZoomAnchorFromClient(clientX, clientY),
      rawZoom: this.displayZoom,
      snapZoom: resolveDirectSnapZoom(this.displayZoom, layout, this.#options.viewportWidth()),
      lastTimestampMs: normalizeTimestamp(timestampMs),
    };
    return true;
  }

  async updateDirectZoom(kind: DirectZoomKind, rawZoom: number, clientX: number, clientY: number, timestampMs: number): Promise<boolean> {
    const session = this.#directSession;
    const layout = this.#options.layout();
    const focal = this.#resolveFocalInViewport(clientX, clientY);
    if (!layout || !focal || !session || session.kind !== kind || layoutKey(layout) !== session.layoutKey) return false;
    const normalizedTimestamp = normalizeTimestamp(timestampMs);
    const rawVelocity = instantaneousLogZoomVelocity(session.rawZoom, rawZoom, session.lastTimestampMs, normalizedTimestamp);
    const snapCandidate = resolveDirectSnapZoom(rawZoom, layout, this.#options.viewportWidth());
    const retainSnap = snapCandidate !== null && session.snapZoom !== null && zoomEquals(snapCandidate, session.snapZoom);
    session.snapZoom =
      snapCandidate !== null && (retainSnap || Math.abs(rawVelocity) < DIRECT_SNAP_VELOCITY_THRESHOLD) ? snapCandidate : null;
    const displayZoom = session.snapZoom ?? resolveDirectDocumentZoom(rawZoom, layout);
    if (!Number.isFinite(displayZoom)) return false;
    session.rawZoom = rawZoom;
    if (normalizedTimestamp > session.lastTimestampMs) session.lastTimestampMs = normalizedTimestamp;
    if (session.anchor) {
      session.anchor.focalX = focal.x;
      session.anchor.focalY = focal.y;
    }
    return this.#enqueueResolvedZoom(
      displayZoom,
      () => session.anchor,
      () => this.#directSession === session,
    );
  }

  async releaseDirectZoom(kind: DirectZoomKind): Promise<void> {
    const session = this.#directSession;
    if (!session || session.kind !== kind) return;
    this.#clearWheelReleaseTimer();
    await this.#applyQueue;
    if (this.#directSession !== session) return;
    this.#directSession = null;
    const reduceMotion = prefersReducedMotion();
    await this.#finishOrRecover(session, reduceMotion ? 'immediate' : 'animated');
  }

  cancelDirectZoom(kind?: DirectZoomKind): void {
    if (kind && this.#directSession?.kind !== kind) return;
    const current = this.#directSession;
    this.#cancelInteractiveMotion();
    if (current) void this.#finishOrRecover(current, prefersReducedMotion() ? 'immediate' : 'animated');
    else this.#scheduleRenderZoom(true);
  }

  interruptForDirectPan(): void {
    if (this.#directSession) {
      const kind = this.#directSession.kind;
      this.#directSession.anchor = null;
      void this.releaseDirectZoom(kind);
      return;
    }
    const active = this.#activeMotion;
    const layout = this.#options.layout();
    if (!active || !layout) return;
    const bounds = computeDocumentZoomBounds(layout);
    if (this.displayZoom >= bounds.min && this.displayZoom <= bounds.max) {
      this.#stopMotion();
      this.#scheduleRenderZoom(true);
      return;
    }
    active.anchor = null;
  }

  async handleWheel(event: WheelEvent): Promise<void> {
    if (!this.#options.layout()) return;
    const zoomDelta = Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
    if (zoomDelta === 0) return;
    if (!event.metaKey && !event.ctrlKey) {
      this.interruptForDirectPan();
      return;
    }
    if (event.cancelable) event.preventDefault();
    if (this.#directSession?.kind !== 'wheel' && !this.beginDirectZoom('wheel', event.clientX, event.clientY, event.timeStamp)) return;
    const session = this.#directSession;
    if (!session || session.kind !== 'wheel') return;
    const nextRawZoom = session.rawZoom * Math.exp(-zoomDelta / 240);
    this.#scheduleWheelRelease();
    await this.updateDirectZoom('wheel', nextRawZoom, event.clientX, event.clientY, event.timeStamp);
  }

  async zoomInByKeyboard(): Promise<void> {
    await this.#stepZoomByKeyboard(EditorZoomController.KEYBOARD_ZOOM_STEP);
  }

  async zoomOutByKeyboard(): Promise<void> {
    await this.#stepZoomByKeyboard(-EditorZoomController.KEYBOARD_ZOOM_STEP);
  }

  async resetByKeyboard(): Promise<void> {
    if (!this.#options.layout()) return;
    await this.#setZoomWithAnchor(1, this.#createZoomAnchorFromViewportCenter());
  }
}

function layoutKey(layout: DocumentZoomLayout): string {
  return layout.type === 'continuous' ? `continuous:${layout.maxWidth}` : `paginated:${layout.pageWidth}`;
}

function normalizeTimestamp(timestampMs: number): number {
  return Number.isFinite(timestampMs) ? timestampMs : nowMilliseconds();
}

function instantaneousLogZoomVelocity(
  previousZoom: number,
  zoom: number,
  previousTimestampMs: number | undefined,
  timestampMs: number,
): number {
  if (!Number.isFinite(previousZoom) || previousZoom <= 0 || !Number.isFinite(zoom) || zoom <= 0) return Infinity;
  if (zoomEquals(previousZoom, zoom)) return 0;
  if (previousTimestampMs === undefined) return Infinity;
  const elapsedSeconds = (timestampMs - previousTimestampMs) / 1000;
  return Number.isFinite(elapsedSeconds) && elapsedSeconds > 0 ? Math.log(zoom / previousZoom) / elapsedSeconds : Infinity;
}

function resolveDirectSnapZoom(zoom: number, layout: DocumentZoomLayout, viewportWidth: number): number | null {
  const resolvedViewportWidth = viewportWidth > 0 ? viewportWidth : 1;
  const bounds = computeDocumentZoomBounds(layout);
  if (!Number.isFinite(zoom) || zoom < bounds.min || zoom > bounds.max) return null;
  const candidate = clampDocumentLayoutZoom({ zoom, layout, viewportWidth: resolvedViewportWidth });
  const fitWidthZoom = computeDocumentFitWidthZoom(layout, resolvedViewportWidth);
  const unitZoom = clampDocumentZoom(1, bounds);
  return zoomEquals(candidate, fitWidthZoom) || zoomEquals(candidate, unitZoom) ? candidate : null;
}

function nowMilliseconds(): number {
  return globalThis.performance?.now() ?? Date.now();
}

function prefersReducedMotion(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function requestZoomAnimationFrame(callback: (timestampMs: number) => void): () => void {
  if (typeof requestAnimationFrame === 'function') {
    const id = requestAnimationFrame(callback);
    return () => cancelAnimationFrame(id);
  }
  const id = setTimeout(() => callback(nowMilliseconds()), 16);
  return () => clearTimeout(id);
}
