import { tick } from 'svelte';
import {
  clampDocumentZoom,
  clampPaginatedZoom,
  computeInitialPaginatedZoom,
  computePaginatedZoomBounds,
  RENDER_ZOOM_DEBOUNCE_MS,
  renderZoomForDisplay,
  zoomDiffers,
  zoomEquals,
} from '$lib/editor/zoom';
import type { Editor } from '$lib/editor/editor.svelte';

type ZoomAnchor = {
  pageIdx: number;
  x: number;
  y: number;
  focalX: number;
  focalY: number;
};

type EditorZoomControllerOptions = {
  editor: Editor;
  isPaginated: () => boolean;
  pageWidth: () => number;
  viewportWidth: () => number;
  getScrollContainer: () => HTMLElement | null;
};

export class EditorZoomController {
  static readonly WHEEL_SESSION_RESET_MS = 150;
  static readonly KEYBOARD_ZOOM_STEP = 0.1;

  displayZoom = $state(1);
  renderZoom = $state(1);

  #didApplyPaginatedInitialZoom = false;
  #renderZoomTimer: ReturnType<typeof setTimeout> | null = null;
  #wheelSessionTimer: ReturnType<typeof setTimeout> | null = null;
  #wheelRawZoom: number | null = null;
  #options: EditorZoomControllerOptions;

  constructor(options: EditorZoomControllerOptions) {
    this.#options = options;
  }

  destroy(): void {
    if (this.#renderZoomTimer) {
      clearTimeout(this.#renderZoomTimer);
      this.#renderZoomTimer = null;
    }
    if (this.#wheelSessionTimer) {
      clearTimeout(this.#wheelSessionTimer);
      this.#wheelSessionTimer = null;
    }
    this.#wheelRawZoom = null;
  }

  setZoom(nextZoom: number, { commitRender = false, source = 'programmatic' as 'wheel' | 'programmatic' } = {}): void {
    if (source !== 'wheel') {
      this.#wheelRawZoom = null;
    }

    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    if (!isPaginated || pageWidth <= 0) {
      if (zoomDiffers(this.displayZoom, 1)) {
        this.displayZoom = 1;
      }
      if (zoomDiffers(this.renderZoom, 1)) {
        this.renderZoom = 1;
      }
      return;
    }

    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : pageWidth;
    const clamped = clampPaginatedZoom({
      zoom: nextZoom,
      pageWidth,
      viewportWidth,
    });
    const nextRenderZoom = renderZoomForDisplay(clamped);

    if (zoomDiffers(this.displayZoom, clamped)) {
      this.displayZoom = clamped;
    }

    if (this.#renderZoomTimer) {
      clearTimeout(this.#renderZoomTimer);
      this.#renderZoomTimer = null;
    }

    if (commitRender) {
      if (zoomDiffers(this.renderZoom, nextRenderZoom)) {
        this.renderZoom = nextRenderZoom;
      }
      return;
    }

    this.#renderZoomTimer = setTimeout(() => {
      this.#renderZoomTimer = null;
      if (!this.#options.isPaginated()) {
        if (zoomDiffers(this.renderZoom, 1)) {
          this.renderZoom = 1;
        }
        return;
      }
      const latestRenderZoom = renderZoomForDisplay(this.displayZoom);
      if (zoomDiffers(this.renderZoom, latestRenderZoom)) {
        this.renderZoom = latestRenderZoom;
      }
    }, RENDER_ZOOM_DEBOUNCE_MS);
  }

  syncInitialZoom(): void {
    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    const viewportWidth = this.#options.viewportWidth();

    if (!isPaginated) {
      this.#didApplyPaginatedInitialZoom = false;
      this.setZoom(1, { commitRender: true });
      return;
    }

    if (this.#didApplyPaginatedInitialZoom || pageWidth <= 0 || viewportWidth <= 0) {
      return;
    }

    const initialZoom = computeInitialPaginatedZoom(pageWidth, viewportWidth);
    this.setZoom(initialZoom, { commitRender: true });
    this.#didApplyPaginatedInitialZoom = true;
  }

  clampCurrentZoomToBounds(): void {
    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    if (!isPaginated || pageWidth <= 0) {
      return;
    }

    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : pageWidth;
    const clamped = clampPaginatedZoom({
      zoom: this.displayZoom,
      pageWidth,
      viewportWidth,
    });
    if (zoomDiffers(clamped, this.displayZoom)) {
      this.setZoom(clamped, { commitRender: true });
    }
  }

  async handleWheel(event: WheelEvent): Promise<void> {
    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    if (!isPaginated || pageWidth <= 0) {
      return;
    }
    if (!event.ctrlKey && !event.metaKey) {
      return;
    }

    event.preventDefault();

    const zoomDelta = Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
    if (zoomDelta === 0) {
      return;
    }

    const bounds = computePaginatedZoomBounds(pageWidth);
    const wheelBaseZoom = this.#wheelRawZoom ?? this.displayZoom;
    const nextRawZoom = clampDocumentZoom(wheelBaseZoom * Math.exp(-zoomDelta / 240), bounds);
    this.#wheelRawZoom = nextRawZoom;
    this.#scheduleWheelSessionReset();

    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : pageWidth;
    const nextZoom = clampPaginatedZoom({
      zoom: nextRawZoom,
      pageWidth,
      viewportWidth,
    });
    if (zoomEquals(nextZoom, this.displayZoom)) {
      return;
    }

    const anchor = this.#createZoomAnchorFromClient(event.clientX, event.clientY);
    await this.#setZoomWithAnchor(nextZoom, anchor, 'wheel');
  }

  async zoomInByKeyboard(): Promise<void> {
    await this.#stepZoomByKeyboard(EditorZoomController.KEYBOARD_ZOOM_STEP);
  }

  async zoomOutByKeyboard(): Promise<void> {
    await this.#stepZoomByKeyboard(-EditorZoomController.KEYBOARD_ZOOM_STEP);
  }

  async resetByKeyboard(): Promise<void> {
    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    if (!isPaginated || pageWidth <= 0) {
      return;
    }
    const anchor = this.#createZoomAnchorFromViewportCenter();
    await this.#setZoomWithAnchor(1, anchor);
  }

  async zoomToClientPoint(nextZoom: number, clientX: number, clientY: number): Promise<void> {
    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    if (!isPaginated || pageWidth <= 0) {
      return;
    }

    const anchor = this.#createZoomAnchorFromClient(clientX, clientY);
    await this.#setZoomWithAnchor(nextZoom, anchor);
  }

  async #stepZoomByKeyboard(delta: number): Promise<void> {
    const isPaginated = this.#options.isPaginated();
    const pageWidth = this.#options.pageWidth();
    if (!isPaginated || pageWidth <= 0) {
      return;
    }
    const anchor = this.#createZoomAnchorFromViewportCenter();
    const nextZoom = this.displayZoom + delta;
    await this.#setZoomWithAnchor(nextZoom, anchor);
  }

