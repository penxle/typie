import { Application, getMemory, SyncVersion } from '@typie/editor';
import icuPostcardUrl from '@typie/editor/pkg/icu_data.postcard?url';
import { nanoid } from 'nanoid';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { FRAGMENT_MIME, PAGE_GAP } from './constants';
import { ensurePhantomFonts, ensureRequiredFonts, ensureRequiredWritingSystems, getAvailableFontsMap } from './fonts';
import { calculateImageDisplaySize, calculateRelativePosition, findNearestPageCoordinate, getPageElement, idleCallback } from './utils';
import type { Editor as WasmEditor, ExportedUpdates, Modifier, PointerButton, TableOverlay } from '@typie/editor';
import type { ThemeColors } from './theme';
import type {
  Cmd,
  EmbedAsset,
  ExternalElement,
  FileAsset,
  ImageAsset,
  LayoutMode,
  Mark,
  MarkType,
  Message,
  Rect,
  SearchOverlay,
  SelectionStats,
  SpellcheckErrorData,
  SpellcheckOverlay,
  WritingSystem,
} from './types';

let sharedApplication: Application | null = null;
let applicationInitPromise: Promise<Application> | null = null;

async function getOrInitializeApplication(): Promise<Application> {
  if (sharedApplication) {
    return sharedApplication;
  }

  if (applicationInitPromise) {
    return applicationInitPromise;
  }

  applicationInitPromise = (async () => {
    const app = new Application();

    const icuPostcard = await fetch(icuPostcardUrl).then((res) => res.arrayBuffer());
    app.loadIcuData(new Uint8Array(icuPostcard));
    await ensurePhantomFonts(app);
    app.setAvailableFonts(getAvailableFontsMap());

    sharedApplication = app;
    return app;
  })();

  return applicationInitPromise;
}

const CLICK_INTERVAL = 500;
const CLICK_DISTANCE = 5;

let EMPTY_DRAG_IMAGE: HTMLImageElement | null = null;
const getEmptyDragImage = () => {
  if (!EMPTY_DRAG_IMAGE && typeof Image !== 'undefined') {
    EMPTY_DRAG_IMAGE = new Image();
    EMPTY_DRAG_IMAGE.src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
  }
  return EMPTY_DRAG_IMAGE;
};

export type EditorOptions = {
  theme: ThemeColors;
  snapshot?: Uint8Array;
  readOnly?: boolean;
  onDocChanged?: () => void;
  onExitedDocumentStart?: () => void;
};

export class Editor {
  static get SyncVersion() {
    return SyncVersion;
  }
  #application: Application | null = null;
  #wasmEditor: WasmEditor | null = null;
  #running = false;
  #rafId: number | null = null;
  #pendingFontLoad = false;
  #onDocChanged?: () => void;
  #onExitedDocumentStart?: () => void;
  #readyResolve?: () => void;
  ready: Promise<void>;

  constructor() {
    this.ready = new Promise((resolve) => {
      this.#readyResolve = resolve;
    });
  }

