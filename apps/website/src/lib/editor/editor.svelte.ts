import { Application, getMemory } from '@typie/editor';
import icuPostcardUrl from '@typie/editor/pkg/icu_data.postcard?url';
import notoPhantomUrl from '@typie/editor/pkg/Noto-Phantom.ttf?url';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { FRAGMENT_MIME } from './constants';
import { ensureRequiredFonts, ensureRequiredScripts, getAvailableFontsMap, loadEmojiFallback, loadInitialFonts } from './fonts';
import { calculateRelativePosition, findNearestPageCoordinate, getPageIndex, idleCallback } from './utils';
import type { Editor as WasmEditor } from '@typie/editor';
import type { ThemeColors } from './theme';
import type { Cmd, ExternalElement, LayoutMode, Mark, MarkType, Message, Rect, SelectionStats, WritingSystem } from './types';

const CLICK_INTERVAL = 500;
const CLICK_DISTANCE = 5;

const EMPTY_DRAG_IMAGE = new Image();
EMPTY_DRAG_IMAGE.src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';

export type EditorOptions = {
  theme: ThemeColors;
  snapshot?: Uint8Array;
  onDocChanged?: () => void;
};

export class Editor {
  #application: Application | null = null;
  #wasmEditor: WasmEditor | null = null;
  #running = false;
  #rafId: number | null = null;
  #pendingFontLoad = false;
  #onDocChanged?: () => void;

  renderVersion = $state(0);

  layout = $state({
    pageCount: 0,
    pageWidth: 0,
    pageHeights: [] as number[],
    layoutMode: {
      type: 'paginated',
      pageWidth: 794,
      pageHeight: 1123,
      pageMarginTop: 96,
      pageMarginBottom: 96,
      pageMarginLeft: 96,
      pageMarginRight: 96,
    } as LayoutMode,
  });

  selection = $state({
    stats: {
      blockCount: 0,
      paragraphCount: 0,
      uniformAlign: undefined,
      uniformLineHeight: undefined,
    } as SelectionStats,
  });

  activeMarks = $state({
    uniformMarks: [] as Mark[],
    mixedMarks: [] as MarkType[],
  });

  settings = $state({
    paragraphIndent: 1,
    blockGap: 0,
  });

  externalElements = $state<ExternalElement[]>([]);

  enabledActions = $state(new SvelteSet<string>());

  cursor = $state({
    pageIdx: -1,
    bounds: null as Rect | null,
    show: false,
  });

  inputElement = $state<HTMLInputElement | null>(null);

  pointer = $state({
    isPressed: false,
    currentHoverTarget: null as HTMLElement | null,
  });

  isDraggable = $state(false);

  contextMenu = $state({
    x: 0,
    y: 0,
    isOpen: false,
  });

  isFocused = $state(false);

  pageVisibility = new SvelteMap<number, number>();

  extensionArea = $state({
    containerEl: null as HTMLElement | null,
    pageElements: [] as HTMLElement[],
  });

  pageContainerEls = $state<HTMLDivElement[]>([]);

  #lastClickTime = 0;
  #lastClickPos: { x: number; y: number } | null = null;
  #clickCount = 0;

