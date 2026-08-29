import { clamp } from '@typie/ui/utils';
import stringify from 'fast-json-stable-stringify';
import { CURSOR_VISIBLE_MARGIN } from './constants';
import { resolvePageAtY } from './geometry';
import { resolveGuardedScrollTop } from './scroll';
import type { ResolvedViewportAnchor, ViewportAnchor, ViewportAnchorPoint } from '@typie/editor-ffi/browser';
import type { EditorSnapshot } from './editor.svelte';
import type { VerticalSpan } from './required-surface-pages';
import type { EditorVisibleArea } from './scroll';
import type { EditorScrollIntoViewTarget, EditorScrollRevealPolicy } from './scroll.svelte';

export type EditorViewportAnchorGeometry = {
  pointX: number;
  pointY: number;
  rect?: { top: number; bottom: number };
};

type ActiveViewportAnchor = {
  identity: ViewportAnchor;
  source: 'selection' | 'viewport';
  pointAttachmentX: number;
  pointAttachmentY: number;
  attachmentPending: boolean;
  rect?: { top: number; bottom: number };
  revealOrigin?: EditorViewportAnchorRevealOrigin;
};

export type EditorViewportAnchorRevealOrigin = {
  scrollTop: number;
  target: EditorScrollIntoViewTarget;
  policy: EditorScrollRevealPolicy;
};

export type EditorViewportAnchorLayout = {
  pages: readonly (VerticalSpan & { page: number; left: number })[];
  zoom: number;
};

export type EditorViewportScrollPosition = { left: number; top: number };

export type EditorViewportAnchorScroll = {
  scroll: EditorViewportScrollPosition;
  attachmentAchieved: boolean;
};

export type EditorViewportCenterMetrics = {
  scrollLeft: number;
  scrollTop: number;
  clientWidth: number;
  clientHeight: number;
};

export class EditorViewportAnchorState {
  #active: ActiveViewportAnchor | null = null;
  #preferredSelection: ViewportAnchor | null = null;

