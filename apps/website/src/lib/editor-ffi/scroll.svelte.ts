import { getAppContext } from '@typie/ui/context';
import { untrack } from 'svelte';
import { CONTINUOUS_VIEW_PADDING, CURSOR_VISIBLE_MARGIN, PAGE_GAP } from './constants';
import { pageRectsToRevealTargetSpan, resolvePageSpans, selectionHeadRect } from './geometry';
import {
  resolveGuardedScrollTop,
  resolveInstantRevealPreparationViewports,
  resolveKeepVisibleBottomPadding,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';
import { EditorViewportAnchorState, resolveViewportAnchorGeometry, viewportCenterAnchorPoint } from './viewport-anchor';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext, EditorSnapshot } from './editor.svelte';
import type { EditorRequest } from './editor-update';
import type { VerticalSpan } from './required-surface-pages';
import type { EditorVisibleArea, RevealTargetSpan, ScrollContainerMetrics } from './scroll';
import type { EditorViewportAnchorGeometry, EditorViewportAnchorLayout, EditorViewportAnchorRevealOrigin } from './viewport-anchor';

export type EditorScrollRevealPolicy = 'cursor_guard' | 'pointer_cursor_guard' | 'typewriter' | 'reveal';
type ResolvedEditorScrollRevealPolicy = Exclude<EditorScrollRevealPolicy, 'pointer_cursor_guard'>;
export type EditorScrollBehavior = 'instant' | 'smooth';

export type EditorScrollIntoViewTarget = { type: 'current_selection_head' } | { type: 'tracked_item'; id: string };

export type EditorScrollIntoViewOptions = {
  target: EditorScrollIntoViewTarget;
  policy: EditorScrollRevealPolicy;
  behavior?: EditorScrollBehavior;
};

export type EditorScrollIntentResult = { type: 'unresolved' } | { type: 'no_scroll' } | { type: 'scroll_to'; y: number };

