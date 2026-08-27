import { tryAppContext } from '@typie/ui/context';
import { untrack } from 'svelte';
import { CURSOR_VISIBLE_MARGIN, PAGE_GAP } from './constants';
import { pageRectsToRevealTargetSpan, resolvePageSpans, selectionHeadRect } from './geometry';
import {
  resolveGuardedScrollTop,
  resolveInstantRevealPreparationViewports,
  resolveKeepVisibleBottomPadding,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';
import { SmoothScrollMotion } from './smooth-scroll-motion';
import { EditorViewportAnchorState, resolveViewportAnchorGeometry, viewportCenterAnchorPoint } from './viewport-anchor';
import { resolveContinuousViewPadding } from './zoom';
import type { PageRect, ViewportAnchorResolution } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext, EditorSnapshot } from './editor.svelte';
import type { EditorRequest } from './editor-update';
import type { VerticalSpan } from './required-surface-pages';
import type { EditorVisibleArea, RevealTargetSpan, ScrollContainerMetrics } from './scroll';
import type { EditorViewportAnchorGeometry, EditorViewportAnchorLayout, EditorViewportAnchorRevealOrigin } from './viewport-anchor';

export type EditorScrollRevealPolicy = 'cursor_guard' | 'pointer_cursor_guard' | 'typewriter' | 'reveal';
type ResolvedEditorScrollRevealPolicy = Exclude<EditorScrollRevealPolicy, 'pointer_cursor_guard'>;
export type EditorScrollBehavior = 'instant' | 'smooth';

export type EditorScrollIntoViewTarget =
  | { type: 'current_selection_head' }
  // 같은 top anchor를 공유하는 UI가 있으면 rect union을 아래로 minimumHeight까지 늘린다.
  | { type: 'tracked_item'; id: string; minimumHeight?: number };

export type EditorScrollIntoViewOptions = {
  target: EditorScrollIntoViewTarget;
  policy: EditorScrollRevealPolicy;
  behavior?: EditorScrollBehavior;
};

export type EditorScrollIntentResult = { type: 'unresolved' } | { type: 'no_scroll' } | { type: 'scroll_to'; y: number };

export type EditorViewportAnchorPublication =
  | {
      type: 'ready';
      geometry: EditorViewportAnchorGeometry | null;
      targetScrollLeft: number | null;
      targetScrollTop: number | null;
    }
  | { type: 'unavailable' };

type TypewriterPreferences = {
  enabled: boolean;
  position: number | undefined;
};

export type EditorBringIntoViewRequest = EditorScrollIntoViewOptions & {
  behavior: EditorScrollBehavior;
  targetRevision: number | null;
  presentation: Promise<void>;
  completePresentation: () => void;
};

const DEFAULT_VISIBLE_AREA: EditorVisibleArea = {
  topInset: 0,
  bottomInset: 0,
};

function sameVisibleArea(a: EditorVisibleArea, b: EditorVisibleArea): boolean {
  return a.topInset === b.topInset && a.bottomInset === b.bottomInset;
}

function sanitizeInset(value: number): number {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function sanitizeVisibleArea(visibleArea: EditorVisibleArea): EditorVisibleArea {
  return {
    topInset: sanitizeInset(visibleArea.topInset),
    bottomInset: sanitizeInset(visibleArea.bottomInset),
  };
}

function sanitizeTypewriterPosition(position: number | undefined): number {
  return typeof position === 'number' && Number.isFinite(position) ? Math.max(0, Math.min(1, position)) : 0.5;
}

function prefersReducedMotion(): boolean {
  return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
}

export function isInstantReveal(request: Pick<EditorBringIntoViewRequest, 'behavior'> | null): boolean {
  return request !== null && (request.behavior === 'instant' || (request.behavior === 'smooth' && prefersReducedMotion()));
}

function createBringIntoViewRequest({ target, policy, behavior = 'instant' }: EditorScrollIntoViewOptions): EditorBringIntoViewRequest {
  const { promise: presentation, resolve } = Promise.withResolvers<undefined>();
  return {
    target,
    policy,
    behavior,
    targetRevision: null,
    presentation,
    completePresentation: () => resolve(undefined),
  };
}

export function setupEditorScroll(ctx: EditorContext): void {
  const app = tryAppContext();

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) {
      ctx.scroll = undefined;
      return;
    }

    // app is absent in the public viewer (no AppContext provider); fall back to typewriter off.
    const scope = new EditorScrollScope(editor, () => {
      const preference = app?.preference.current;
      return {
        enabled: preference?.typewriterEnabled ?? false,
        position: preference?.typewriterPosition,
      };
    });
    ctx.scroll = scope;
    editor.registerScrollIntoView((options, request) => scope.scrollIntoView(options, request));
    editor.registerViewportScrollObserver(() => scope.observeViewportScroll());

    return () => {
      editor.registerScrollIntoView(null);
      editor.registerViewportScrollObserver(null);
      scope.destroy();
      if (ctx.scroll === scope) {
        ctx.scroll = undefined;
      }
    };
  });

  $effect(() => {
    const editor = ctx.editor;
    const scroll = ctx.scroll;
    if (editor?.terminal) scroll?.destroy();
  });
}