  #createZoomAnchorFromClient(clientX: number, clientY: number): ZoomAnchor | null {
    const scrollContainer = this.#options.getScrollContainer();
    if (!scrollContainer) {
      return null;
    }
    const resolved = this.#options.editor.resolvePointerCoordinateFromClient(clientX, clientY);
    if (!resolved) {
      return null;
    }
    const rect = scrollContainer.getBoundingClientRect();
    return {
      ...resolved,
      focalX: clientX - rect.left,
      focalY: clientY - rect.top,
    };
  }

  #createZoomAnchorFromViewportCenter(): ZoomAnchor | null {
    const scrollContainer = this.#options.getScrollContainer();
    if (!scrollContainer) {
      return null;
    }
    const rect = scrollContainer.getBoundingClientRect();
    const clientX = rect.left + rect.width / 2;
    const clientY = rect.top + rect.height / 2;
    return this.#createZoomAnchorFromClient(clientX, clientY);
  }

  async #setZoomWithAnchor(nextZoom: number, anchor: ZoomAnchor | null, source: 'wheel' | 'programmatic' = 'programmatic'): Promise<void> {
    const previousZoom = this.displayZoom;
    this.setZoom(nextZoom, { source });
    if (!anchor || zoomEquals(previousZoom, this.displayZoom)) {
      return;
    }
    await this.#syncZoomAnchor(anchor, this.displayZoom);
  }

  #scheduleWheelSessionReset(): void {
    if (this.#wheelSessionTimer) {
      clearTimeout(this.#wheelSessionTimer);
    }
    this.#wheelSessionTimer = setTimeout(() => {
      this.#wheelSessionTimer = null;
      this.#wheelRawZoom = null;
    }, EditorZoomController.WHEEL_SESSION_RESET_MS);
  }

  async #syncZoomAnchor(anchor: ZoomAnchor, zoom: number): Promise<void> {
    const scrollContainer = this.#options.getScrollContainer();
    if (!scrollContainer) {
      return;
    }

    const pageCount = this.#options.editor.layout?.pages.length ?? 0;
    if (pageCount === 0) {
      return;
    }

    const pageIdx = Math.max(0, Math.min(anchor.pageIdx, pageCount - 1));
    const pageEl = this.#options.editor.pageContainerEls[pageIdx];
    if (!pageEl) {
      return;
    }

    await tick();

    const pageRect = pageEl.getBoundingClientRect();
    const scrollRect = scrollContainer.getBoundingClientRect();

    const targetClientX = scrollRect.left + anchor.focalX;
    const targetClientY = scrollRect.top + anchor.focalY;
    const anchoredClientX = pageRect.left + anchor.x * zoom;
    const anchoredClientY = pageRect.top + anchor.y * zoom;

    const deltaX = anchoredClientX - targetClientX;
    const deltaY = anchoredClientY - targetClientY;
    if (Math.abs(deltaX) > 0.5 || Math.abs(deltaY) > 0.5) {
      scrollContainer.scrollBy({ left: deltaX, top: deltaY });
    }
  }
}