  async initialize(options: EditorOptions): Promise<void> {
    if (this.#wasmEditor) return;

    this.#onDocChanged = options.onDocChanged;

    const app = new Application();
    this.#application = app;

    const [icuPostcard, notoPhantom] = await Promise.all(
      [icuPostcardUrl, notoPhantomUrl].map((url) => fetch(url).then((res) => res.arrayBuffer())),
    );

    app.loadIcuData(new Uint8Array(icuPostcard));
    app.registerFallbackFont('Noto-Phantom', 400, new Uint8Array(notoPhantom));
    app.setAvailableFonts(getAvailableFontsMap());

    const scaleFactor = window.devicePixelRatio * (window.visualViewport?.scale || 1);
    const wasmEditor = app.createEditor(scaleFactor, options.snapshot);
    this.#wasmEditor = wasmEditor;

    this.dispatch({
      type: 'initialize',
      theme: options.theme,
    });

    this.dispatch({ type: 'navigate', direction: 'documentStart', extend: false });

    this.#start();

    Promise.all([loadInitialFonts(app), loadEmojiFallback(app)]).then(() => {
      this.dispatch({ type: 'fontsLoaded' });
    });
  }

  #start(): void {
    if (this.#running) return;
    this.#running = true;
    this.#tick();
  }

  #stop(): void {
    this.#running = false;
    if (this.#rafId !== null) {
      cancelAnimationFrame(this.#rafId);
      this.#rafId = null;
    }
  }

  #tick = (): void => {
    if (!this.#running) return;

    const cmds = this.#wasmEditor?.tick() as Cmd[] | null;
    if (cmds) {
      this.#processCommands(cmds);
    }

    idleCallback(() => {
      this.#wasmEditor?.flush();
    });

    this.#rafId = requestAnimationFrame(this.#tick);
  };

  #processCommands(cmds: Cmd[]): void {
    for (const cmd of cmds) {
      switch (cmd.type) {
        case 'docChanged': {
          this.#onDocChanged?.();
          break;
        }

        case 'settingsChanged': {
          this.settings.paragraphIndent = cmd.paragraphIndent;
          this.settings.blockGap = cmd.blockGap;
          break;
        }

        case 'layoutChanged': {
          this.layout.pageCount = cmd.pageCount;
          this.layout.layoutMode = cmd.layoutMode;
          this.layout.pageWidth = cmd.pageWidth;
          this.layout.pageHeights = cmd.pageHeights;
          break;
        }

        case 'cursorChanged': {
          if (cmd.pageIdx !== null && cmd.pageIdx !== undefined && cmd.bounds) {
            this.cursor.pageIdx = cmd.pageIdx;
            this.cursor.bounds = cmd.bounds;
            this.cursor.show = cmd.show;
          } else {
            this.cursor.pageIdx = -1;
            this.cursor.bounds = null;
            this.cursor.show = false;
          }
          break;
        }

        case 'selectionChanged': {
          this.selection.stats = cmd.stats;
          break;
        }

        case 'activeMarksChanged': {
          this.activeMarks.uniformMarks = cmd.uniformMarks;
          this.activeMarks.mixedMarks = cmd.mixedMarks;
          break;
        }

        case 'externalElementChanged': {
          this.externalElements = cmd.elements;
          break;
        }

        case 'pointerStyleChanged': {
          if (this.pointer.currentHoverTarget) {
            this.pointer.currentHoverTarget.style.cursor = cmd.style;
          }
          break;
        }

        case 'fontsRequired': {
          this.#handleFontsRequired(cmd.fonts);
          break;
        }

        case 'writingSystemRequired': {
          this.#handleWritingSystemsRequired(cmd.systems);
          break;
        }

        case 'renderRequired': {
          this.renderVersion++;
          break;
        }

        case 'enabledActionsChanged': {
          this.enabledActions = new SvelteSet(cmd.enabled);
          break;
        }
      }
    }
  }

