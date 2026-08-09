import { clamp } from '@typie/ui/utils';
import stringify from 'fast-json-stable-stringify';
import { CURSOR_VISIBLE_MARGIN } from './constants';
import { resolveGuardedScrollTop } from './scroll';
import type { ResolvedViewportAnchor, ViewportAnchor, ViewportAnchorPoint } from '@typie/editor-ffi/browser';
import type { EditorSnapshot } from './editor.svelte';
import type { VerticalSpan } from './required-surface-pages';
import type { EditorVisibleArea } from './scroll';
import type { EditorScrollIntoViewTarget, EditorScrollRevealPolicy } from './scroll.svelte';

export type EditorViewportAnchorGeometry = {
  pointY: number;
  rect?: { top: number; bottom: number };
};

type ActiveViewportAnchor = {
  identity: ViewportAnchor;
  pointAttachmentY: number;
  rect?: { top: number; bottom: number };
  revealOrigin?: EditorViewportAnchorRevealOrigin;
};

export type EditorViewportAnchorRevealOrigin = {
  scrollTop: number;
  target: EditorScrollIntoViewTarget;
  policy: EditorScrollRevealPolicy;
};

export type EditorViewportAnchorLayout = {
  pages: readonly (VerticalSpan & { page: number })[];
  zoom: number;
};

export class EditorViewportAnchorState {
  #active: ActiveViewportAnchor | null = null;
  #preferredSelection: ViewportAnchor | null = null;

  #attachActive(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollTop: number,
    revealOrigin?: EditorViewportAnchorRevealOrigin,
  ): void {
    if (!Number.isFinite(geometry.pointY) || !Number.isFinite(scrollTop)) return;
    this.#active = { identity, pointAttachmentY: geometry.pointY - scrollTop, rect: geometry.rect, revealOrigin };
  }

  get identity(): ViewportAnchor | null {
    return this.#active?.identity ?? null;
  }

  get pointAttachmentY(): number | null {
    return this.#active?.pointAttachmentY ?? null;
  }

  get preferredSelectionIdentity(): ViewportAnchor | null {
    return this.#preferredSelection;
  }

  clear(): void {
    this.#active = null;
    this.#preferredSelection = null;
  }

  needsSelectionAdoption(identity: ViewportAnchor): boolean {
    return this.#preferredSelection === null || stringify(this.#preferredSelection) !== stringify(identity);
  }

  attach(identity: ViewportAnchor, geometry: EditorViewportAnchorGeometry, scrollTop: number): void {
    this.#attachActive(identity, geometry, scrollTop);
  }

  attachSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollTop: number,
    revealOrigin?: EditorViewportAnchorRevealOrigin,
  ): void {
    this.#preferredSelection = identity;
    this.#attachActive(identity, geometry, scrollTop, revealOrigin);
  }

  attachViewport(identity: ViewportAnchor, geometry: EditorViewportAnchorGeometry, scrollTop: number): void {
    this.#attachActive(identity, geometry, scrollTop);
  }

  adoptSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollTop: number,
    clientHeight: number,
    visibleArea: EditorVisibleArea,
    preserveActiveAnchor: boolean,
  ): void {
    if (!this.needsSelectionAdoption(identity)) return;
    const activate =
      !preserveActiveAnchor && (this.#active !== null || this.canRetainAfterDirectScroll(geometry, scrollTop, clientHeight, visibleArea));
    this.#preferredSelection = identity;
    if (activate) this.#attachActive(identity, geometry, scrollTop);
  }

  clearPreferredSelection(): void {
    this.#preferredSelection = null;
  }

  tryReactivatePreferredSelection(
    geometry: EditorViewportAnchorGeometry,
    scrollTop: number,
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
      rect.top - scrollTop < guard.top ||
      rect.bottom - scrollTop > guard.bottom
    ) {
      return false;
    }
    this.#attachActive(identity, geometry, scrollTop);
    return true;
  }

  publicationScroll(geometry: EditorViewportAnchorGeometry, currentScrollTop: number, maximumScrollTop: number): number {
    const attachment = this.#active?.pointAttachmentY;
    if (attachment === undefined || !Number.isFinite(geometry.pointY) || !Number.isFinite(maximumScrollTop) || maximumScrollTop < 0) {
      return currentScrollTop;
    }
    return clamp(geometry.pointY - attachment, 0, maximumScrollTop);
  }

  publicationRevealScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollTop: number,
    clientHeight: number,
    scrollHeight: number,
    visibleArea: EditorVisibleArea,
    resolveReveal?: (origin: EditorViewportAnchorRevealOrigin) => number | null,
  ): number {
    const exact = this.publicationScroll(geometry, currentScrollTop, Math.max(0, scrollHeight - clientHeight));
    if (!rectHeightChanged(this.#active?.rect, geometry.rect)) return exact;
    const revealOrigin = this.#active?.revealOrigin;
    if (revealOrigin) {
      const reveal = resolveReveal?.(revealOrigin);
      if (reveal !== null && reveal !== undefined) return clamp(reveal, 0, Math.max(0, scrollHeight - clientHeight));
    }
    return this.resizeScroll(geometry, exact, clientHeight, scrollHeight, visibleArea);
  }

  acceptGeometry(geometry: EditorViewportAnchorGeometry, scrollTop: number): void {
    const active = this.#active;
    if (active) this.#attachActive(active.identity, geometry, scrollTop, active.revealOrigin);
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
  const pointY = page.top + resolved.point.y * layout.zoom;
  if (!Number.isFinite(pointY)) return null;

  const rectPage = resolved.rect ? layout.pages[resolved.rect.page_idx] : undefined;
  const rect =
    rectPage && resolved.rect
      ? {
          top: rectPage.top + resolved.rect.rect.y * layout.zoom,
          bottom: rectPage.top + (resolved.rect.rect.y + resolved.rect.rect.height) * layout.zoom,
        }
      : undefined;
  return { pointY, rect };
}

export function viewportCenterAnchorPoint(
  snapshot: EditorSnapshot,
  layout: EditorViewportAnchorLayout,
  scrollTop: number,
  clientHeight: number,
  visibleArea: EditorVisibleArea,
): ViewportAnchorPoint | null {
  if (layout.pages.length === 0 || !Number.isFinite(scrollTop) || !Number.isFinite(clientHeight) || clientHeight <= 0) return null;
  const topInset = Math.max(0, visibleArea.topInset);
  const visibleHeight = Math.max(0, clientHeight - topInset - Math.max(0, visibleArea.bottomInset));
  const viewportCenter = topInset + visibleHeight / 2;
  const contentY = scrollTop + viewportCenter;

  let page = layout.pages.at(-1);
  for (let index = 0; index < layout.pages.length; index += 1) {
    const next = layout.pages[index + 1];
    if (!next || contentY < next.top) {
      page = layout.pages[index];
      break;
    }
  }
  if (!page) return null;
  const size = snapshot.pageSizes[page.page];
  if (!size) return null;
  return {
    page_idx: page.page,
    x: size.width / 2,
    y: (contentY - page.top) / layout.zoom,
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
