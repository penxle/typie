import { flushSync, untrack } from 'svelte';
import { PAGE_GAP } from './constants';
import { resolvePageSpans } from './geometry';
import { requiredSurfacePages } from './required-surface-pages';
import { zoomDiffers } from './zoom';
import type { EditorContext, EditorSnapshot, PublishedBundle } from './editor.svelte';
import type { EditorSurfaceHost } from './editor-surface-host.svelte';
import type { RevealTargetSpan, ScrollContainerMetrics } from './scroll';
import type { EditorBringIntoViewRequest, EditorScrollIntentResult } from './scroll.svelte';

type EditorSurfacePreparation = {
  requiredPages: Set<number>;
  pendingRequest: EditorBringIntoViewRequest | null;
  scrollIntent: EditorScrollIntentResult | null;
};

export function setupEditorPublication(ctx: EditorContext, getSurfaceHost: () => EditorSurfaceHost | undefined): void {
  let generation = 0;

  $effect.pre(() => {
    const editor = ctx.editor;
    const scroll = ctx.scroll;
    const surfaceHost = getSurfaceHost();
    if (!editor || !scroll || !surfaceHost) return;

    void editor.publicationVersion;
    void editor.appliedSnapshot;
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

  scroll.discardObsoleteForRevision(editor.appliedSnapshot.revision);
  const preparation = resolveEditorSurfacePreparation(editor, scroll);
  if (!preparation) return;
  editor.requestSurfacePages(preparation.requiredPages);
  surfaceHost.reconcile(preparation.requiredPages);
  surfaceHost.syncPublished();
  if (preparation.pendingRequest?.behavior === 'instant' && preparation.scrollIntent?.type === 'unresolved') return;

  const bundle = editor.publishIfReady(preparation.requiredPages);
  if (!bundle) return;
  if (!publicationStillCurrent(editor, scroll, bundle, preparation.pendingRequest, isCurrent)) return;
  // Svelte cannot flush recursively from $effect.pre. Start the presentation in the
  // following microtask, then revalidate and finish every visible change without yielding.
  queueMicrotask(() => {
    if (!publicationStillCurrent(editor, scroll, bundle, preparation.pendingRequest, isCurrent)) return;
    const finalPreparation = resolveEditorSurfacePreparation(editor, scroll);
    if (
      !finalPreparation ||
      (finalPreparation.pendingRequest?.behavior === 'instant' && finalPreparation.scrollIntent?.type === 'unresolved') ||
      finalPreparation.pendingRequest !== preparation.pendingRequest ||
      !samePages(finalPreparation.requiredPages, preparation.requiredPages)
    ) {
      editor.requestPublication();
      return;
    }
    if (!editor.acceptPublication(bundle)) return;

    flushSync();
    surfaceHost.syncPublished(bundle);
    if (finalPreparation.pendingRequest && finalPreparation.scrollIntent) {
      const applied = scroll.applyPending(finalPreparation.pendingRequest, bundle.snapshot, finalPreparation.scrollIntent);
      if (!applied && finalPreparation.pendingRequest.behavior === 'instant') {
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

function resolveEditorSurfacePreparation(
  editor: NonNullable<EditorContext['editor']>,
  scroll: NonNullable<EditorContext['scroll']>,
): EditorSurfacePreparation | null {
  const viewport = editor.scrollViewport;
  if (!viewport) return null;

  const snapshot = editor.appliedSnapshot;
  const viewportRect = viewport.getRect();
  const clientHeight = viewportRect.bottom - viewportRect.top;
  const scrollTop = viewport.getScrollTop();
  if (!Number.isFinite(scrollTop) || scrollTop < 0 || !Number.isFinite(clientHeight) || clientHeight <= 0) return null;

  const zoom = displayZoom(snapshot, editor.displayZoom);
  const origin = editor.extensionAreaEl ? editor.extensionAreaEl.getBoundingClientRect().top - viewportRect.top + scrollTop : 0;
  const pageSpans = resolvePageSpans(snapshot.pageSizes, {
    origin,
    displayZoom: zoom,
    scaleFactor: editor.scaleFactor,
    pageGap: snapshot.rootAttrs?.layout_mode.type === 'paginated' ? PAGE_GAP * zoom : 0,
  });
  const bottomPadding = scroll.bottomPaddingFor(snapshot);
  const predictedExtent = pageSpans.at(-1)?.bottom ?? origin;
  const scrollHeight = Math.max(predictedExtent + bottomPadding, clientHeight);
  const currentViewport = { top: scrollTop, bottom: scrollTop + clientHeight };

  const pendingRequest = scroll.activateForRevision(snapshot.revision);
  const targetRects = pendingRequest ? scroll.resolveTargetRects(pendingRequest.target, snapshot) : null;
  const target = targetRects ? resolveTargetSpan(targetRects, pageSpans, zoom) : null;
  const preparationViewports =
    target && pendingRequest?.behavior === 'instant'
      ? scroll.resolvePreparationViewports(
          pendingRequest,
          {
            scrollTop,
            clientHeight,
            scrollHeight,
            targetTop: target.targetTop,
            targetBottom: target.targetBottom,
          },
          snapshot,
        )
      : [];
  const scrollIntent = resolveScrollIntent(pendingRequest, targetRects, target, snapshot, scroll, {
    scrollTop,
    clientHeight,
    scrollHeight,
  });
  const requiredPages = requiredSurfacePages({
    pages: pageSpans,
    currentViewport,
    activePages: editor.activeSurfacePages,
    preparationViewports,
  });
  if (pendingRequest?.behavior === 'instant') {
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

  return {
    requiredPages,
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

function resolveTargetSpan(
  rects: ReturnType<NonNullable<EditorContext['scroll']>['resolveTargetRects']>,
  pages: ReturnType<typeof resolvePageSpans>,
  zoom: number,
): RevealTargetSpan | null {
  let targetTop = Infinity;
  let targetBottom = -Infinity;
  for (const rect of rects ?? []) {
    const page = pages[rect.page_idx];
    if (!page) continue;
    targetTop = Math.min(targetTop, page.top + rect.rect.y * zoom);
    targetBottom = Math.max(targetBottom, page.top + (rect.rect.y + rect.rect.height) * zoom);
  }
  return targetTop === Infinity ? null : { targetTop, targetBottom };
}

function displayZoom(snapshot: EditorSnapshot, zoom: number): number {
  return snapshot.rootAttrs?.layout_mode.type === 'paginated' && Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
}
