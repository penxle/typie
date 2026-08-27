import { tick } from 'svelte';
import {
  clampDocumentLayoutZoom,
  clampDocumentZoom,
  computeDocumentZoomBounds,
  computeInitialDocumentZoom,
  RENDER_ZOOM_DEBOUNCE_MS,
  renderZoomForDisplay,
  zoomDiffers,
  zoomEquals,
} from '$lib/editor-ffi/zoom';
import type { ScrollViewport } from '@typie/ui/utils';
import type { Editor } from '$lib/editor-ffi/editor.svelte';
import type { DocumentZoomLayout } from '$lib/editor-ffi/zoom';

type ZoomAnchor = {
  page: number;
  x: number;
  y: number;
  focalX: number;
  focalY: number;
};

type EditorZoomControllerOptions = {
  editor: Editor;
  layout: () => DocumentZoomLayout | null;
  viewportWidth: () => number;
  getScrollViewport: () => ScrollViewport | null | undefined;
  attachViewportAnchor?: (point: Pick<ZoomAnchor, 'page' | 'x' | 'y'>) => void;
};

export class EditorZoomController {
  static readonly WHEEL_RAW_ZOOM_RESET_MS = 150;
  static readonly KEYBOARD_ZOOM_STEP = 0.1;

  #initializedLayoutKey: string | null = null;
  #renderZoomTimer: ReturnType<typeof setTimeout> | null = null;
  #wheelRawZoomResetTimer: ReturnType<typeof setTimeout> | null = null;
  #wheelRawZoom: number | null = null;
  #options: EditorZoomControllerOptions;

  displayZoom = $state(1);
  renderZoom = $state(1);

  constructor(options: EditorZoomControllerOptions) {
    this.#options = options;
  }

  async #stepZoomByKeyboard(delta: number): Promise<void> {
    if (!this.#options.layout()) return;
    const anchor = this.#createZoomAnchorFromViewportCenter();
    const nextZoom = this.displayZoom + delta;
    await this.#setZoomWithAnchor(nextZoom, anchor);
  }