export type EditorViewportAnchorPublication =
  { type: 'ready'; geometry: EditorViewportAnchorGeometry | null; targetScrollTop: number | null } | { type: 'unavailable' };

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
  const app = getAppContext();

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

    return () => {
      editor.registerScrollIntoView(null);
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
  #smoothRequest: EditorBringIntoViewRequest | null = null;
  #smoothTargetY: number | null = null;
  #smoothSettleTimer: ReturnType<typeof setTimeout> | null = null;
  readonly #editor: Editor;
  readonly #typewriterPreferences: () => TypewriterPreferences;
  readonly #viewportAnchor = new EditorViewportAnchorState();

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
    const minimumPadding = needsKeepVisiblePadding
      ? resolveKeepVisibleBottomPadding({ visibleArea: this.visibleArea })
      : this.visibleArea.bottomInset;
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
        if (presented) this.#attachSelectionOrCenter(false, revealOrigin);
        return presented;
      }
      case 'scroll_to': {
        const viewport = this.#editor.scrollViewport;
        if (!viewport) return false;
        if (request.behavior === 'smooth') {
          if (this.#smoothRequest === request && this.#smoothTargetY !== null && Math.abs(this.#smoothTargetY - result.y) <= 1) {
            return true;
          }
          this.#smoothRequest = request;
          this.#smoothTargetY = result.y;
          this.#attachViewportCenter();
          viewport.scrollTo({ top: result.y, behavior: request.behavior });
          this.#scheduleSmoothSettle();
          return true;
        }
        const revealOrigin = this.#selectionRevealOrigin(request, viewport.getScrollTop());
        viewport.scrollTo({ top: result.y, behavior: request.behavior });
        this.#expectedScrollTop = result.y;
        const presented = this.markPresented(snapshot.revision, request);
        if (presented) this.#attachSelectionOrCenter(false, revealOrigin);
        return presented;
      }
    }
  }

  declare(options: EditorScrollIntoViewOptions): EditorBringIntoViewRequest {
    const request = createBringIntoViewRequest(options);
    this.#interruptSmoothReveal();
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
    const eligible = request.policy === 'pointer_cursor_guard' ? revision === request.targetRevision : revision >= request.targetRevision;
    return eligible ? request : null;
  }

  discardObsoleteForRevision(revision: number): void {
    const request = this.#pendingRequest;
    if (request?.policy === 'pointer_cursor_guard' && request.targetRevision !== null && revision > request.targetRevision) {
      this.discard(request);
    }
  }

  discard(request: EditorBringIntoViewRequest): void {
    if (this.#pendingRequest !== request) return;
    if (this.#smoothRequest === request) this.#interruptSmoothReveal();
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
    if (!viewport) return { type: 'ready', geometry: null, targetScrollTop: null };
    if (this.#smoothRequest) this.#attachViewportCenter();
    else this.#ensureViewportAnchor();

    const identity = this.#viewportAnchor.identity;
    if (!identity) return { type: 'ready', geometry: null, targetScrollTop: null };
    let resolution = this.#editor.resolveViewportAnchor(snapshot.revision, identity);
    if (resolution.type === 'unavailable') return { type: 'unavailable' };
    if (resolution.type === 'deleted' || resolution.type === 'not_laid_out') {
      this.#attachViewportCenter();
      const fallback = this.#viewportAnchor.identity;
      if (!fallback) return { type: 'ready', geometry: null, targetScrollTop: null };
      resolution = this.#editor.resolveViewportAnchor(snapshot.revision, fallback);
      if (resolution.type === 'unavailable') return { type: 'unavailable' };
      if (resolution.type === 'deleted' || resolution.type === 'not_laid_out') {
        this.#viewportAnchor.clear();
        return { type: 'ready', geometry: null, targetScrollTop: null };
      }
    }

    const metrics = this.#viewportMetrics(snapshot);
    if (!metrics) return { type: 'unavailable' };
    const geometry = resolveViewportAnchorGeometry(resolution.geometry, metrics.layout);
    if (!geometry) return { type: 'unavailable' };
    return {
      type: 'ready',
      geometry,
      targetScrollTop: this.#viewportAnchor.publicationRevealScroll(
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
      ),
    };
  }

  applyViewportAnchorPublication(publication: EditorViewportAnchorPublication): void {
    if (publication.type !== 'ready') return;
    if (!publication.geometry || publication.targetScrollTop === null) {
      this.#ensureViewportAnchor();
      return;
    }
    const viewport = this.#editor.scrollViewport;
    if (!viewport) return;
    const viewportRect = viewport.getRect();
    const clientHeight = viewportRect.bottom - viewportRect.top;
    const maximumScrollTop = Math.max(0, viewport.getScrollHeight() - clientHeight);
    const target = Math.max(0, Math.min(publication.targetScrollTop, maximumScrollTop));
    if (Math.abs(viewport.getScrollTop() - target) > 1) {
      if (this.#smoothRequest) this.#smoothTargetY = null;
      this.#expectedScrollTop = target;
      viewport.scrollTo({ top: target, behavior: 'instant' });
    }
    this.#viewportAnchor.acceptGeometry(publication.geometry, viewport.getScrollTop());
  }

  observeViewportScroll(): void {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) return;
    const scrollTop = viewport.getScrollTop();
    if (this.#expectedScrollTop !== null && Math.abs(scrollTop - this.#expectedScrollTop) <= 1) {
      this.#expectedScrollTop = null;
      return;
    }
    this.#expectedScrollTop = null;
    if (this.#smoothRequest) {
      this.#attachViewportCenter();
      this.#scheduleSmoothSettle();
      return;
    }
    this.#viewportAnchor.finishRevealConvergence();

    const metrics = this.#viewportMetrics(this.#editor.published?.snapshot);
    const preferred = this.#resolvePreferredSelectionAnchor();
    if (
      preferred &&
      metrics &&
      this.#viewportAnchor.tryReactivatePreferredSelection(preferred, metrics.scrollTop, metrics.clientHeight, this.visibleArea)
    ) {
      return;
    }

    const current = this.#resolveCurrentAnchor();
    if (
      current &&
      metrics &&
      this.#viewportAnchor.canRetainAfterDirectScroll(current, metrics.scrollTop, metrics.clientHeight, this.visibleArea)
    ) {
      this.#viewportAnchor.acceptGeometry(current, metrics.scrollTop);
    } else {
      this.#attachViewportCenter();
    }
  }

  reconcileViewportResize(): void {
    if (this.#smoothRequest) {
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
    this.#viewportAnchor.acceptGeometry(geometry, target);
  }

  settleSmoothReveal(): void {
    const request = this.#smoothRequest;
    if (!request) return;
    const target = this.#smoothTargetY;
    const scrollTop = this.#editor.scrollViewport?.getScrollTop();
    if (target === null || scrollTop === undefined || Math.abs(scrollTop - target) > 1) return;
    this.#clearSmoothReveal();
    if (this.markPresented(this.#editor.publishedRevision ?? this.#editor.appliedRevision, request)) {
      this.#attachSelectionOrCenter(false);
    }
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
    if (request.behavior !== 'instant') return [];
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
    const trailingBottomMargin = layoutMode?.type === 'paginated' ? layoutMode.page_margin_bottom * zoom : CONTINUOUS_VIEW_PADDING;
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
        this.#viewportAnchor.attachSelection(capture.identity, geometry, metrics.scrollTop, revealOrigin);
        return;
      }
    }
    this.#attachViewportCenter();
  }

  #attachViewportCenter(): void {
    const snapshot = this.#editor.published?.snapshot;
    const metrics = this.#viewportMetrics(snapshot);
    if (!snapshot || !metrics) return;
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
    if (!point) return;
    const capture = this.#editor.captureViewportAnchorAt(snapshot.revision, point);
    if (!capture) return;
    const geometry = resolveViewportAnchorGeometry(capture.geometry, metrics.layout);
    if (geometry) this.#viewportAnchor.attachViewport(capture.identity, geometry, metrics.scrollTop);
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

  #viewportMetrics(snapshot: EditorSnapshot | undefined): {
    layout: EditorViewportAnchorLayout;
    scrollTop: number;
    clientHeight: number;
    scrollHeight: number;
    maximumScrollTop: number;
  } | null {
    const viewport = this.#editor.scrollViewport;
    if (!snapshot || !viewport) return null;
    const viewportRect = viewport.getRect();
    const clientHeight = viewportRect.bottom - viewportRect.top;
    const scrollTop = viewport.getScrollTop();
    if (!Number.isFinite(scrollTop) || !Number.isFinite(clientHeight) || clientHeight <= 0) return null;
    const zoom = snapshot.rootAttrs?.layout_mode.type === 'paginated' ? this.#editor.safeDisplayZoom() : 1;
    const origin = this.#editor.extensionAreaEl
      ? this.#editor.extensionAreaEl.getBoundingClientRect().top - viewportRect.top + scrollTop
      : 0;
    const pages = resolvePageSpans(snapshot.pageSizes, {
      origin,
      displayZoom: zoom,
      scaleFactor: this.#editor.scaleFactor,
      pageGap: snapshot.rootAttrs?.layout_mode.type === 'paginated' ? PAGE_GAP * zoom : 0,
    });
    const predictedExtent = pages.at(-1)?.bottom ?? origin;
    const scrollHeight = Math.max(viewport.getScrollHeight(), predictedExtent + this.bottomPaddingFor(snapshot), clientHeight);
    return {
      layout: { pages, zoom },
      scrollTop,
      clientHeight,
      scrollHeight,
      maximumScrollTop: Math.max(0, scrollHeight - clientHeight),
    };
  }

  #scheduleSmoothSettle(): void {
    if (this.#smoothSettleTimer !== null) clearTimeout(this.#smoothSettleTimer);
    this.#smoothSettleTimer = setTimeout(() => this.settleSmoothReveal(), 120);
  }

  #clearSmoothReveal(): void {
    if (this.#smoothSettleTimer !== null) clearTimeout(this.#smoothSettleTimer);
    this.#smoothSettleTimer = null;
    this.#smoothRequest = null;
    this.#smoothTargetY = null;
  }

  #interruptSmoothReveal(): void {
    if (!this.#smoothRequest) {
      this.#clearSmoothReveal();
      return;
    }
    const viewport = this.#editor.scrollViewport;
    const scrollTop = viewport?.getScrollTop();
    if (viewport && scrollTop !== undefined && Number.isFinite(scrollTop)) {
      this.#expectedScrollTop = scrollTop;
      viewport.scrollTo({ top: scrollTop, behavior: 'instant' });
    }
    this.#clearSmoothReveal();
  }
}