  #attachActive(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scroll: EditorViewportScrollPosition,
    revealOrigin?: EditorViewportAnchorRevealOrigin,
    source: ActiveViewportAnchor['source'] = 'viewport',
    attachmentPending = false,
  ): void {
    if (![geometry.pointX, geometry.pointY, scroll.left, scroll.top].every(Number.isFinite)) return;
    this.#active = {
      identity,
      source,
      pointAttachmentX: geometry.pointX - scroll.left,
      pointAttachmentY: geometry.pointY - scroll.top,
      attachmentPending,
      rect: geometry.rect,
      revealOrigin,
    };
  }

  get identity(): ViewportAnchor | null {
    return this.#active?.identity ?? null;
  }

  get pointAttachmentY(): number | null {
    return this.#active?.pointAttachmentY ?? null;
  }

  get pointAttachmentX(): number | null {
    return this.#active?.pointAttachmentX ?? null;
  }

  get preferredSelectionIdentity(): ViewportAnchor | null {
    return this.#preferredSelection;
  }

  get pendingViewportAttachment(): { identity: ViewportAnchor; focalX: number; focalY: number } | null {
    const attachment = this.viewportAttachment;
    return this.#active?.attachmentPending ? attachment : null;
  }

  get viewportAttachment(): { identity: ViewportAnchor; focalX: number; focalY: number } | null {
    const active = this.#active;
    if (!active || active.source !== 'viewport') return null;
    return { identity: active.identity, focalX: active.pointAttachmentX, focalY: active.pointAttachmentY };
  }

  clear(): void {
    this.#active = null;
    this.#preferredSelection = null;
  }

  needsSelectionAdoption(identity: ViewportAnchor): boolean {
    return this.#preferredSelection === null || stringify(this.#preferredSelection) !== stringify(identity);
  }

  attach(identity: ViewportAnchor, geometry: EditorViewportAnchorGeometry, scroll: EditorViewportScrollPosition): void {
    this.#attachActive(identity, geometry, scroll);
  }

  attachSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scroll: EditorViewportScrollPosition,
    revealOrigin?: EditorViewportAnchorRevealOrigin,
  ): void {
    this.#preferredSelection = identity;
    this.#attachActive(identity, geometry, scroll, revealOrigin, 'selection');
  }

  attachViewport(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scroll: EditorViewportScrollPosition,
    attachmentPending = false,
  ): void {
    this.#attachActive(identity, geometry, scroll, undefined, 'viewport', attachmentPending);
  }

  reattachViewport(geometry: EditorViewportAnchorGeometry, scroll: EditorViewportScrollPosition, attachmentPending = false): boolean {
    const active = this.#active;
    if (!active || active.source !== 'viewport') return false;
    this.#attachActive(active.identity, geometry, scroll, undefined, 'viewport', attachmentPending);
    return true;
  }

  adoptSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scroll: EditorViewportScrollPosition,
    clientHeight: number,
    visibleArea: EditorVisibleArea,
    preserveActiveAnchor: boolean,
  ): void {
    if (!this.needsSelectionAdoption(identity)) return;
    const activate =
      !preserveActiveAnchor && (this.#active !== null || this.canRetainAfterDirectScroll(geometry, scroll.top, clientHeight, visibleArea));
    this.#preferredSelection = identity;
    if (activate) this.#attachActive(identity, geometry, scroll, undefined, 'selection');
  }

  clearPreferredSelection(): void {
    this.#preferredSelection = null;
  }

  tryReactivatePreferredSelection(
    geometry: EditorViewportAnchorGeometry,
    scroll: EditorViewportScrollPosition,
    clientHeight: number,
    visibleArea: EditorVisibleArea,
  ): boolean {
    const identity = this.#preferredSelection;
    const rect = geometry.rect;
    const guard = resolveGuard(geometry, clientHeight, visibleArea);
    if (
      !identity ||
      !rect ||
      !guard ||
      !Number.isFinite(rect.top) ||
      !Number.isFinite(rect.bottom) ||
      rect.bottom < rect.top ||
      rect.top - scroll.top < guard.top ||
      rect.bottom - scroll.top > guard.bottom
    ) {
      return false;
    }
    this.#attachActive(identity, geometry, scroll, undefined, 'selection');
    return true;
  }

  publicationScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScroll: EditorViewportScrollPosition,
    maximumScroll: EditorViewportScrollPosition,
  ): EditorViewportAnchorScroll {
    const active = this.#active;
    if (
      !active ||
      ![geometry.pointX, geometry.pointY, maximumScroll.left, maximumScroll.top].every(Number.isFinite) ||
      maximumScroll.left < 0 ||
      maximumScroll.top < 0
    ) {
      return { scroll: currentScroll, attachmentAchieved: false };
    }
    const desiredScroll = {
      left: geometry.pointX - active.pointAttachmentX,
      top: geometry.pointY - active.pointAttachmentY,
    };
    const scroll = {
      left: clamp(desiredScroll.left, 0, maximumScroll.left),
      top: clamp(desiredScroll.top, 0, maximumScroll.top),
    };
    return {
      scroll,
      attachmentAchieved: scroll.left === desiredScroll.left && scroll.top === desiredScroll.top,
    };
  }

  publicationRevealScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollTop: number,
    clientHeight: number,
    scrollHeight: number,
    visibleArea: EditorVisibleArea,
    resolveReveal?: (origin: EditorViewportAnchorRevealOrigin) => number | null,
    currentScrollLeft = 0,
    maximumScrollLeft = 0,
  ): EditorViewportAnchorScroll {
    const exact = this.publicationScroll(
      geometry,
      { left: currentScrollLeft, top: currentScrollTop },
      { left: maximumScrollLeft, top: Math.max(0, scrollHeight - clientHeight) },
    );
    if (!rectHeightChanged(this.#active?.rect, geometry.rect)) return exact;
    const revealOrigin = this.#active?.revealOrigin;
    if (revealOrigin) {
      const reveal = resolveReveal?.(revealOrigin);
      if (reveal !== null && reveal !== undefined) {
        return {
          scroll: { ...exact.scroll, top: clamp(reveal, 0, Math.max(0, scrollHeight - clientHeight)) },
          attachmentAchieved: true,
        };
      }
    }
    return {
      scroll: {
        ...exact.scroll,
        top: this.resizeScroll(geometry, exact.scroll.top, clientHeight, scrollHeight, visibleArea),
      },
      attachmentAchieved: true,
    };
  }

  acceptGeometry(geometry: EditorViewportAnchorGeometry, scroll: EditorViewportScrollPosition): void {
    const active = this.#active;
    if (active) this.#attachActive(active.identity, geometry, scroll, active.revealOrigin, active.source);
  }

  deferAttachment(): void {
    if (this.#active) this.#active = { ...this.#active, attachmentPending: true };
  }

  finishRevealConvergence(): void {
    if (this.#active) this.#active = { ...this.#active, revealOrigin: undefined };
  }

  canRetainAfterDirectScroll(
    geometry: EditorViewportAnchorGeometry,
    scrollTop: number,
    clientHeight: number,
    visibleArea: EditorVisibleArea,
  ): boolean {
    const guard = resolveGuard(geometry, clientHeight, visibleArea);
    if (!guard) return false;
    const span = guardedSpan(geometry, guard.bottom - guard.top);
    return span.top - scrollTop >= guard.top && span.bottom - scrollTop <= guard.bottom;
  }

  resizeScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollTop: number,
    clientHeight: number,
    scrollHeight: number,
    visibleArea: EditorVisibleArea,
  ): number {
    if (this.canRetainAfterDirectScroll(geometry, currentScrollTop, clientHeight, visibleArea)) return currentScrollTop;

    const guard = resolveGuard(geometry, clientHeight, visibleArea);
    if (!guard) return currentScrollTop;
    const span = guardedSpan(geometry, guard.bottom - guard.top);
    return (
      resolveGuardedScrollTop({
        scrollTop: currentScrollTop,
        clientHeight,
        scrollHeight,
        targetTop: span.top,
        targetBottom: span.bottom,
        visibleArea,
      }) ?? currentScrollTop
    );
  }
}

export function resolveViewportAnchorGeometry(
  resolved: ResolvedViewportAnchor,
  layout: EditorViewportAnchorLayout,
): EditorViewportAnchorGeometry | null {
  const page = layout.pages[resolved.point.page_idx];
  if (!page || !Number.isFinite(layout.zoom) || layout.zoom <= 0) return null;
  const pointX = page.left + resolved.point.x * layout.zoom;
  const pointY = page.top + resolved.point.y * layout.zoom;
  if (!Number.isFinite(pointX) || !Number.isFinite(pointY)) return null;

  const rectPage = resolved.rect ? layout.pages[resolved.rect.page_idx] : undefined;
  const rect =
    rectPage && resolved.rect
      ? {
          top: rectPage.top + resolved.rect.rect.y * layout.zoom,
          bottom: rectPage.top + (resolved.rect.rect.y + resolved.rect.rect.height) * layout.zoom,
        }
      : undefined;
  return { pointX, pointY, rect };
}

export function viewportCenterAnchorPoint(
  snapshot: EditorSnapshot,
  layout: EditorViewportAnchorLayout,
  metrics: EditorViewportCenterMetrics,
  visibleArea: EditorVisibleArea,
): ViewportAnchorPoint | null {
  const { scrollLeft, scrollTop, clientWidth, clientHeight } = metrics;
  if (
    layout.pages.length === 0 ||
    !Number.isFinite(scrollLeft) ||
    !Number.isFinite(scrollTop) ||
    !Number.isFinite(clientWidth) ||
    !Number.isFinite(clientHeight) ||
    clientWidth <= 0 ||
    clientHeight <= 0
  ) {
    return null;
  }
  const topInset = Math.max(0, visibleArea.topInset);
  const visibleHeight = Math.max(0, clientHeight - topInset - Math.max(0, visibleArea.bottomInset));
  const viewportCenter = topInset + visibleHeight / 2;
  const contentX = scrollLeft + clientWidth / 2;
  const contentY = scrollTop + viewportCenter;

  const resolved = resolvePageAtY(layout.pages, snapshot.pageSizes, contentY, layout.zoom);
  if (!resolved) return null;
  const page = layout.pages[resolved.page];
  const size = snapshot.pageSizes[resolved.page];
  if (!page || !size) return null;
  return {
    page_idx: resolved.page,
    x: Math.max(0, Math.min((contentX - page.left) / layout.zoom, size.width)),
    y: resolved.y,
  };
}

function resolveGuard(
  geometry: EditorViewportAnchorGeometry,
  clientHeight: number,
  visibleArea: EditorVisibleArea,
): { top: number; bottom: number } | null {
  if (!Number.isFinite(geometry.pointY) || !Number.isFinite(clientHeight) || clientHeight <= 0) return null;
  const top = Math.max(0, visibleArea.topInset) + CURSOR_VISIBLE_MARGIN;
  const bottom = clientHeight - Math.max(0, visibleArea.bottomInset) - CURSOR_VISIBLE_MARGIN;
  return bottom > top ? { top, bottom } : null;
}

function guardedSpan(geometry: EditorViewportAnchorGeometry, guardHeight: number): { top: number; bottom: number } {
  const rect = geometry.rect;
  if (
    rect &&
    Number.isFinite(rect.top) &&
    Number.isFinite(rect.bottom) &&
    rect.bottom >= rect.top &&
    rect.bottom - rect.top <= guardHeight
  ) {
    return rect;
  }
  return { top: geometry.pointY, bottom: geometry.pointY };
}

function rectHeightChanged(previous: EditorViewportAnchorGeometry['rect'], current: EditorViewportAnchorGeometry['rect']): boolean {
  if (!current) return false;
  if (!previous) return true;
  return Math.abs(current.bottom - current.top - (previous.bottom - previous.top)) > 1;
}
