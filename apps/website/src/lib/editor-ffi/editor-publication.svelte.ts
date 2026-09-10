import { flushSync, untrack } from 'svelte';
import { applyMinimumRevealTargetHeight, pageRectsToRevealTargetSpan } from './geometry';
import { nearestSurfacePage, requiredSurfacePages } from './required-surface-pages';
import { requiredSurfaceTiles } from './required-surface-tiles';
import { isInstantReveal } from './scroll.svelte';
import { browserScaleFactor, zoomDiffers } from './zoom';
import type { EditorContext, EditorSnapshot, PublishedBundle } from './editor.svelte';
import type { EditorSurfaceHost } from './editor-surface-host.svelte';
import type { RevealTargetSpan, ScrollContainerMetrics } from './scroll';
import type { EditorBringIntoViewRequest, EditorScrollIntentResult } from './scroll.svelte';

export type EditorSurfacePreparation = {
  requiredPages: Set<number>;
  tiles: Map<number, number[]>;
  pendingRequest: EditorBringIntoViewRequest | null;
  scrollIntent: EditorScrollIntentResult | null;
};

export function setupEditorPublication(ctx: EditorContext, getSurfaceHost: () => EditorSurfaceHost | undefined): void {
  let generation = 0;

  $effect(() => {
    const editor = ctx.editor;
    const viewport = window.visualViewport;
    if (!editor || !viewport) return;
    const requestPublication = () => editor.requestPublication();
    viewport.addEventListener('resize', requestPublication);
    viewport.addEventListener('scroll', requestPublication);
    return () => {
      viewport.removeEventListener('resize', requestPublication);
      viewport.removeEventListener('scroll', requestPublication);
    };
  });

  $effect.pre(() => {
    const editor = ctx.editor;
    const scroll = ctx.scroll;
    const surfaceHost = getSurfaceHost();
    if (!editor || !scroll || !surfaceHost) return;

    void editor.publicationVersion;
    void editor.appliedSnapshot;
    void editor.viewport.width;
    void editor.viewport.height;
    void editor.viewport.scale_factor;
    const displayZoom = editor.displayZoom;
    const renderZoom = editor.renderZoom;
    void editor.extensionAreaEl;
    void editor.scrollViewport;
    void scroll.pendingRequest;

    const current = ++generation;
    untrack(() => {
      if (zoomDiffers(displayZoom, renderZoom)) return;
      reconcilePublication(editor, scroll, surfaceHost, () => current === generation);
    });
  });
}

function reconcilePublication(
  editor: NonNullable<EditorContext['editor']>,
  scroll: NonNullable<EditorContext['scroll']>,
  surfaceHost: EditorSurfaceHost,
  isCurrent: () => boolean,
): void {
  if (editor.terminal) return;

  const anchorPublication = scroll.prepareViewportAnchorPublication(editor.appliedSnapshot);
  if (anchorPublication.type === 'unavailable') return;
  const preparation = resolveEditorSurfacePreparation(
    editor,
    scroll,
    anchorPublication.targetScrollTop ?? undefined,
    anchorPublication.targetScrollLeft ?? undefined,
  );
  if (!preparation) return;
  editor.requestSurfacePages(preparation.requiredPages, preparation.tiles);
  surfaceHost.reconcile(preparation.requiredPages);
  surfaceHost.syncPublished();
  if (isInstantReveal(preparation.pendingRequest) && preparation.scrollIntent?.type === 'unresolved') return;

  const bundle = editor.publishIfReady(preparation.requiredPages);
  if (!bundle) return;
  if (!publicationStillCurrent(editor, scroll, bundle, preparation.pendingRequest, isCurrent)) return;
  // Svelte cannot flush recursively from $effect.pre. Start the presentation in the
  // following microtask, then revalidate and finish every visible change without yielding.
  queueMicrotask(() => {
    if (!publicationStillCurrent(editor, scroll, bundle, preparation.pendingRequest, isCurrent)) return;
    const finalAnchorPublication = scroll.prepareViewportAnchorPublication(bundle.snapshot);
    if (finalAnchorPublication.type === 'unavailable') {
      editor.requestPublication();
      return;
    }
    const finalPreparation = resolveEditorSurfacePreparation(
      editor,
      scroll,
      finalAnchorPublication.targetScrollTop ?? undefined,
      finalAnchorPublication.targetScrollLeft ?? undefined,
    );
    if (
      !finalPreparation ||
      (isInstantReveal(finalPreparation.pendingRequest) && finalPreparation.scrollIntent?.type === 'unresolved') ||
      finalPreparation.pendingRequest !== preparation.pendingRequest ||
      !samePages(finalPreparation.requiredPages, preparation.requiredPages) ||
      !sameTiles(finalPreparation.tiles, preparation.tiles)
    ) {
      editor.requestPublication();
      return;
    }
    if (!editor.acceptPublication(bundle)) return;

    flushSync();
    scroll.applyViewportAnchorPublication(finalAnchorPublication);
    surfaceHost.syncPublished(bundle);
    if (finalPreparation.pendingRequest && finalPreparation.scrollIntent) {
      const applied = scroll.applyPending(finalPreparation.pendingRequest, bundle.snapshot, finalPreparation.scrollIntent);
      if (!applied && isInstantReveal(finalPreparation.pendingRequest)) {
        editor.requestPublication();
        return;
      }
    }
    flushSync();
    editor.completePresentation(bundle);
  });
}