  #handleFontsRequired(fonts: [string, number][]): void {
    if (fonts.length === 0 || !this.#application || this.#pendingFontLoad) return;

    this.#pendingFontLoad = true;
    ensureRequiredFonts(this.#application, fonts).then((loaded) => {
      this.#pendingFontLoad = false;
      if (loaded) {
        this.dispatch({ type: 'fontsLoaded' });
      }
    });
  }

  #handleWritingSystemsRequired(systems: WritingSystem[]): void {
    if (systems.length === 0 || !this.#application) return;

    ensureRequiredScripts(this.#application, systems).then((loaded) => {
      if (loaded) {
        this.dispatch({ type: 'fontsLoaded' });
      }
    });
  }

  dispatch(message: Message): Editor {
    this.#wasmEditor?.enqueueMessage(message);

    return this;
  }

  updatePageVisibility(pageIndex: number, ratio: number): void {
    if (ratio > 0) {
      this.pageVisibility.set(pageIndex, ratio);
    } else {
      this.pageVisibility.delete(pageIndex);
    }
  }

  renderPage(pageIdx: number) {
    return this.#wasmEditor?.renderPage(pageIdx);
  }

  getSnapshot(): Uint8Array | undefined {
    return this.#wasmEditor?.getSnapshot();
  }

  inspectState(): string | undefined {
    return this.#wasmEditor?.inspectState();
  }

  inspectStateAsMacro(): string | undefined {
    return this.#wasmEditor?.inspectStateAsMacro();
  }

  inspectSelectionAsFragmentMacro(): string | undefined {
    return this.#wasmEditor?.inspectSelectionAsFragmentMacro();
  }

  #getClickCount(x: number, y: number, timestamp: number): number {
    const isSamePosition = this.#lastClickPos !== null && Math.hypot(x - this.#lastClickPos.x, y - this.#lastClickPos.y) < CLICK_DISTANCE;
    const isWithinInterval = timestamp - this.#lastClickTime < CLICK_INTERVAL;

    if (isSamePosition && isWithinInterval) {
      this.#clickCount = this.#clickCount >= 3 ? 1 : this.#clickCount + 1;
    } else {
      this.#clickCount = 1;
    }

    this.#lastClickTime = timestamp;
    this.#lastClickPos = { x, y };
    return this.#clickCount;
  }

  #resolvePointerCoordinate(
    e: MouseEvent | PointerEvent,
    targetEl: HTMLElement,
  ): { pageIdx: number; x: number; y: number; pageElement: HTMLElement; isExtensionArea: boolean } | null {
    const pageIdx = getPageIndex(targetEl);

    if (pageIdx !== -1) {
      let pageElement: HTMLElement | null = targetEl;
      while (pageElement && pageElement.dataset && !pageElement.dataset.pageIndex) {
        pageElement = pageElement.parentElement;
      }
      const point = calculateRelativePosition(targetEl, e);
      return {
        pageIdx,
        x: point.x,
        y: point.y,
        pageElement: pageElement ?? targetEl,
        isExtensionArea: false,
      };
    }

    if (this.layout.layoutMode.type === 'continuous') {
      const { containerEl, pageElements } = this.extensionArea;
      if (containerEl && pageElements.length > 0) {
        const coord = findNearestPageCoordinate(e, pageElements, this.layout.pageWidth);
        if (coord) {
          return {
            pageIdx: coord.pageIdx,
            x: coord.x,
            y: coord.y,
            pageElement: coord.pageElement,
            isExtensionArea: true,
          };
        }
      }
    }

    return null;
  }

  handlePointerDown(e: PointerEvent): void {
    if (!(e.target instanceof HTMLElement)) return;

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) {
      this.isDraggable = false;
      return;
    }

    const { pageIdx, x, y, pageElement } = resolved;

    const rect = pageElement.getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const relY = e.clientY - rect.top;
    this.isDraggable = this.canDragAt(pageIdx, relX, relY);

    if (e.button === 0) {
      if (!this.isDraggable) {
        e.target.setPointerCapture(e.pointerId);
      }
      this.pointer.isPressed = true;
    }

    const count = e.button === 0 ? this.#getClickCount(e.clientX, e.clientY, e.timeStamp) : 1;

    this.dispatch({
      type: 'pointerDown',
      pageIdx,
      x,
      y,
      clickCount: count,
      shiftKey: e.shiftKey,
      isPrimary: e.button === 0,
    });
  }

  handlePointerMove(e: PointerEvent): void {
    const targetEl = document.elementFromPoint(e.clientX, e.clientY);
    if (!(targetEl instanceof HTMLElement)) return;

    const resolved = this.#resolvePointerCoordinate(e, targetEl);
    if (!resolved) return;

    const { pageIdx, x, y, isExtensionArea } = resolved;

    this.pointer.currentHoverTarget = isExtensionArea ? (this.extensionArea.containerEl ?? targetEl) : targetEl;

    this.dispatch({
      type: 'pointerMove',
      pageIdx,
      x,
      y,
      isPressed: this.pointer.isPressed,
    });
  }

  handlePointerUp(e: PointerEvent): void {
    this.isDraggable = false;

    if (!(e.target instanceof HTMLElement)) return;

    e.target.releasePointerCapture(e.pointerId);

    const targetEl = document.elementFromPoint(e.clientX, e.clientY);
    if (!(targetEl instanceof HTMLElement)) return;

    const resolved = this.#resolvePointerCoordinate(e, targetEl);
    if (!resolved) {
      this.pointer.isPressed = false;
      return;
    }

    const { pageIdx, x, y } = resolved;

    this.dispatch({
      type: 'pointerUp',
      pageIdx,
      x,
      y,
    });

    this.pointer.isPressed = false;
  }

  handleContextMenu(e: MouseEvent): void {
    if (!(e.target instanceof HTMLElement)) return;

    const index = getPageIndex(e.target);
    if (index === -1) return;

    e.preventDefault();

    const point = calculateRelativePosition(e.target, e);
    this.dispatch({
      type: 'pointerDown',
      pageIdx: index,
      x: point.x,
      y: point.y,
      clickCount: 1,
      shiftKey: e.shiftKey,
      isPrimary: false,
    });

    this.contextMenu.x = e.clientX;
    this.contextMenu.y = e.clientY;
    this.contextMenu.isOpen = true;
  }

  handleOverlayContextMenu(e: MouseEvent, overlayEl: HTMLElement): void {
    e.preventDefault();

    overlayEl.style.visibility = 'hidden';
    const targetEl = document.elementFromPoint(e.clientX, e.clientY);
    overlayEl.style.visibility = 'visible';

    if (!(targetEl instanceof HTMLElement)) return;

    const index = getPageIndex(targetEl);
    if (index === -1) return;

    const point = calculateRelativePosition(targetEl, e);
    this.dispatch({
      type: 'pointerDown',
      pageIdx: index,
      x: point.x,
      y: point.y,
      clickCount: 1,
      shiftKey: e.shiftKey,
      isPrimary: false,
    });

    this.contextMenu.x = e.clientX;
    this.contextMenu.y = e.clientY;
    this.contextMenu.isOpen = true;
  }

  closeContextMenu(): void {
    this.contextMenu.isOpen = false;
  }

  async handleCopy(): Promise<void> {
    const data = this.getClipboardData();
    if (data) {
      await this.#writeToClipboard(data.fragment, data.html, data.text);
    }
    this.closeContextMenu();
  }

  async handleCut(): Promise<void> {
    const data = this.getClipboardData();
    if (data) {
      await this.#writeToClipboard(data.fragment, data.html, data.text);
      this.dispatch({ type: 'deleteSelection' });
    }
    this.closeContextMenu();
  }

  async #writeToClipboard(fragment: string, html: string, text: string): Promise<void> {
    try {
      const items = new ClipboardItem({
        [FRAGMENT_MIME]: new Blob([fragment], { type: FRAGMENT_MIME }),
        'text/html': new Blob([html], { type: 'text/html' }),
        'text/plain': new Blob([text], { type: 'text/plain' }),
      });
      await navigator.clipboard.write([items]);
    } catch {
      await navigator.clipboard.writeText(text);
    }
  }

  async handlePaste(): Promise<void> {
    try {
      const items = await navigator.clipboard.read();
      let fragment: string | undefined = undefined;
      let html: string | undefined = undefined;
      let text = '';

      for (const item of items) {
        if (item.types.includes(FRAGMENT_MIME)) {
          const blob = await item.getType(FRAGMENT_MIME);
          fragment = await blob.text();
        }
        if (item.types.includes('text/html')) {
          const blob = await item.getType('text/html');
          html = await blob.text();
        }
        if (item.types.includes('text/plain')) {
          const blob = await item.getType('text/plain');
          text = await blob.text();
        }
      }

      this.dispatch({ type: 'paste', fragment, html, text });
    } catch {
      const text = await navigator.clipboard.readText();
      this.dispatch({ type: 'paste', fragment: undefined, html: undefined, text });
    }
    this.closeContextMenu();
  }

  handleSelectAll(): void {
    this.dispatch({ type: 'selectAll' });
    this.closeContextMenu();
  }

  handleDragStart(e: DragEvent): void {
    const targetEl = document.elementFromPoint(e.clientX, e.clientY);
    if (!(targetEl instanceof HTMLElement)) return;

    let pageElement: HTMLElement | null = targetEl;
    while (pageElement && pageElement.dataset && !pageElement.dataset.pageIndex) {
      pageElement = pageElement.parentElement;
    }

    if (!pageElement || !pageElement.dataset.pageIndex) {
      return;
    }

    const pageIdx = Number.parseInt(pageElement.dataset.pageIndex);
    const rect = pageElement.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (!this.canDragAt(pageIdx, x, y)) {
      e.preventDefault();
      return;
    }

    const data = this.getClipboardData();
    if (e.dataTransfer && data) {
      e.dataTransfer.setData(FRAGMENT_MIME, data.fragment);
      e.dataTransfer.setData('text/html', data.html);
      e.dataTransfer.setData('text/plain', data.text);
      e.dataTransfer.effectAllowed = 'copyMove';

      const visiblePages = [...this.pageVisibility.keys()];
      const dragImage = this.#renderDragImage(visiblePages, pageIdx);
      if (dragImage) {
        dragImage.element.style.position = 'absolute';
        dragImage.element.style.top = '-9999px';
        dragImage.element.style.left = '-9999px';
        document.body.append(dragImage.element);

        const offsetX = e.clientX - rect.left - dragImage.offsetX;
        const offsetY = e.clientY - rect.top - dragImage.offsetY;
        e.dataTransfer.setDragImage(dragImage.element, offsetX, offsetY);

        const cleanup = () => dragImage.element.remove();
        setTimeout(cleanup, 0);
      } else {
        e.dataTransfer.setDragImage(EMPTY_DRAG_IMAGE, 0, 0);
      }
    }

    this.dispatch({ type: 'dragStart', pageIdx, x, y });
  }

  #renderDragImage(visiblePages: number[], pageIdx: number): { element: HTMLCanvasElement; offsetX: number; offsetY: number } | null {
    const dragImageInfo = this.#wasmEditor?.renderDragImage(new Uint32Array(visiblePages), pageIdx);
    if (!dragImageInfo) return null;

    const { ptr, len, width, height, offsetX, offsetY, scaleFactor } = dragImageInfo;

    const wasmMemory = getMemory() as WebAssembly.Memory;
    if (!wasmMemory) return null;

    const buffer = new Uint8ClampedArray(wasmMemory.buffer, ptr, len);
    const imageData = new ImageData(new Uint8ClampedArray(buffer), width, height);

    const tempCanvas = document.createElement('canvas');
    tempCanvas.width = width;
    tempCanvas.height = height;
    const tempCtx = tempCanvas.getContext('2d');
    if (!tempCtx) return null;
    tempCtx.putImageData(imageData, 0, 0);

    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const cssWidth = Math.ceil(width / scaleFactor);
    const cssHeight = Math.ceil(height / scaleFactor);
    canvas.style.width = `${cssWidth}px`;
    canvas.style.height = `${cssHeight}px`;

    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    ctx.save();
    ctx.globalAlpha = 0.7;

    ctx.drawImage(tempCanvas, 0, 0);

    const GAP = 24;

    for (const el of this.externalElements) {
      if (!el.isSelected || el.data.type !== 'image' || !visiblePages.includes(el.pageIdx)) {
        continue;
      }

      let relativePageY = 0;
      if (el.pageIdx !== pageIdx) {
        const start = Math.min(pageIdx, el.pageIdx);
        const end = Math.max(pageIdx, el.pageIdx);
        let dist = 0;
        for (let i = start; i < end; i++) {
          dist += (this.layout.pageHeights[i] ?? 0) + GAP;
        }
        relativePageY = el.pageIdx < pageIdx ? -dist : dist;
      }

      const imgElement = document.querySelector(`div[data-node-id="${el.nodeId}"] img`);
      if (imgElement instanceof HTMLImageElement) {
        const globalX = el.bounds.x;
        const globalY = el.bounds.y;

        const destX = (globalX - offsetX) * scaleFactor;
        const destY = (relativePageY + globalY - offsetY) * scaleFactor;
        const destW = el.bounds.width * scaleFactor;
        const destH = el.bounds.height * scaleFactor;

        ctx.drawImage(imgElement, destX, destY, destW, destH);
      }
    }

    ctx.restore();

    return { element: canvas, offsetX, offsetY };
  }

  handleDragOver(e: DragEvent): void {
    e.preventDefault();
    if (!(e.target instanceof HTMLElement)) return;

    let pageElement: HTMLElement | null = e.target;
    while (pageElement && pageElement.dataset && !pageElement.dataset.pageIndex) {
      pageElement = pageElement.parentElement;
    }

    if (!pageElement || !pageElement.dataset.pageIndex) return;

    const pageIdx = Number.parseInt(pageElement.dataset.pageIndex);
    const rect = pageElement.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    this.dispatch({ type: 'dragOver', pageIdx, x, y });
  }

  handleDragLeave(e: DragEvent): void {
    if (e.relatedTarget instanceof Node && (e.currentTarget as Node).contains(e.relatedTarget)) {
      return;
    }
    e.preventDefault();
    this.dispatch({ type: 'dragLeave' });
  }

  handleDragEnter(e: DragEvent): void {
    e.preventDefault();
    this.dispatch({ type: 'dragEnter' });
  }

  handleDrop(e: DragEvent): void {
    e.preventDefault();
    if (!(e.target instanceof HTMLElement)) return;

    let pageElement: HTMLElement | null = e.target;
    while (pageElement && pageElement.dataset && !pageElement.dataset.pageIndex) {
      pageElement = pageElement.parentElement;
    }

    if (!pageElement || !pageElement.dataset.pageIndex) return;

    const pageIdx = Number.parseInt(pageElement.dataset.pageIndex);
    const rect = pageElement.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    let fragment: string | undefined;
    let html: string | undefined;
    let text: string | undefined;

    if (e.dataTransfer) {
      if (e.dataTransfer.types.includes(FRAGMENT_MIME)) {
        fragment = e.dataTransfer.getData(FRAGMENT_MIME);
      }
      if (e.dataTransfer.types.includes('text/html')) {
        html = e.dataTransfer.getData('text/html');
      }
      if (e.dataTransfer.types.includes('text/plain')) {
        text = e.dataTransfer.getData('text/plain');
      }
    }

    this.dispatch({
      type: 'drop',
      pageIdx,
      x,
      y,
      text,
      html,
      fragment,
    } as unknown as Message);
  }

  handleDragEnd(e: DragEvent): void {
    void e;
    this.dispatch({ type: 'dragEnd' });
  }

  can(messageType: string): boolean {
    return this.enabledActions.has(messageType);
  }

  canDragAt(pageIdx: number, x: number, y: number): boolean {
    return this.#wasmEditor?.canDragAt(pageIdx, x, y) ?? false;
  }

  getClipboardData(): { fragment: string; html: string; text: string } | null {
    return this.#wasmEditor?.getClipboardData() ?? null;
  }

  focus(): Editor {
    this.inputElement?.focus({ preventScroll: true });

    return this;
  }

  destroy(): void {
    this.#stop();
    this.#wasmEditor = null;
    this.#application = null;
  }
}