export class EditorScrollScope {
  #pendingRequest: EditorBringIntoViewRequest | null = null;
  #keepVisibleTarget = $state<EditorScrollIntoViewTarget | null>(null);
  #destroyed = false;
  #expectedScrollTop: number | null = null;
  #expectedScrollLeft: number | null = null;
  #smoothRequest: EditorBringIntoViewRequest | null = null;
  #smoothMotion: SmoothScrollMotion | null = null;
  #smoothAnimationFrame: number | null = null;
  #smoothAnimationTime: number | null = null;
  readonly #editor: Editor;
  readonly #typewriterPreferences: () => TypewriterPreferences;
  readonly #viewportAnchor = new EditorViewportAnchorState();
  #contentBottomOverflow = $state(0);

  visibleArea = $state<EditorVisibleArea>(DEFAULT_VISIBLE_AREA);

  bottomPadding = $derived.by(() => {
    void this.#editor.viewport.height;
    return this.bottomPaddingFor(this.#editor.published?.snapshot);
  });

  constructor(editor: Editor, typewriterPreferences: () => TypewriterPreferences) {
    this.#editor = editor;
    this.#typewriterPreferences = typewriterPreferences;
  }

  get pendingRequest(): EditorBringIntoViewRequest | null {
    return this.#pendingRequest;
  }

  bottomPaddingFor(snapshot: EditorSnapshot | undefined): number {
    const rect = selectionHeadRect(snapshot);
    const needsKeepVisiblePadding = rect !== null || this.#hasResolvedKeepVisibleTarget(snapshot);
    const contentExtentPadding = this.#contentBottomOverflow + this.visibleArea.bottomInset;
    const minimumPadding = Math.max(
      contentExtentPadding,
      needsKeepVisiblePadding ? resolveKeepVisibleBottomPadding({ visibleArea: this.visibleArea }) : 0,
    );
    if (!rect) {
      return minimumPadding;
    }

    const prefs = this.#typewriterPreferences();
    if (!prefs.enabled) {
      return minimumPadding;
    }

    return Math.max(minimumPadding, this.#typewriterBottomPaddingForRect(rect, snapshot));
  }

  resolveTargetRects(target: EditorScrollIntoViewTarget, snapshot: EditorSnapshot | undefined): PageRect[] | null {
    if (!snapshot) return null;
    switch (target.type) {
      case 'current_selection_head': {
        const rect = selectionHeadRect(snapshot);
        return rect ? [rect] : null;
      }
      case 'tracked_item': {
        const range = this.#editor.trackedRangeForSnapshot(target.id, snapshot);
        return range && range.rects.length > 0 ? range.rects : null;
      }
    }
  }

  applyPending(request: EditorBringIntoViewRequest, snapshot: EditorSnapshot, result: EditorScrollIntentResult): boolean {
    if (this.#destroyed || this.#editor.destroyed || this.activateForRevision(snapshot.revision) !== request) return false;
    switch (result.type) {
      case 'unresolved': {
        return false;
      }
      case 'no_scroll': {
        this.#interruptSmoothReveal();
        const revealOrigin = this.#selectionRevealOrigin(request, this.#editor.scrollViewport?.getScrollTop());
        const presented = this.markPresented(snapshot.revision, request);
        if (presented) this.#attachSelectionOrCenter(request.target.type !== 'current_selection_head', revealOrigin);
        return presented;
      }
      case 'scroll_to': {
        const viewport = this.#editor.scrollViewport;
        if (!viewport) return false;
        if (!isInstantReveal(request)) {
          return this.#applySmoothReveal(request, snapshot, result.y);
        }
        this.#interruptSmoothReveal();
        const revealOrigin = this.#selectionRevealOrigin(request, viewport.getScrollTop());
        viewport.scrollTo({ top: result.y, behavior: 'instant' });
        this.#expectedScrollTop = viewport.getScrollTop();
        const presented = this.markPresented(snapshot.revision, request);
        if (presented) this.#attachSelectionOrCenter(request.target.type !== 'current_selection_head', revealOrigin);
        return presented;
      }
    }
  }

  declare(options: EditorScrollIntoViewOptions): EditorBringIntoViewRequest {
    const request = createBringIntoViewRequest(options);
    if (options.behavior === 'smooth' && this.#smoothMotion) this.#pauseSmoothReveal();
    else this.#interruptSmoothReveal();
    this.#pendingRequest?.completePresentation();
    this.#pendingRequest = request;
    this.#keepVisibleTarget = request.policy === 'pointer_cursor_guard' ? null : request.target;
    this.#editor.requestPublication();
    return request;
  }

  bind(request: EditorBringIntoViewRequest, revision: number): boolean {
    if (this.#pendingRequest !== request) return false;
    if (request.targetRevision !== null) return request.targetRevision === revision;
    request.targetRevision = revision;
    return this.#pendingRequest === request;
  }

  activateForRevision(revision: number): EditorBringIntoViewRequest | null {
    const request = this.#pendingRequest;
    if (!request || request.targetRevision === null) return null;
    return revision >= request.targetRevision ? request : null;
  }

  discard(request: EditorBringIntoViewRequest): void {
    if (this.#pendingRequest !== request) return;
    this.#interruptSmoothReveal();
    this.#pendingRequest = null;
    request.completePresentation();
    this.#editor.requestPublication();
  }

  discardFailedForRevision(revision: number): void {
    const request = this.activateForRevision(revision);
    if (request) this.discard(request);
  }

  cancel(): void {
    const request = this.#pendingRequest;
    if (request) this.discard(request);
  }

  markPresented(revision: number, request: EditorBringIntoViewRequest): boolean {
    if (this.activateForRevision(revision) !== request) return false;
    this.#pendingRequest = null;
    request.completePresentation();
    this.#editor.requestPublication();
    return true;
  }

  prepareViewportAnchorPublication(snapshot: EditorSnapshot): EditorViewportAnchorPublication {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) return { type: 'ready', geometry: null, targetScrollLeft: null, targetScrollTop: null };
    const selectionCapture = this.#editor.captureSelectionViewportAnchor(snapshot.revision);
    if (selectionCapture && this.#viewportAnchor.needsSelectionAdoption(selectionCapture.identity)) {
      const selectionMetrics = this.#viewportMetrics(snapshot, true);
      if (!selectionMetrics) return { type: 'unavailable' };
      const selectionGeometry = resolveViewportAnchorGeometry(selectionCapture.geometry, selectionMetrics.layout);
      if (!selectionGeometry) return { type: 'unavailable' };
      this.#viewportAnchor.adoptSelection(
        selectionCapture.identity,
        selectionGeometry,
        { left: selectionMetrics.scrollLeft, top: selectionMetrics.scrollTop },
        selectionMetrics.clientHeight,
        this.visibleArea,
        this.#smoothMotion !== null,
      );
    } else if (!snapshot.selection && this.#viewportAnchor.preferredSelectionIdentity) {
      if (!this.#smoothMotion && !this.#attachViewportCenter()) return { type: 'unavailable' };
      this.#viewportAnchor.clearPreferredSelection();
    }
    if (this.#smoothMotion) this.#attachViewportCenter();
    else this.#ensureViewportAnchor();

    const metrics = this.#viewportMetrics(snapshot, true);
    if (!metrics) return { type: 'unavailable' };
    const clampedScrollLeft = Math.max(0, Math.min(metrics.scrollLeft, metrics.maximumScrollLeft));
    const clampedScrollTop = Math.max(0, Math.min(metrics.scrollTop, metrics.maximumScrollTop));
    const fallbackPublication = (): EditorViewportAnchorPublication => ({
      type: 'ready',
      geometry: null,
      targetScrollLeft: clampedScrollLeft === metrics.scrollLeft ? null : clampedScrollLeft,
      targetScrollTop: clampedScrollTop === metrics.scrollTop ? null : clampedScrollTop,
    });

    const resolution = this.#resolveCandidateViewportAnchor(snapshot);
    if (!resolution) return fallbackPublication();
    if (resolution.type === 'unavailable') return { type: 'unavailable' };
    if (resolution.type === 'deleted' || resolution.type === 'not_laid_out') {
      this.#viewportAnchor.clear();
      return fallbackPublication();
    }

    const geometry = resolveViewportAnchorGeometry(resolution.geometry, metrics.layout);
    if (!geometry) return { type: 'unavailable' };
    const targetScroll = this.#viewportAnchor.publicationRevealScroll(
      geometry,
      metrics.scrollTop,
      metrics.clientHeight,
      metrics.scrollHeight,
      this.visibleArea,
      (origin) => {
        const rects = this.resolveTargetRects(origin.target, snapshot);
        const target = rects && pageRectsToRevealTargetSpan(rects, metrics.layout.pages, metrics.layout.zoom);
        if (!target) return null;
        return (
          this.resolveScrollTop(
            origin,
            {
              scrollTop: origin.scrollTop,
              clientHeight: metrics.clientHeight,
              scrollHeight: metrics.scrollHeight,
              ...target,
            },
            snapshot,
          ) ?? origin.scrollTop
        );
      },
      metrics.scrollLeft,
      metrics.maximumScrollLeft,
    );
    return {
      type: 'ready',
      geometry,
      targetScrollLeft: targetScroll.left === metrics.scrollLeft ? null : targetScroll.left,
      targetScrollTop: targetScroll.top,
    };
  }

  applyViewportAnchorPublication(publication: EditorViewportAnchorPublication): void {
    if (publication.type !== 'ready') return;
    const viewport = this.#editor.scrollViewport;
    if (publication.targetScrollLeft === null && publication.targetScrollTop === null) {
      if (viewport && publication.geometry) {
        this.#viewportAnchor.acceptGeometry(publication.geometry, {
          left: viewport.getScrollLeft(),
          top: viewport.getScrollTop(),
        });
      } else {
        this.#ensureViewportAnchor();
      }
      return;
    }
    if (!viewport) return;
    const viewportRect = viewport.getRect();
    const clientWidth = viewportRect.right - viewportRect.left;
    const clientHeight = viewportRect.bottom - viewportRect.top;
    const maximumScrollLeft = Math.max(0, viewport.getScrollWidth() - clientWidth);
    const maximumScrollTop = Math.max(0, viewport.getScrollHeight() - clientHeight);
    const previousScrollLeft = viewport.getScrollLeft();
    const previousScrollTop = viewport.getScrollTop();
    const targetLeft = Math.max(0, Math.min(publication.targetScrollLeft ?? previousScrollLeft, maximumScrollLeft));
    const targetTop = Math.max(0, Math.min(publication.targetScrollTop ?? previousScrollTop, maximumScrollTop));
    if (Math.abs(previousScrollLeft - targetLeft) > 1 || Math.abs(previousScrollTop - targetTop) > 1) {
      viewport.scrollTo({ left: targetLeft, top: targetTop, behavior: 'instant' });
      const actualScrollLeft = viewport.getScrollLeft();
      const actualScrollTop = viewport.getScrollTop();
      this.#expectedScrollLeft = actualScrollLeft;
      this.#expectedScrollTop = actualScrollTop;
      this.#smoothMotion?.translate(actualScrollTop - previousScrollTop);
    }
    if (!publication.geometry) {
      this.#ensureViewportAnchor();
      return;
    }
    this.#viewportAnchor.acceptGeometry(publication.geometry, {
      left: viewport.getScrollLeft(),
      top: viewport.getScrollTop(),
    });
  }

  observeViewportScroll(): void {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) return;
    const scrollLeft = viewport.getScrollLeft();
    const scrollTop = viewport.getScrollTop();
    const hasExpectedScroll = this.#expectedScrollLeft !== null || this.#expectedScrollTop !== null;
    const expectedLeftMatches = this.#expectedScrollLeft === null || Math.abs(scrollLeft - this.#expectedScrollLeft) <= 1;
    const expectedTopMatches = this.#expectedScrollTop === null || Math.abs(scrollTop - this.#expectedScrollTop) <= 1;
    if (hasExpectedScroll && expectedLeftMatches && expectedTopMatches) {
      this.#expectedScrollLeft = null;
      this.#expectedScrollTop = null;
      return;
    }
    this.#expectedScrollLeft = null;
    this.#expectedScrollTop = null;
    if (this.#smoothMotion) this.cancel();
    this.#viewportAnchor.finishRevealConvergence();

    const metrics = this.#viewportMetrics(this.#editor.published?.snapshot);
    const preferred = this.#resolvePreferredSelectionAnchor();
    if (
      preferred &&
      metrics &&
      this.#viewportAnchor.tryReactivatePreferredSelection(
        preferred,
        { left: metrics.scrollLeft, top: metrics.scrollTop },
        metrics.clientHeight,
        this.visibleArea,
      )
    ) {
      return;
    }

    const current = this.#resolveCurrentAnchor();
    if (
      current &&
      metrics &&
      this.#viewportAnchor.canRetainAfterDirectScroll(current, metrics.scrollTop, metrics.clientHeight, this.visibleArea)
    ) {
      this.#viewportAnchor.acceptGeometry(current, { left: metrics.scrollLeft, top: metrics.scrollTop });
    } else {
      this.#attachViewportCenter();
    }
  }