function publicationStillCurrent(
  editor: NonNullable<EditorContext['editor']>,
  scroll: NonNullable<EditorContext['scroll']>,
  bundle: PublishedBundle,
  pendingRequest: EditorBringIntoViewRequest | null,
  isCurrent: () => boolean,
): boolean {
  return (
    isCurrent() &&
    !editor.terminal &&
    editor.appliedSnapshot === bundle.snapshot &&
    (pendingRequest === null || scroll.pendingRequest === pendingRequest)
  );
}

export function resolveEditorSurfacePreparation(
  editor: NonNullable<EditorContext['editor']>,
  scroll: NonNullable<EditorContext['scroll']>,
  currentScrollTop?: number,
  currentScrollLeft?: number,
): EditorSurfacePreparation | null {
  const viewport = editor.scrollViewport;
  if (!viewport) return null;
  // VisualViewport changes before View installs the matching raster scale.
  // Every publication path must wait for that pair, not just resize events:
  // old high-resolution pixels combined with new wide bounds exceed the budget.
  if (zoomDiffers(editor.scaleFactor, browserScaleFactor())) return null;

  const snapshot = editor.appliedSnapshot;
  const viewportRect = viewport.getRect();
  const clientHeight = viewportRect.bottom - viewportRect.top;
  const actualScrollTop = viewport.getScrollTop();
  const scrollTop = currentScrollTop ?? actualScrollTop;
  if (!Number.isFinite(scrollTop) || scrollTop < 0 || !Number.isFinite(clientHeight) || clientHeight <= 0) return null;

  const zoom = displayZoom(snapshot, editor.displayZoom);
  const metrics = scroll.viewportMetrics(snapshot, true);
  if (!metrics) return null;
  const pageSpans = metrics.layout.pages;
  const origin = pageSpans[0]?.top ?? 0;
  const bottomPadding = scroll.bottomPaddingFor(snapshot);
  const predictedExtent = pageSpans.at(-1)?.bottom ?? origin;
  const scrollHeight = Math.max(predictedExtent + bottomPadding, clientHeight);
  const maximumScrollTop = Math.max(0, scrollHeight - clientHeight);
  const planningScrollTop = editor.scrollRootEl ? Math.max(0, Math.min(scrollTop, maximumScrollTop)) : scrollTop;
  const currentViewport = { top: planningScrollTop, bottom: planningScrollTop + clientHeight };

  const pendingRequest = scroll.activateForRevision(snapshot.revision);
  const targetRects = pendingRequest ? scroll.resolveTargetRects(pendingRequest.target, snapshot) : null;
  const resolvedTarget = targetRects ? pageRectsToRevealTargetSpan(targetRects, pageSpans, zoom) : null;
  const target =
    resolvedTarget && pendingRequest?.target.type === 'tracked_item'
      ? applyMinimumRevealTargetHeight(resolvedTarget, pendingRequest.target.minimumHeight)
      : resolvedTarget;
  const instantReveal = isInstantReveal(pendingRequest);
  const preparationViewports =
    target && pendingRequest && instantReveal
      ? scroll.resolvePreparationViewports(
          pendingRequest,
          {
            scrollTop: planningScrollTop,
            clientHeight,
            scrollHeight,
            targetTop: target.targetTop,
            targetBottom: target.targetBottom,
          },
          snapshot,
        )
      : [];
  const scrollIntent = resolveScrollIntent(pendingRequest, targetRects, target, snapshot, scroll, {
    scrollTop: planningScrollTop,
    clientHeight,
    scrollHeight,
  });
  const requiredPages = requiredSurfacePages({
    pages: pageSpans,
    currentViewport,
    activePages: editor.activeSurfacePages,
    preparationViewports,
  });
  // 요구 집합이 비면 프레임 없는 발행이 수용되는데, 그 뒤로는 requireFrame 게이트를 깨울 트리거가 없다 —
  // 첫 페이지 프레임이 발행되기 전까지는 최근접 페이지 하나를 항상 준비한다.
  const published = editor.published;
  if (requiredPages.size === 0 && (!published || published.snapshot.pageSizes.length === 0)) {
    const nearest = nearestSurfacePage(pageSpans, currentViewport);
    if (nearest !== null) requiredPages.add(nearest);
  }
  if (instantReveal) {
    const exactViewport =
      scrollIntent?.type === 'scroll_to' ? { top: scrollIntent.y, bottom: scrollIntent.y + clientHeight } : currentViewport;
    const exactPages = requiredSurfacePages({
      pages: pageSpans,
      currentViewport: exactViewport,
      activePages: editor.activeSurfacePages,
      preparationViewports: [],
    });
    if (target !== null && scrollIntent?.type !== 'unresolved' && [...exactPages].some((page) => !requiredPages.has(page))) {
      throw new Error(
        `Instant reveal destination requires unprepared surfaces: required=${[...requiredPages]} destination=${[...exactPages]}`,
      );
    }
  }

  // Browser pinch/smart zoom increases raster scale while narrowing and panning
  // the visible part of the layout viewport. Keep scroll planning in layout
  // coordinates, but allocate pixels only around that visible intersection.
  const visualViewport = window.visualViewport;
  const visibleLeft = Math.max(viewportRect.left, visualViewport?.offsetLeft ?? viewportRect.left);
  const visibleTop = Math.max(viewportRect.top, visualViewport?.offsetTop ?? viewportRect.top);
  const visibleRight = Math.min(viewportRect.right, visualViewport ? visualViewport.offsetLeft + visualViewport.width : viewportRect.right);
  const visibleBottom = Math.min(
    viewportRect.bottom,
    visualViewport ? visualViewport.offsetTop + visualViewport.height : viewportRect.bottom,
  );
  const visibleWidth = Math.max(0, visibleRight - visibleLeft);
  const visibleHeight = Math.max(0, visibleBottom - visibleTop);
  const left = currentScrollLeft ?? metrics.scrollLeft;
  // Page preparation covers every possible reveal alignment. Rasterize only the
  // resolved destination: unused alignments would add and remove tiles on each input.
  const rasterViewports = [currentViewport];
  if (instantReveal && scrollIntent?.type === 'scroll_to') {
    rasterViewports.push({ top: scrollIntent.y, bottom: scrollIntent.y + clientHeight });
  }
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable preparation snapshot
  const tiles = new Map<number, number[]>();
  for (const page of requiredPages) {
    const span = pageSpans.find((span) => span.page === page);
    const size = snapshot.pageSizes[page];
    if (!span || !size) continue;
    const regions =
      visibleWidth > 0 && visibleHeight > 0
        ? rasterViewports.map((view) => ({
            x: (left + visibleLeft - viewportRect.left - span.left - visibleWidth * 0.25) / zoom,
            y: (view.top + visibleTop - viewportRect.top - span.top - visibleHeight * 0.5) / zoom,
            width: (visibleWidth * 1.5) / zoom,
            height: (visibleHeight * 2) / zoom,
          }))
        : [];
    tiles.set(
      page,
      requiredSurfaceTiles(size.width, snapshot.pageBackingSizes[page]?.height ?? size.height, editor.surfaceScaleFactor, regions),
    );
  }

  return {
    requiredPages,
    tiles,
    pendingRequest,
    scrollIntent,
  };
}