  #createZoomAnchorFromClient(clientX: number, clientY: number): ZoomAnchor | null {
    const viewport = this.#options.getScrollViewport();
    if (!viewport) {
      return null;
    }
    const resolved = this.#options.editor.clientToLocal(clientX, clientY);
    if (!resolved) {
      return null;
    }
    const rect = viewport.getRect();
    return {
      ...resolved,
      focalX: clientX - rect.left,
      focalY: clientY - rect.top,
    };
  }

  #createZoomAnchorFromViewportCenter(): ZoomAnchor | null {
    const viewport = this.#options.getScrollViewport();
    if (!viewport) {
      return null;
    }
    const rect = viewport.getRect();
    const clientX = rect.left + (rect.right - rect.left) / 2;
    const clientY = rect.top + (rect.bottom - rect.top) / 2;
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

  #scheduleWheelRawZoomReset(): void {
    if (this.#wheelRawZoomResetTimer) {
      clearTimeout(this.#wheelRawZoomResetTimer);
    }
    this.#wheelRawZoomResetTimer = setTimeout(() => {
      this.#wheelRawZoomResetTimer = null;
      this.#wheelRawZoom = null;
    }, EditorZoomController.WHEEL_RAW_ZOOM_RESET_MS);
  }

  #resetWheelRawZoom(): void {
    if (this.#wheelRawZoomResetTimer) {
      clearTimeout(this.#wheelRawZoomResetTimer);
      this.#wheelRawZoomResetTimer = null;
    }
    this.#wheelRawZoom = null;
  }

  async #syncZoomAnchor(anchor: ZoomAnchor, zoom: number): Promise<void> {
    const viewport = this.#options.getScrollViewport();
    if (!viewport) {
      return;
    }

    const pageCount = this.#options.editor.pageSizes.length;
    if (pageCount === 0) {
      return;
    }

    const page = Math.max(0, Math.min(anchor.page, pageCount - 1));
    const pageEl = this.#options.editor.pageEls[page];
    if (!pageEl) {
      return;
    }

    await tick();

    const pageRect = pageEl.getBoundingClientRect();
    const scrollRect = viewport.getRect();

    const targetClientX = scrollRect.left + anchor.focalX;
    const targetClientY = scrollRect.top + anchor.focalY;
    const anchoredClientX = pageRect.left + anchor.x * zoom;
    const anchoredClientY = pageRect.top + anchor.y * zoom;

    const deltaX = anchoredClientX - targetClientX;
    const deltaY = anchoredClientY - targetClientY;
    if (Math.abs(deltaX) > 0.5 || Math.abs(deltaY) > 0.5) {
      viewport.scrollBy(deltaX, deltaY);
    }
    this.#options.attachViewportAnchor?.(anchor);
  }

  destroy(): void {
    if (this.#renderZoomTimer) {
      clearTimeout(this.#renderZoomTimer);
      this.#renderZoomTimer = null;
    }
    this.#resetWheelRawZoom();
  }

  setZoom(nextZoom: number, { commitRender = false, source = 'programmatic' as 'wheel' | 'programmatic' } = {}): void {
    if (source !== 'wheel') {
      this.#resetWheelRawZoom();
    }

    if (this.#renderZoomTimer) {
      clearTimeout(this.#renderZoomTimer);
      this.#renderZoomTimer = null;
    }

    const layout = this.#options.layout();
    if (!layout) {
      if (zoomDiffers(this.displayZoom, 1)) {
        this.displayZoom = 1;
      }
      if (zoomDiffers(this.renderZoom, 1)) {
        this.renderZoom = 1;
      }
      return;
    }

    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : 1;
    const clamped = clampDocumentLayoutZoom({
      zoom: nextZoom,
      layout,
      viewportWidth,
    });
    const nextRenderZoom = renderZoomForDisplay(clamped);

    if (zoomDiffers(this.displayZoom, clamped)) {
      this.displayZoom = clamped;
    }

    if (commitRender) {
      if (zoomDiffers(this.renderZoom, nextRenderZoom)) {
        this.renderZoom = nextRenderZoom;
      }
      return;
    }

    this.#renderZoomTimer = setTimeout(() => {
      this.#renderZoomTimer = null;
      if (!this.#options.layout()) {
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
    const layout = this.#options.layout();
    const viewportWidth = this.#options.viewportWidth();

    if (!layout) {
      this.#initializedLayoutKey = null;
      this.setZoom(1, { commitRender: true });
      return;
    }

    if (viewportWidth <= 0) return;

    const width = layout.type === 'continuous' ? layout.maxWidth : layout.pageWidth;
    const key = `${layout.type}:${width}`;
    if (this.#initializedLayoutKey === key) return;

    this.#initializedLayoutKey = key;
    const initialZoom = computeInitialDocumentZoom(layout, viewportWidth);
    this.setZoom(initialZoom, { commitRender: true });
  }

  clampCurrentZoomToBounds(): void {
    const layout = this.#options.layout();
    if (!layout) return;

    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : 1;
    const clamped = clampDocumentLayoutZoom({
      zoom: this.displayZoom,
      layout,
      viewportWidth,
    });
    if (zoomDiffers(clamped, this.displayZoom)) {
      this.setZoom(clamped, { commitRender: true });
    }
  }

  async handleWheel(event: WheelEvent): Promise<void> {
    const layout = this.#options.layout();
    if (!layout) return;

    const zoomDelta = Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
    if (zoomDelta === 0) {
      return;
    }

    if (!event.metaKey && !event.ctrlKey) {
      return;
    }

    if (event.cancelable) {
      event.preventDefault();
    }
    this.#scheduleWheelRawZoomReset();

    const bounds = computeDocumentZoomBounds(layout);
    const wheelBaseZoom = this.#wheelRawZoom ?? this.displayZoom;
    const nextRawZoom = clampDocumentZoom(wheelBaseZoom * Math.exp(-zoomDelta / 240), bounds);
    this.#wheelRawZoom = nextRawZoom;

    const viewportWidth = this.#options.viewportWidth() > 0 ? this.#options.viewportWidth() : 1;
    const nextZoom = clampDocumentLayoutZoom({
      zoom: nextRawZoom,
      layout,
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
    if (!this.#options.layout()) return;
    const anchor = this.#createZoomAnchorFromViewportCenter();
    await this.#setZoomWithAnchor(1, anchor);
  }

  async zoomToClientPoint(nextZoom: number, clientX: number, clientY: number): Promise<void> {
    if (!this.#options.layout()) return;

    const anchor = this.#createZoomAnchorFromClient(clientX, clientY);
    await this.#setZoomWithAnchor(nextZoom, anchor);
  }

  commitRenderZoom(): void {
    this.setZoom(this.displayZoom, { commitRender: true });
  }
}