  reconcileViewportResize(): void {
    if (this.#smoothMotion) {
      const viewport = this.#editor.scrollViewport;
      const rect = viewport?.getRect();
      const viewportHeight = rect ? rect.bottom - rect.top : 0;
      if (viewportHeight > 0) {
        const target = this.#smoothMotion.snapshot().target;
        this.#smoothMotion.retarget(target, viewportHeight);
      }
      this.#attachViewportCenter();
      return;
    }
    const geometry = this.#resolveCurrentAnchor();
    const metrics = this.#viewportMetrics(this.#editor.published?.snapshot);
    if (!geometry || !metrics) {
      this.#ensureViewportAnchor();
      return;
    }
    const target = this.#viewportAnchor.resizeScroll(
      geometry,
      metrics.scrollTop,
      metrics.clientHeight,
      metrics.scrollHeight,
      this.visibleArea,
    );
    if (Math.abs(target - metrics.scrollTop) <= 1) return;
    this.#expectedScrollTop = target;
    this.#editor.scrollViewport?.scrollTo({ top: target, behavior: 'instant' });
    this.#viewportAnchor.acceptGeometry(geometry, { left: metrics.scrollLeft, top: target });
  }

  resolveScrollTop(
    request: Pick<EditorBringIntoViewRequest, 'target' | 'policy'>,
    metrics: ScrollContainerMetrics & RevealTargetSpan,
    snapshot: EditorSnapshot,
  ): number | null {
    const policy = this.#resolvePolicy(request, snapshot);
    switch (policy) {
      case 'typewriter': {
        return resolveTypewriterScrollTop({
          ...metrics,
          visibleArea: this.visibleArea,
          position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
        });
      }
      case 'reveal': {
        return resolveGuardedScrollTop({
          ...metrics,
          visibleArea: this.visibleArea,
          oversizedAlignment: 'start',
        });
      }
      case 'cursor_guard': {
        return resolveGuardedScrollTop({
          ...metrics,
          visibleArea: this.visibleArea,
          oversizedMinimumVisibleHeight: request.policy === 'pointer_cursor_guard' ? CURSOR_VISIBLE_MARGIN : undefined,
        });
      }
    }
  }

  resolvePreparationViewports(
    request: EditorBringIntoViewRequest,
    metrics: ScrollContainerMetrics & RevealTargetSpan,
    snapshot: EditorSnapshot,
  ): VerticalSpan[] {
    if (!isInstantReveal(request)) return [];
    const policy = this.#resolvePolicy(request, snapshot);
    return resolveInstantRevealPreparationViewports({
      ...metrics,
      mode: policy === 'typewriter' ? 'typewriter' : 'cursor_guard',
      visibleArea: this.visibleArea,
      oversizedMinimumVisibleHeight: request.policy === 'pointer_cursor_guard' ? CURSOR_VISIBLE_MARGIN : undefined,
      position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
    });
  }

  // eslint-disable-next-line unicorn/consistent-class-member-order -- public scroll contract is grouped before private policy details
  #resolveCandidateViewportAnchor(snapshot: EditorSnapshot): ViewportAnchorResolution | null {
    const identity = this.#viewportAnchor.identity;
    if (!identity) return null;
    const resolution = this.#editor.resolveViewportAnchor(snapshot.revision, identity);
    if (resolution.type !== 'deleted' && resolution.type !== 'not_laid_out') return resolution;

    this.#attachViewportCenter();
    const fallback = this.#viewportAnchor.identity;
    return fallback ? this.#editor.resolveViewportAnchor(snapshot.revision, fallback) : null;
  }

  #resolvePolicy(
    request: Pick<EditorBringIntoViewRequest, 'target' | 'policy'>,
    snapshot: EditorSnapshot,
  ): ResolvedEditorScrollRevealPolicy {
    if (request.policy === 'pointer_cursor_guard') return 'cursor_guard';
    if (request.policy !== 'typewriter') return request.policy;
    return request.target.type === 'current_selection_head' &&
      this.#typewriterPreferences().enabled &&
      this.resolveTargetRects(request.target, snapshot) !== null
      ? 'typewriter'
      : 'cursor_guard';
  }

  #hasResolvedKeepVisibleTarget(snapshot: EditorSnapshot | undefined): boolean {
    const target = this.#keepVisibleTarget;
    return target !== null && this.resolveTargetRects(target, snapshot) !== null;
  }

  #typewriterBottomPaddingForRect(rect: PageRect, snapshot: EditorSnapshot | undefined): number {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) {
      return 0;
    }

    const viewportRect = viewport.getRect();
    const zoom = this.#editor.safeDisplayZoom();
    const layoutMode = snapshot?.rootAttrs?.layout_mode;
    const trailingBottomMargin =
      layoutMode?.type === 'paginated' ? layoutMode.page_margin_bottom * zoom : resolveContinuousViewPadding(zoom);
    return resolveTypewriterBottomPadding({
      clientHeight: viewportRect.bottom - viewportRect.top,
      targetHeight: rect.rect.height * zoom,
      visibleArea: this.visibleArea,
      position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
      trailingBottomMargin,
    });
  }

  destroy(): void {
    this.#destroyed = true;
    this.#interruptSmoothReveal();
    this.#pendingRequest?.completePresentation();
    this.#pendingRequest = null;
  }

  setVisibleArea(visibleArea: EditorVisibleArea): void {
    const next = sanitizeVisibleArea(visibleArea);
    if (sameVisibleArea(this.visibleArea, next)) {
      return;
    }
    this.visibleArea = next;
    this.reconcileViewportResize();
    this.#editor.requestPublication();
  }

  setBottomInset(bottomInset: number): void {
    untrack(() => {
      this.setVisibleArea({
        topInset: this.visibleArea.topInset,
        bottomInset,
      });
    });
  }

  setContentBottomOverflow(bottomOverflow: number): void {
    const next = sanitizeInset(bottomOverflow);
    if (this.#contentBottomOverflow === next) return;
    this.#contentBottomOverflow = next;
    this.#editor.requestPublication();
  }

  attachViewportAnchorAt(point: { page: number; x: number; y: number }): void {
    const snapshot = this.#editor.published?.snapshot;
    const metrics = this.#viewportMetrics(snapshot);
    if (!snapshot || !metrics) return;
    const capture = this.#editor.captureViewportAnchorAt(snapshot.revision, {
      page_idx: point.page,
      x: point.x,
      y: point.y,
    });
    if (!capture) return;
    const geometry = resolveViewportAnchorGeometry(capture.geometry, metrics.layout);
    if (!geometry) return;
    this.#viewportAnchor.attachViewport(capture.identity, geometry, {
      left: metrics.scrollLeft,
      top: metrics.scrollTop,
    });
    this.#expectedScrollLeft = metrics.scrollLeft;
    this.#expectedScrollTop = metrics.scrollTop;
  }

  scrollIntoView(options: EditorScrollIntoViewOptions, admission?: EditorRequest): Promise<void> | undefined {
    if (this.#destroyed) {
      return;
    }

    const request = this.declare(options);
    if (admission) {
      admission.beforePublish(
        (update) => {
          if (!this.bind(request, update.revision)) this.discard(request);
        },
        () => this.discard(request),
      );
    } else {
      this.bind(request, this.#editor.appliedSnapshot.revision);
    }
    return request.presentation;
  }

  #ensureViewportAnchor(): void {
    if (this.#viewportAnchor.identity) return;
    this.#attachSelectionOrCenter();
  }

  #attachSelectionOrCenter(requireGuard = true, revealOrigin?: EditorViewportAnchorRevealOrigin): void {
    const snapshot = this.#editor.published?.snapshot;
    const metrics = this.#viewportMetrics(snapshot);
    if (!snapshot || !metrics) return;
    const capture = this.#editor.captureSelectionViewportAnchor(snapshot.revision);
    if (capture) {
      const geometry = resolveViewportAnchorGeometry(capture.geometry, metrics.layout);
      if (
        geometry &&
        (!requireGuard ||
          this.#viewportAnchor.canRetainAfterDirectScroll(geometry, metrics.scrollTop, metrics.clientHeight, this.visibleArea))
      ) {
        this.#viewportAnchor.attachSelection(
          capture.identity,
          geometry,
          { left: metrics.scrollLeft, top: metrics.scrollTop },
          revealOrigin,
        );
        return;
      }
    }
    this.#attachViewportCenter();
  }

  #attachViewportCenter(): boolean {
    const snapshot = this.#editor.published?.snapshot;
    const metrics = this.#viewportMetrics(snapshot);
    if (!snapshot || !metrics) return false;
    const viewportRect = this.#editor.scrollViewport?.getRect();
    const topInset = Math.max(0, this.visibleArea.topInset);
    const visibleHeight = Math.max(0, metrics.clientHeight - topInset - Math.max(0, this.visibleArea.bottomInset));
    const local = viewportRect
      ? this.#editor.clientToLocal(
          viewportRect.left + (viewportRect.right - viewportRect.left) / 2,
          viewportRect.top + topInset + visibleHeight / 2,
        )
      : null;
    const point = local
      ? { page_idx: local.page, x: local.x, y: local.y }
      : viewportCenterAnchorPoint(snapshot, metrics.layout, metrics.scrollTop, metrics.clientHeight, this.visibleArea);
    if (!point) return false;
    const capture = this.#editor.captureViewportAnchorAt(snapshot.revision, point);
    if (!capture) return false;
    const geometry = resolveViewportAnchorGeometry(capture.geometry, metrics.layout);
    if (!geometry) return false;
    this.#viewportAnchor.attachViewport(capture.identity, geometry, { left: metrics.scrollLeft, top: metrics.scrollTop });
    return true;
  }

  #selectionRevealOrigin(request: EditorBringIntoViewRequest, scrollTop: number | undefined): EditorViewportAnchorRevealOrigin | undefined {
    return scrollTop !== undefined && Number.isFinite(scrollTop) && request.target.type === 'current_selection_head'
      ? { scrollTop, target: request.target, policy: request.policy }
      : undefined;
  }

  #resolvePreferredSelectionAnchor(): EditorViewportAnchorGeometry | null {
    const snapshot = this.#editor.published?.snapshot;
    const identity = this.#viewportAnchor.preferredSelectionIdentity;
    if (!snapshot || !identity) return null;
    const resolution = this.#editor.resolveViewportAnchor(snapshot.revision, identity);
    if (resolution.type === 'deleted') {
      this.#viewportAnchor.clearPreferredSelection();
      return null;
    }
    if (resolution.type !== 'resolved') return null;
    const metrics = this.#viewportMetrics(snapshot);
    return metrics ? resolveViewportAnchorGeometry(resolution.geometry, metrics.layout) : null;
  }

  #resolveCurrentAnchor(): EditorViewportAnchorGeometry | null {
    const snapshot = this.#editor.published?.snapshot;
    const identity = this.#viewportAnchor.identity;
    if (!snapshot || !identity) return null;
    const resolution = this.#editor.resolveViewportAnchor(snapshot.revision, identity);
    if (resolution.type !== 'resolved') return null;
    const metrics = this.#viewportMetrics(snapshot);
    return metrics ? resolveViewportAnchorGeometry(resolution.geometry, metrics.layout) : null;
  }

  #viewportMetrics(
    snapshot: EditorSnapshot | undefined,
    candidateExtent = false,
  ): {
    layout: EditorViewportAnchorLayout;
    scrollLeft: number;
    scrollTop: number;
    clientWidth: number;
    clientHeight: number;
    scrollWidth: number;
    scrollHeight: number;
    maximumScrollLeft: number;
    maximumScrollTop: number;
  } | null {
    const viewport = this.#editor.scrollViewport;
    if (!snapshot || !viewport) return null;
    const viewportRect = viewport.getRect();
    const clientWidth = viewportRect.right - viewportRect.left;
    const clientHeight = viewportRect.bottom - viewportRect.top;
    const scrollLeft = viewport.getScrollLeft();
    const scrollTop = viewport.getScrollTop();
    if (![scrollLeft, scrollTop, clientWidth, clientHeight].every(Number.isFinite) || clientWidth <= 0 || clientHeight <= 0) return null;
    const zoom = this.#editor.safeDisplayZoom();
    const extensionRect = this.#editor.extensionAreaEl?.getBoundingClientRect();
    const origin = extensionRect ? extensionRect.top - viewportRect.top + scrollTop : 0;
    const extensionLeft = extensionRect ? extensionRect.left - viewportRect.left + scrollLeft : 0;
    const useCandidateHorizontalGeometry = candidateExtent && extensionRect !== undefined;
    const parsedPaddingLeft = useCandidateHorizontalGeometry
      ? Number.parseFloat(this.#editor.extensionAreaEl?.style?.paddingLeft ?? '')
      : NaN;
    const parsedPaddingRight = useCandidateHorizontalGeometry
      ? Number.parseFloat(this.#editor.extensionAreaEl?.style?.paddingRight ?? '')
      : NaN;
    const extensionPaddingLeft = Number.isFinite(parsedPaddingLeft) ? Math.max(0, parsedPaddingLeft) : 0;
    const extensionPaddingRight = Number.isFinite(parsedPaddingRight) ? Math.max(0, parsedPaddingRight) : 0;
    const candidateExtensionWidth = useCandidateHorizontalGeometry
      ? Math.max(
          clientWidth,
          snapshot.pageSizes.reduce((width, page) => Math.max(width, page.width * zoom), 0) + extensionPaddingLeft + extensionPaddingRight,
        )
      : clientWidth;
    const candidateContentWidth = Math.max(0, candidateExtensionWidth - extensionPaddingLeft - extensionPaddingRight);
    const pageSpans = resolvePageSpans(snapshot.pageSizes, {
      origin,
      displayZoom: zoom,
      scaleFactor: this.#editor.scaleFactor,
      pageGap: snapshot.rootAttrs?.layout_mode.type === 'paginated' ? PAGE_GAP * zoom : 0,
    });
    const pages = pageSpans.map((span) => {
      const pageRect = this.#editor.pageEls[span.page]?.getBoundingClientRect();
      const displayedWidth = snapshot.pageSizes[span.page].width * zoom;
      const left = useCandidateHorizontalGeometry
        ? extensionLeft + extensionPaddingLeft + Math.max(0, (candidateContentWidth - displayedWidth) / 2)
        : pageRect
          ? pageRect.left - viewportRect.left + scrollLeft
          : extensionLeft + Math.max(0, ((extensionRect?.width ?? clientWidth) - displayedWidth) / 2);
      return { ...span, left };
    });
    const predictedRight = pages.reduce(
      (right, page) => Math.max(right, page.left + snapshot.pageSizes[page.page].width * zoom),
      useCandidateHorizontalGeometry ? Math.max(clientWidth, extensionLeft + candidateExtensionWidth) : clientWidth,
    );
    const predictedExtent = pages.at(-1)?.bottom ?? origin;
    const candidateScrollWidth = Math.max(predictedRight, clientWidth);
    const candidateScrollHeight = Math.max(predictedExtent + this.bottomPaddingFor(snapshot), clientHeight);
    const scrollWidth =
      candidateExtent && this.#editor.scrollRootEl ? candidateScrollWidth : Math.max(viewport.getScrollWidth(), candidateScrollWidth);
    const scrollHeight =
      candidateExtent && this.#editor.scrollRootEl ? candidateScrollHeight : Math.max(viewport.getScrollHeight(), candidateScrollHeight);
    return {
      layout: { pages, zoom },
      scrollLeft,
      scrollTop,
      clientWidth,
      clientHeight,
      scrollWidth,
      scrollHeight,
      maximumScrollLeft: Math.max(0, scrollWidth - clientWidth),
      maximumScrollTop: Math.max(0, scrollHeight - clientHeight),
    };
  }

  #applySmoothReveal(request: EditorBringIntoViewRequest, snapshot: EditorSnapshot, target: number): boolean {
    const viewport = this.#editor.scrollViewport;
    const metrics = this.#viewportMetrics(snapshot);
    if (!viewport || !metrics) return false;
    const clampedTarget = Math.max(0, Math.min(target, metrics.maximumScrollTop));
    const previous = this.#smoothMotion?.snapshot();
    const direction = Math.sign(clampedTarget - metrics.scrollTop);
    const previousDirection = previous ? Math.sign(previous.target - previous.position) : 0;
    if (this.#smoothRequest === request && previous?.target === clampedTarget) {
      if (this.#smoothMotion?.finished) this.#finishSmoothReveal();
      else this.#scheduleSmoothAnimationFrame();
      return true;
    }
    if (
      !this.#smoothMotion ||
      (direction !== 0 && previousDirection !== 0 && direction !== previousDirection && this.#smoothRequest !== request)
    ) {
      this.#smoothMotion = SmoothScrollMotion.start({
        position: metrics.scrollTop,
        target: clampedTarget,
        viewportHeight: metrics.clientHeight,
      });
    } else {
      this.#smoothMotion.synchronizeBounds(metrics.scrollTop, metrics.maximumScrollTop, metrics.clientHeight);
      this.#smoothMotion.retarget(clampedTarget, metrics.clientHeight);
    }
    this.#smoothRequest = request;
    this.#attachViewportCenter();
    if (this.#smoothMotion.finished) {
      this.#finishSmoothReveal();
    } else {
      this.#scheduleSmoothAnimationFrame();
    }
    return true;
  }

  #scheduleSmoothAnimationFrame(): void {
    if (this.#smoothAnimationFrame !== null) return;
    this.#smoothAnimationFrame = requestAnimationFrame((time) => this.#advanceSmoothReveal(time));
  }

  #advanceSmoothReveal(time: number): void {
    this.#smoothAnimationFrame = null;
    const request = this.#smoothRequest;
    const motion = this.#smoothMotion;
    const viewport = this.#editor.scrollViewport;
    if (!request || !motion || !viewport) return;
    const previousTime = this.#smoothAnimationTime;
    this.#smoothAnimationTime = time;
    if (previousTime === null) {
      this.#scheduleSmoothAnimationFrame();
      return;
    }

    const rect = viewport.getRect();
    const viewportHeight = rect.bottom - rect.top;
    const maximumScrollTop = Math.max(0, viewport.getScrollHeight() - viewportHeight);
    const current = viewport.getScrollTop();
    const snapshot = motion.snapshot();
    if (Math.abs(current - snapshot.position) > 1 || snapshot.target < 0 || snapshot.target > maximumScrollTop) {
      motion.synchronizeBounds(current, maximumScrollTop, viewportHeight);
    }
    const next = motion.advance((time - previousTime) / 1000);
    this.#expectedScrollTop = next.position;
    viewport.scrollTo({ top: next.position, behavior: 'instant' });
    const actual = viewport.getScrollTop();
    if (Math.abs(actual - next.position) > 1) {
      this.#expectedScrollTop = actual;
      motion.synchronizeBounds(actual, maximumScrollTop, viewportHeight);
    }
    if (motion.finished) this.#finishSmoothReveal();
    else this.#scheduleSmoothAnimationFrame();
  }

  #finishSmoothReveal(): void {
    const request = this.#smoothRequest;
    const motion = this.#smoothMotion;
    const viewport = this.#editor.scrollViewport;
    if (!request || !motion || !viewport) return;
    const target = motion.snapshot().target;
    if (Math.abs(viewport.getScrollTop() - target) > 0.5) {
      viewport.scrollTo({ top: target, behavior: 'instant' });
      this.#expectedScrollTop = viewport.getScrollTop();
    }
    const publicationCurrent = this.#editor.published?.snapshot === this.#editor.appliedSnapshot;
    if (!publicationCurrent) {
      this.#smoothAnimationTime = null;
      this.#editor.requestPublication();
      return;
    }
    this.#clearSmoothReveal();
    if (this.markPresented(this.#editor.publishedRevision ?? this.#editor.appliedRevision, request)) {
      this.#attachSelectionOrCenter(request.target.type !== 'current_selection_head');
    }
  }

  #pauseSmoothReveal(): void {
    if (this.#smoothAnimationFrame !== null) cancelAnimationFrame(this.#smoothAnimationFrame);
    this.#smoothAnimationFrame = null;
    this.#smoothAnimationTime = null;
    this.#smoothRequest = null;
  }

  #clearSmoothReveal(): void {
    this.#pauseSmoothReveal();
    this.#smoothMotion = null;
  }

  #interruptSmoothReveal(): void {
    this.#smoothMotion?.cancel();
    this.#clearSmoothReveal();
  }
}