function resolveScrollIntent(
  request: EditorBringIntoViewRequest | null,
  targetRects: ReturnType<NonNullable<EditorContext['scroll']>['resolveTargetRects']>,
  target: RevealTargetSpan | null,
  snapshot: EditorSnapshot,
  scroll: NonNullable<EditorContext['scroll']>,
  metrics: Pick<ScrollContainerMetrics, 'scrollTop' | 'clientHeight' | 'scrollHeight'>,
): EditorScrollIntentResult | null {
  if (!request) return null;
  if (targetRects === null) return { type: 'no_scroll' };
  if (target === null) return { type: 'unresolved' };

  const y = scroll.resolveScrollTop(request, { ...metrics, ...target }, snapshot);
  return y === null ? { type: 'no_scroll' } : { type: 'scroll_to', y };
}

function samePages(a: ReadonlySet<number>, b: ReadonlySet<number>): boolean {
  return a.size === b.size && [...a].every((page) => b.has(page));
}

function displayZoom(snapshot: EditorSnapshot, zoom: number): number {
  return snapshot.rootAttrs?.layout_mode && Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
}

function sameTiles(a: ReadonlyMap<number, readonly number[]>, b: ReadonlyMap<number, readonly number[]>): boolean {
  return (
    a.size === b.size &&
    [...a].every(([page, bounds]) => {
      const other = b.get(page);
      return other?.length === bounds.length && bounds.every((value, index) => value === other[index]);
    })
  );
}