  renderVersion = $state(0);
  #fontLoadingCount = $state(0);
  isLoadingFonts = $derived(this.#fontLoadingCount > 0);

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
    collapsed: true,
  });

  characterCounts = $state({
    docWithWhitespace: 0,
    docWithoutWhitespace: 0,
    docWithoutWhitespaceAndPunctuation: 0,
    selectionWithWhitespace: 0,
    selectionWithoutWhitespace: 0,
    selectionWithoutWhitespaceAndPunctuation: 0,
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

  imageAssets = $state(new SvelteMap<string, ImageAsset>());
  fileAssets = $state(new SvelteMap<string, FileAsset>());
  embedAssets = $state(new SvelteMap<string, EmbedAsset>());
  inflightImages = $state(new SvelteMap<string, { url: string; width: number; height: number }>());
  inflightFiles = $state(new SvelteMap<string, { url: string; name: string; size: number }>());

  enabledActions = $state(new SvelteSet<string>());

  cursor = $state({
    pageIdx: -1,
    bounds: null as Rect | null,
    show: false,
    scrollToCursor: false,
    animate: false,
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
  isPointerModeIdle = $state(false);
  readOnly = $state(false);

  placeholder = $state({
    visible: false,
    bounds: null as Rect | null,
  });

  linkOverlays = $state<{ pageIdx: number; href: string; bounds: Rect[] }[]>([]);

  spellcheckOverlays = $state<SpellcheckOverlay[]>([]);
  activeSpellcheckErrorId = $state<string | null>(null);
  fullSpellcheckErrors = $state<SpellcheckErrorData[]>([]);

  searchResults = $state({
    overlays: [] as SearchOverlay[],
    totalCount: 0,
    currentIndex: 0,
  });

  tableOverlays = $state<TableOverlay[]>([]);

  typewriter = $state({
    needsScroll: false,
  });

  pageVisibility = new SvelteMap<number, number>();

  extensionArea = $state({
    containerEl: null as HTMLElement | null,
    pageElements: [] as HTMLElement[],
  });

  scrollContainerEl = $state<HTMLElement | null>(null);

  pageContainerEls = $state<HTMLDivElement[]>([]);

  #lastClickTime = 0;
  #lastClickPos: { x: number; y: number } | null = null;
  #clickCount = 0;

  uploadQueue = new SvelteMap<string, File>();

  queueUpload(uploadId: string, file: File): void {
    this.uploadQueue.set(uploadId, file);
    setTimeout(() => {
      if (this.uploadQueue.has(uploadId)) {
        this.uploadQueue.delete(uploadId);
        console.warn('Upload timed out for', uploadId);
      }
    }, 30_000);
  }

  popUpload(uploadId: string): File | undefined {
    const file = this.uploadQueue.get(uploadId);
    if (file) {
      this.uploadQueue.delete(uploadId);
    }
    return file;
  }

  async initialize(options: EditorOptions): Promise<void> {
    if (this.#wasmEditor) {
      return;
    }

    this.#onDocChanged = options.onDocChanged;
    this.#onExitedDocumentStart = options.onExitedDocumentStart;

    const app = await getOrInitializeApplication();
    this.#application = app;

    const scaleFactor = window.devicePixelRatio * (window.visualViewport?.scale || 1);
    const wasmEditor = app.createEditor(scaleFactor, options.snapshot);
    this.#wasmEditor = wasmEditor;

    this.dispatch({
      type: 'initialize',
      theme: options.theme,
    });

    this.dispatch({ type: 'navigate', direction: 'documentStart', extend: false });

    if (options.readOnly) {
      this.setReadOnly(true);
    }

    this.#start();
    this.#readyResolve?.();
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
          this.typewriter.needsScroll = true;
          this.characterCountsVersion++;
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
            this.cursor.scrollToCursor = cmd.scrollToCursor;
          } else {
            this.cursor.pageIdx = -1;
            this.cursor.bounds = null;
            this.cursor.show = false;
            this.cursor.scrollToCursor = false;
          }
          break;
        }

        case 'selectionChanged': {
          this.selection.stats = cmd.stats;
          this.selection.collapsed = cmd.collapsed;
          this.characterCountsVersion++;
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
          if (this.pointer.currentHoverTarget && !this.pointer.currentHoverTarget.closest('[data-external-element]')) {
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

        case 'exitedDocumentStart': {
          this.#onExitedDocumentStart?.();
          break;
        }

        case 'pointerModeChanged': {
          this.isPointerModeIdle = cmd.is_idle;
          break;
        }

        case 'placeholderChanged': {
          this.placeholder.visible = cmd.visible;
          this.placeholder.bounds = cmd.bounds ?? null;
          break;
        }

        case 'linkOverlaysChanged': {
          this.linkOverlays = cmd.overlays;
          break;
        }

        case 'spellcheckOverlaysChanged': {
          this.spellcheckOverlays = cmd.overlays;
          this.activeSpellcheckErrorId = cmd.overlays.find((o) => o.isActive)?.id ?? null;

          const validIds = new SvelteSet(cmd.overlays.map((o) => o.id));
          if (this.fullSpellcheckErrors.length > 0) {
            this.fullSpellcheckErrors = this.fullSpellcheckErrors.filter((e) => validIds.has(e.id));
          }
          break;
        }

        case 'searchResultsChanged': {
          this.searchResults.overlays = cmd.overlays;
          this.searchResults.totalCount = cmd.totalCount;
          this.searchResults.currentIndex = cmd.currentIndex;

          const currentOverlay = cmd.overlays.find((o) => o.isCurrent);
          if (currentOverlay && currentOverlay.bounds.length > 0) {
            const pageEl = this.pageContainerEls[currentOverlay.pageIdx];
            const scroller = this.scrollContainerEl;
            if (pageEl && scroller) {
              const pageRect = pageEl.getBoundingClientRect();
              const scrollerRect = scroller.getBoundingClientRect();
              const bound = currentOverlay.bounds[0];
              const targetY = pageRect.top + bound.y - scrollerRect.top + scroller.scrollTop;
              const viewportCenter = scroller.clientHeight / 2;
              const targetScroll = targetY - viewportCenter + bound.height / 2;
              scroller.scrollTo({ top: Math.max(0, targetScroll), behavior: 'smooth' });
            }
          }
          break;
        }

        case 'tableOverlaysChanged': {
          this.tableOverlays = cmd.overlays;
          break;
        }
      }
    }
  }

  #handleFontsRequired(fonts: [string, number][]): void {
    if (fonts.length === 0 || !this.#application || this.#pendingFontLoad) return;

    this.#pendingFontLoad = true;
    ensureRequiredFonts(this.#application, fonts, {
      onStart: () => this.#fontLoadingCount++,
      onEnd: () => this.#fontLoadingCount--,
    }).then((loaded) => {
      this.#pendingFontLoad = false;
      if (loaded) {
        this.dispatch({ type: 'fontsLoaded' });
      }
    });
  }

  #handleWritingSystemsRequired(systems: WritingSystem[]): void {
    if (systems.length === 0 || !this.#application) return;

    ensureRequiredWritingSystems(this.#application, systems, {
      onStart: () => this.#fontLoadingCount++,
      onEnd: () => this.#fontLoadingCount--,
    }).then((loaded) => {
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

  getVersion(): Uint8Array | undefined {
    return this.#wasmEditor?.getVersion();
  }

  exportAllUpdates(): Uint8Array | undefined {
    return this.#wasmEditor?.exportAllUpdates();
  }

  exportUpdatesFrom(version: Uint8Array): Uint8Array | undefined {
    return this.#wasmEditor?.exportUpdatesFrom(version);
  }

  exportNewUpdates(): ExportedUpdates | undefined {
    return this.#wasmEditor?.exportNewUpdates();
  }

  commitSync(version: SyncVersion): void {
    this.#wasmEditor?.commitSync(version);
  }

  importUpdates(updates: Uint8Array): void {
    this.#wasmEditor?.importUpdates(updates);
  }

  insertTemplateFragment(snapshot: Uint8Array): void {
    this.#wasmEditor?.insertTemplateFragment(snapshot);
  }

  importUpdatesBatch(updatesBatch: Uint8Array[]): void {
    this.#wasmEditor?.importUpdatesBatch(updatesBatch);
  }

  checkout(version: Uint8Array): void {
    this.#wasmEditor?.checkout(version);
  }

  checkoutToLatest(): void {
    this.#wasmEditor?.checkoutToLatest();
  }

  isDetached(): boolean {
    return this.#wasmEditor?.isDetached() ?? false;
  }

  getCharacterCountAtVersion(version: Uint8Array): number | undefined {
    return this.#wasmEditor?.getCharacterCountAtVersion(version);
  }

  revertTo(version: Uint8Array): void {
    this.#wasmEditor?.revertTo(version);
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
    const fromTarget = this.#resolvePageCoordinateFromElement(e, targetEl);
    if (fromTarget) {
      return fromTarget;
    }

    if (this.layout.layoutMode.type === 'paginated') {
      const el = document.elementFromPoint(e.clientX, e.clientY);
      if (el instanceof HTMLElement) {
        return this.#resolvePageCoordinateFromElement(e, el);
      }
    }

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

    return null;
  }

  #resolvePageCoordinateFromElement(
    e: MouseEvent | PointerEvent,
    element: HTMLElement,
  ): { pageIdx: number; x: number; y: number; pageElement: HTMLElement; isExtensionArea: boolean } | null {
    const pageElement = getPageElement(element);
    if (!pageElement) return null;

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const pageIdx = Number.parseInt(pageElement.dataset.pageIndex!);
    const point = calculateRelativePosition(pageElement, e);
    return {
      pageIdx,
      x: point.x,
      y: point.y,
      pageElement,
      isExtensionArea: false,
    };
  }

  handlePointerDown(e: PointerEvent): void {
    if (!(e.target instanceof HTMLElement)) return;

    if (e.target.closest('[data-pointer-capture]')) return;

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) {
      this.isDraggable = false;
      return;
    }

    const { pageIdx, x, y, pageElement } = resolved;

    const rect = pageElement.getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const relY = e.clientY - rect.top;
    this.isDraggable = !this.readOnly && this.canDragAt(pageIdx, relX, relY);

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
      button: this.#toPointerButton(e.button),
      modifier: this.#toModifier(e),
    });
  }

  handlePointerMove(e: PointerEvent): void {
    const targetEl = document.elementFromPoint(e.clientX, e.clientY);
    if (!(targetEl instanceof HTMLElement)) return;

    if (targetEl.closest('[data-pointer-capture]')) return;

    const resolved = this.#resolvePointerCoordinate(e, targetEl);
    if (!resolved) return;

    const { pageIdx, x, y, isExtensionArea } = resolved;

    this.pointer.currentHoverTarget = isExtensionArea ? (this.extensionArea.containerEl ?? targetEl) : targetEl;

    this.dispatch({
      type: 'pointerMove',
      pageIdx,
      x,
      y,
      buttons: e.buttons,
      modifier: this.#toModifier(e),
    });
  }

  handlePointerMoveFromCoordinate(clientX: number, clientY: number): void {
    const pointEl = document.elementFromPoint(clientX, clientY);
    const targetEl = pointEl instanceof HTMLElement ? pointEl : this.extensionArea.containerEl;
    if (!targetEl) return;

    const syntheticEvent = { clientX, clientY, target: targetEl } as unknown as MouseEvent;
    const resolved = this.#resolvePointerCoordinate(syntheticEvent, targetEl);
    if (!resolved) return;

    const { pageIdx, x, y, isExtensionArea } = resolved;

    this.pointer.currentHoverTarget = isExtensionArea ? (this.extensionArea.containerEl ?? targetEl) : targetEl;

    this.dispatch({
      type: 'pointerMove',
      pageIdx,
      x,
      y,
      buttons: 1,
      modifier: { shift: false, ctrl: false, alt: false, meta: false },
    });
  }

  handlePointerUp(e: PointerEvent): void {
    this.isDraggable = false;

    if (!(e.target instanceof HTMLElement)) return;

    e.target.releasePointerCapture(e.pointerId);

    const targetEl = document.elementFromPoint(e.clientX, e.clientY);
    if (!(targetEl instanceof HTMLElement)) return;

    if (targetEl.closest('[data-pointer-capture]')) {
      this.pointer.isPressed = false;
      return;
    }

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
      button: this.#toPointerButton(e.button),
      modifier: this.#toModifier(e),
    });

    this.pointer.isPressed = false;
  }

  handleContextMenu(e: MouseEvent): void {
    if (!(e.target instanceof HTMLElement)) return;

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) return;

    const { pageIdx, x, y } = resolved;

    e.preventDefault();

    this.dispatch({
      type: 'pointerDown',
      pageIdx,
      x,
      y,
      clickCount: 1,
      button: 'secondary',
      modifier: this.#toModifier(e),
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
      try {
        const items = new ClipboardItem({
          'text/html': new Blob([html], { type: 'text/html' }),
          'text/plain': new Blob([text], { type: 'text/plain' }),
        });
        await navigator.clipboard.write([items]);
      } catch {
        await navigator.clipboard.writeText(text);
      }
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
    if (!(e.target instanceof HTMLElement)) return;

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) return;

    const { pageIdx, x, y, pageElement } = resolved;

    const rect = pageElement.getBoundingClientRect();

    if (this.readOnly || !this.canDragAt(pageIdx, x, y)) {
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
        const emptyImage = getEmptyDragImage();
        if (emptyImage) {
          e.dataTransfer.setDragImage(emptyImage, 0, 0);
        }
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
          dist += (this.layout.pageHeights[i] ?? 0) + PAGE_GAP;
        }
        relativePageY = el.pageIdx < pageIdx ? -dist : dist;
      }

      const imgElement = document.querySelector(`div[data-node-id="${el.nodeId}"] img`);
      if (imgElement instanceof HTMLImageElement && el.data.type === 'image') {
        const imageId = el.data.id;
        const uploadId = el.data.uploadId;
        const asset = imageId ? this.imageAssets.get(imageId) : undefined;
        const inflight = uploadId ? this.inflightImages.get(uploadId) : undefined;
        const originalWidth = asset?.width ?? inflight?.width ?? 0;
        const originalHeight = asset?.height ?? inflight?.height ?? 0;
        const { displayWidth, xOffset } = calculateImageDisplaySize(el.bounds, originalWidth, originalHeight);

        const globalX = el.bounds.x + xOffset;
        const globalY = el.bounds.y;

        const destX = (globalX - offsetX) * scaleFactor;
        const destY = (relativePageY + globalY - offsetY) * scaleFactor;
        const destW = displayWidth * scaleFactor;
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

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) return;

    const { pageIdx, x, y } = resolved;

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

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) return;

    const { pageIdx, x, y } = resolved;

    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const allFiles = [...e.dataTransfer.files];
      const imageFiles = allFiles.filter((file) => file.type.startsWith('image/'));
      const otherFiles = allFiles.filter((file) => !file.type.startsWith('image/'));

      if (imageFiles.length > 0) {
        const uploadIds: string[] = [];
        for (const file of imageFiles) {
          const uploadId = nanoid();
          this.queueUpload(uploadId, file);
          uploadIds.push(uploadId);
        }
        this.dispatch({
          type: 'dropImages',
          pageIdx,
          x,
          y,
          uploadIds,
        }).focus();
      }

      if (otherFiles.length > 0) {
        const uploadIds: string[] = [];
        for (const file of otherFiles) {
          const uploadId = nanoid();
          this.queueUpload(uploadId, file);
          uploadIds.push(uploadId);
        }
        this.dispatch({
          type: 'dropFiles',
          pageIdx,
          x,
          y,
          uploadIds,
        }).focus();
      }

      if (imageFiles.length > 0 || otherFiles.length > 0) {
        return;
      }
    }

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
      modifier: this.#toModifier(e),
    });
  }

  handleDragEnd(e: DragEvent): void {
    void e;
    this.dispatch({ type: 'dragEnd' });
  }

  can(messageType: string): boolean {
    return this.enabledActions.has(messageType);
  }

  setReadOnly(readOnly: boolean): void {
    this.readOnly = readOnly;
    this.#wasmEditor?.setReadOnly(readOnly);
  }

  isReadOnly(): boolean {
    return this.#wasmEditor?.isReadOnly() ?? this.readOnly;
  }

  setAutoSurroundEnabled(enabled: boolean): void {
    this.ready.then(() => {
      this.#wasmEditor?.setAutoSurroundEnabled(enabled);
    });
  }

  setSpellcheckErrors(errors: { id: string; nodeId: string; startOffset: number; endOffset: number }[]): void {
    this.ready.then(() => {
      this.#wasmEditor?.setSpellcheckErrors(errors);
    });
  }

  clearSpellcheckErrors(): void {
    this.ready.then(() => {
      this.#wasmEditor?.clearSpellcheckErrors();
    });
  }

  applySpellcheckCorrection(nodeId: string, startOffset: number, endOffset: number, correction: string): boolean {
    return this.#wasmEditor?.applySpellcheckCorrection(nodeId, startOffset, endOffset, correction) ?? false;
  }

  getSpellcheckErrors(): { id: string; nodeId: string; startOffset: number; endOffset: number }[] {
    return this.#wasmEditor?.getSpellcheckErrors() ?? [];
  }

  getSpellcheckText(): { text: string; mappings: { nodeId: string; textStart: number; textEnd: number; blockOffset: number }[] } | null {
    return this.#wasmEditor?.getSpellcheckText() ?? null;
  }

  selectSpellcheckError(errorId: string, options?: { focusEditor?: boolean; animate?: boolean }): void {
    const focusEditor = options?.focusEditor ?? true;
    const animate = options?.animate ?? false;

    if (animate) {
      this.cursor.animate = true;
    }

    this.dispatch({ type: 'selectSpellcheckError', errorId });
    if (focusEditor) {
      this.focus();
    }
  }

  canDragAt(pageIdx: number, x: number, y: number): boolean {
    return this.#wasmEditor?.canDragAt(pageIdx, x, y) ?? false;
  }

  getClipboardData(): { fragment: string; html: string; text: string } | null {
    return this.#wasmEditor?.getClipboardData() ?? null;
  }

  #characterCountsDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  characterCountsVersion = $state(0);

  updateCharacterCounts(): void {
    if (this.#characterCountsDebounceTimer) {
      clearTimeout(this.#characterCountsDebounceTimer);
    }

    this.#characterCountsDebounceTimer = setTimeout(() => {
      const counts = this.#wasmEditor?.getCharacterCounts();
      if (counts) {
        this.characterCounts = {
          docWithWhitespace: counts.doc_with_whitespace,
          docWithoutWhitespace: counts.doc_without_whitespace,
          docWithoutWhitespaceAndPunctuation: counts.doc_without_whitespace_and_punctuation,
          selectionWithWhitespace: counts.selection_with_whitespace,
          selectionWithoutWhitespace: counts.selection_without_whitespace,
          selectionWithoutWhitespaceAndPunctuation: counts.selection_without_whitespace_and_punctuation,
        };
      }
      this.#characterCountsDebounceTimer = null;
    }, 150);
  }

  search(query: string, options?: { matchWholeWord?: boolean }): void {
    this.dispatch({
      type: 'search',
      query,
      matchWholeWord: options?.matchWholeWord ?? false,
    });
  }

  clearSearch(): void {
    this.dispatch({ type: 'clearSearch' });
  }

  findNext(): void {
    this.dispatch({ type: 'findNext' });
  }

  findPrevious(): void {
    this.dispatch({ type: 'findPrevious' });
  }

  replace(replacement: string): void {
    this.dispatch({ type: 'replace', replacement });
  }

  replaceAll(replacement: string): void {
    this.dispatch({ type: 'replaceAll', replacement });
  }

  focus(): Editor {
    this.inputElement?.focus({ preventScroll: true });

    return this;
  }

  destroy(): void {
    this.#stop();
    this.#wasmEditor = null;
  }

  #toPointerButton(button: number): PointerButton {
    switch (button) {
      case 0: {
        return 'primary';
      }
      case 1: {
        return 'auxiliary';
      }
      case 2: {
        return 'secondary';
      }
      default: {
        return 'primary';
      }
    }
  }

  #toModifier(e: MouseEvent | PointerEvent): Modifier {
    return {
      shift: e.shiftKey,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      meta: e.metaKey,
    };
  }
}
