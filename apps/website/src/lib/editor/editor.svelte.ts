import icuPostcardUrl from '@typie/editor/icu/data.postcard?url';
import { Tip } from '@typie/ui/notification';
import { nanoid } from 'nanoid';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { defaultValues } from '@/const';
import { wasm } from '$lib/wasm';
import { PAGE_GAP } from './constants';
import { ensureRequiredFallbackFont, ensureRequiredFont, filterUncoveredCodepoints, initFonts, preloadRemainingChunks } from './fonts';
import {
  DIRTY_ATTRS,
  DIRTY_CURSOR,
  DIRTY_DEFAULT_ATTRS,
  DIRTY_DOC_CHANGED,
  DIRTY_ENABLED_ACTIONS,
  DIRTY_EXITED_DOCUMENT_START,
  DIRTY_EXTERNAL_ELEMENTS,
  DIRTY_FONT_REQUIRED,
  DIRTY_LINK_OVERLAYS,
  DIRTY_PAGES,
  DIRTY_PLACEHOLDER,
  DIRTY_POINTER,
  DIRTY_REMARKS,
  DIRTY_RENDER_REQUIRED,
  DIRTY_REPASTE,
  DIRTY_SELECTION,
  DIRTY_SETTINGS,
  DIRTY_TABLE_OVERLAYS,
  DIRTY_TRACKED_ITEMS,
  SELECTION_EXPAND_ALL,
  SlateReader,
} from './slate';
import { TouchGestureController } from './touch-gesture.svelte';
import { calculateImageDisplaySize, calculateRelativePosition, findNearestPageCoordinate, getPageElement, idleCallback } from './utils';
import { WebGLRenderer } from './webgl';
import type { Placement } from '@floating-ui/dom';
import type { DocExportMode, Editor as WasmEditor, Modifier, PointerButton } from '@typie/editor';
import type { ScrollViewport } from '@typie/ui/utils';
import type { FontFamily } from './fonts';
import type { RemarkOverlay, TableOverlay, TrackedItem } from './slate';
import type { ThemeColors } from './theme';
import type {
  AiFeedback,
  ArchivedAsset,
  Attribute,
  EmbedAsset,
  ExternalElement,
  FileAsset,
  ImageAsset,
  LayoutMode,
  Message,
  Position,
  Rect,
  Selection,
  SpellcheckError,
} from './types';

let initPromise: Promise<void> | null = null;

function ensureInitialized(): Promise<void> {
  if (!initPromise) {
    initPromise = (async () => {
      const icuPostcard = await fetch(icuPostcardUrl).then((res) => res.arrayBuffer());
      wasm.loadIcuData(new Uint8Array(icuPostcard));
      await initFonts(wasm);
    })();
  }
  return initPromise;
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
  fontFamilies: FontFamily[];
  readOnly?: boolean;
  onDocChanged?: () => void;
  onExitedDocumentStart?: () => void;
  onSelectionChanged?: (anchor: Position, head: Position) => void;
};

export class Editor {
  #wasmEditor: WasmEditor | null = null;
  #slateReader: SlateReader | null = null;
  #running = false;
  #rafId: number | null = null;
  #flushPending = false;
  #awake = false;
  #renderDebugEnabled = false;
  #layoutDebugEnabled = false;
  #settledResolvers: (() => void)[] = [];
  #onDocChanged?: () => void;
  #onExitedDocumentStart?: () => void;
  #onSelectionChanged?: (anchor: Position, head: Position) => void;
  #pendingSelectAllShortcut = false;
  #readyResolve?: () => void;
  #renderer: WebGLRenderer | null = null;
  ready: Promise<void>;

  constructor() {
    this.ready = new Promise((resolve) => {
      this.#readyResolve = resolve;
    });
  }

  fontFamilies = $state<FontFamily[]>([]);

  renderVersion = $state(0);

  layout = $state({
    pages: [] as { width: number; height: number }[],
    layoutMode: {
      type: 'continuous',
      maxWidth: defaultValues.maxWidth,
    } as LayoutMode,
  });

  selection: Selection | null = $state(null);

  characterCounts = $state({
    docWithWhitespace: 0,
    docWithoutWhitespace: 0,
    docWithoutWhitespaceAndPunctuation: 0,
    selectionWithWhitespace: 0,
    selectionWithoutWhitespace: 0,
    selectionWithoutWhitespaceAndPunctuation: 0,
  });

  attrs = $state<Attribute[]>([]);

  getAttr(type: string): Attribute | undefined {
    return this.attrs.find((a) => a.type === type);
  }

  settings = $state({
    paragraphIndent: defaultValues.paragraphIndent as number,
    blockGap: defaultValues.blockGap as number,
  });

  defaultAttrs = $state<{
    fontFamily: string;
    fontSize: number;
    fontWeight: number;
    textColor: string;
    backgroundColor: string;
    letterSpacing: number;
    lineHeight: number;
  } | null>(null);

  externalElements = $state<ExternalElement[]>([]);

  imageAssets = $state(new SvelteMap<string, ImageAsset>());
  fileAssets = $state(new SvelteMap<string, FileAsset>());
  embedAssets = $state(new SvelteMap<string, EmbedAsset>());
  archivedAssets = $state(new SvelteMap<string, ArchivedAsset>());
  inflightImages = $state(new SvelteMap<string, { url: string; width: number; height: number }>());
  inflightFiles = $state(new SvelteMap<string, { url: string; name: string; size: number }>());

  enabledActions = $state(new SvelteSet<string>());

  cursor = $state({
    pageIdx: -1,
    bounds: null as Rect | null,
    visible: false,
  });

  pendingScrollConsumer = $state<'cursor' | 'typewriter' | null>(null);
  #pendingTypewriterRequest = false;
  #typewriterAvailable = false;

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
    source: 'mouse' as 'mouse' | 'touch',
    placement: 'bottom-start' as Placement,
  });

  isFocused = $state(false);
  pointerState = $state(0);
  readOnly = $state(false);
  protectContent = $state(false);

  placeholder = $state({
    visible: false,
    bounds: null as Rect | null,
  });

  linkOverlays = $state<{ pageIdx: number; href: string; bounds: Rect[] }[]>([]);

  trackedItems = $state<TrackedItem[]>([]);

  spellcheckErrors = $state<SpellcheckError[]>([]);
  aiFeedbacks = $state<AiFeedback[]>([]);
  searchMatches = $state<{ id: string; active: boolean }[]>([]);

  tableOverlays = $state<TableOverlay[]>([]);

  remarkOverlays = $state<RemarkOverlay[]>([]);
  remarkFocus = $state<{ nodeId: string; remarkId: string } | null>(null);
  currentBlock = $state<{ nodeId: string; pageIdx: number; bounds: Rect } | null>(null);

  repasteAsTextEnabled = $state(false);

  pageVisibility = new SvelteMap<number, number>();

  extensionArea = $state({
    containerEl: null as HTMLElement | null,
    pageElements: [] as HTMLElement[],
  });

  scrollContainerEl = $state<HTMLElement | null>(null);

  scrollViewport = $state<ScrollViewport | null>(null);

  pageContainerEls = $state<HTMLDivElement[]>([]);
  touchGesture = new TouchGestureController(this);

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

  insertImagesFromFiles(files: Iterable<File>): boolean {
    let handled = false;

    for (const file of files) {
      if (!file.type.startsWith('image/')) continue;

      const uploadId = nanoid();
      this.queueUpload(uploadId, file);
      this.dispatch({ type: 'insertImage', uploadId }).scrollIntoView({ mode: 'typewriter' });
      handled = true;
    }

    return handled;
  }

  async initialize(options: EditorOptions): Promise<void> {
    if (this.#wasmEditor) {
      return;
    }

    if (options.fontFamilies?.length) {
      this.fontFamilies = options.fontFamilies;
    }

    this.#onDocChanged = options.onDocChanged;
    this.#onExitedDocumentStart = options.onExitedDocumentStart;
    this.#onSelectionChanged = options.onSelectionChanged;

    await ensureInitialized();

    const scaleFactor = window.devicePixelRatio * (window.visualViewport?.scale || 1);
    const wasmEditor = wasm.createEditor(scaleFactor, options.snapshot);
    this.#wasmEditor = wasmEditor;
    wasmEditor.setRenderDebug(this.#renderDebugEnabled);
    wasmEditor.setLayoutDebug(this.#layoutDebugEnabled);

    const memory = wasm.getMemory() as WebAssembly.Memory;
    const rawOffsets = wasmEditor.getSlateOffsets();
    const offsets: Record<string, number> = {};
    for (const [key, value] of rawOffsets) {
      offsets[key] = value;
    }
    this.#slateReader = new SlateReader(memory, offsets, wasmEditor.getSlatePtr(), wasmEditor.getSlabPtr());

    this.dispatch({
      type: 'initialize',
      theme: options.theme,
    });

    if (options.readOnly) {
      this.setReadOnly(true);
    }

    this.#start();
    this.#readyResolve?.();
  }

  #start(): void {
    if (this.#running) return;
    this.#running = true;
    this.#ensureActive();
  }

  #stop(): void {
    this.#running = false;
    if (this.#rafId !== null) {
      cancelAnimationFrame(this.#rafId);
      this.#rafId = null;
    }
  }

  #wakeUp(): void {
    if (!this.#awake) {
      this.#awake = true;
      this.#ensureActive();
    }
  }

  #ensureActive(): void {
    if (this.#running && this.#rafId === null) {
      this.#rafId = requestAnimationFrame(this.#tick);
    }
  }

  #tick = (): void => {
    this.#rafId = null;
    if (!this.#running) return;
    const hadPendingSelectAllShortcut = this.#pendingSelectAllShortcut;

    if (this.#wasmEditor && this.#slateReader && this.#awake) {
      this.#awake = false;
      this.#wasmEditor.tick();
      this.#slateReader.refresh(this.#wasmEditor.getSlatePtr(), this.#wasmEditor.getSlabPtr());
      if (this.#slateReader.hasDirty) {
        this.#readSlate(this.#slateReader);
      }
      this.#pendingTypewriterRequest = false;

      if (!this.#flushPending) {
        this.#flushPending = true;
        idleCallback(() => {
          this.#flushPending = false;
          this.#wasmEditor?.flush();
        });
      }
    }

    if (hadPendingSelectAllShortcut) {
      this.#pendingSelectAllShortcut = false;
    }

    if (this.#settledResolvers.length > 0) {
      const resolvers = this.#settledResolvers;
      this.#settledResolvers = [];
      for (const resolve of resolvers) {
        resolve();
      }
    }

    if (this.#awake) {
      this.#rafId = requestAnimationFrame(this.#tick);
    }
  };

  #readSlate(slate: SlateReader): void {
    if (slate.isDirty(DIRTY_DOC_CHANGED)) {
      this.#onDocChanged?.();
      this.characterCountsVersion++;
    }

    if (slate.isDirty(DIRTY_RENDER_REQUIRED)) {
      this.renderVersion++;
    }

    if (slate.isDirty(DIRTY_SETTINGS)) {
      const s = slate.readSettings();
      this.settings.paragraphIndent = s.paragraphIndent;
      this.settings.blockGap = s.blockGap;
      this.layout.layoutMode = s.layoutMode;
    }

    if (slate.isDirty(DIRTY_DEFAULT_ATTRS)) {
      const attrs = slate.readDefaultAttrs();
      const ds: Record<string, string | number> = {};
      for (const attr of attrs) {
        const v = attr.values[0];
        if (v === null) continue;
        switch (attr.type) {
          case 'font_family': {
            ds.fontFamily = v as string;
            break;
          }
          case 'font_size': {
            ds.fontSize = v as number;
            break;
          }
          case 'font_weight': {
            ds.fontWeight = v as number;
            break;
          }
          case 'text_color': {
            ds.textColor = v as string;
            break;
          }
          case 'background_color': {
            ds.backgroundColor = v as string;
            break;
          }
          case 'letter_spacing': {
            ds.letterSpacing = v as number;
            break;
          }
          case 'line_height': {
            ds.lineHeight = v as number;
            break;
          }
        }
      }
      this.defaultAttrs = ds as NonNullable<typeof this.defaultAttrs>;
    }

    if (slate.isDirty(DIRTY_PAGES)) {
      this.layout.pages = slate.readPages();
    }

    if (slate.isDirty(DIRTY_CURSOR)) {
      const c = slate.readCursor();
      if (c.pageIdx >= 0 && c.bounds) {
        this.cursor.pageIdx = c.pageIdx;
        this.cursor.bounds = c.bounds;
        this.cursor.visible = c.visible;
        if (this.#pendingTypewriterRequest) {
          this.pendingScrollConsumer = this.#typewriterAvailable ? 'typewriter' : 'cursor';
        }
      } else {
        this.cursor.pageIdx = -1;
        this.cursor.bounds = null;
        this.cursor.visible = false;
        this.pendingScrollConsumer = null;
      }
    }

    if (slate.isDirty(DIRTY_SELECTION)) {
      const sel = slate.readSelection();
      const selectionExpandable = slate.readSelectionExpandable();
      this.selection = sel;
      this.characterCountsVersion++;
      this.#onSelectionChanged?.(sel.anchor, sel.head);
      this.#updateActiveTrackedItems();
      this.currentBlock = slate.readCurrentBlock();

      if (this.#pendingSelectAllShortcut && (selectionExpandable & SELECTION_EXPAND_ALL) !== 0) {
        Tip.show('editor.shortcut.select-all-document', '`Mod-A`를 한 번 더 누르면 문서 전체가 선택돼요.');
      }
    }

    if (slate.isDirty(DIRTY_ATTRS)) {
      this.attrs = slate.readAttrs();
    }

    if (slate.isDirty(DIRTY_EXTERNAL_ELEMENTS)) {
      this.externalElements = slate.readExternalElements();
    }

    if (slate.isDirty(DIRTY_POINTER)) {
      const style = slate.readPointerStyle();
      if (this.pointer.currentHoverTarget && !this.pointer.currentHoverTarget.closest('[data-external-element]')) {
        this.pointer.currentHoverTarget.style.cursor = style;
      }
      this.pointerState = slate.readPointerState();
    }

    if (slate.isDirty(DIRTY_FONT_REQUIRED)) {
      for (const req of slate.readFontRequests()) {
        this.#handleFontRequired(req.family, req.weight, req.codepoints);
      }
    }

    if (slate.isDirty(DIRTY_ENABLED_ACTIONS)) {
      this.enabledActions = new SvelteSet(slate.readEnabledActions());
    }

    if (slate.isDirty(DIRTY_EXITED_DOCUMENT_START)) {
      this.#onExitedDocumentStart?.();
    }

    if (slate.isDirty(DIRTY_PLACEHOLDER)) {
      const p = slate.readPlaceholder();
      this.placeholder.visible = p.visible;
      this.placeholder.bounds = p.bounds;
    }

    if (slate.isDirty(DIRTY_LINK_OVERLAYS)) {
      this.linkOverlays = slate.readLinkOverlays();
    }

    if (slate.isDirty(DIRTY_TRACKED_ITEMS)) {
      this.trackedItems = slate.readTrackedItems();

      const spellcheckIds = new SvelteSet(this.trackedItems.filter((v) => v.group === 0).map((v) => v.id));
      this.spellcheckErrors = this.spellcheckErrors.filter((v) => spellcheckIds.has(v.id));

      const aiFeedbackIds = new SvelteSet(this.trackedItems.filter((v) => v.group === 1).map((v) => v.id));
      this.aiFeedbacks = this.aiFeedbacks.filter((v) => aiFeedbackIds.has(v.id));

      const searchMatchIds = new SvelteSet(this.trackedItems.filter((v) => v.group === 2).map((v) => v.id));
      this.searchMatches = this.searchMatches.filter((v) => searchMatchIds.has(v.id));
    }

    if (slate.isDirty(DIRTY_TABLE_OVERLAYS)) {
      this.tableOverlays = slate.readTableOverlays();
    }

    if (slate.isDirty(DIRTY_REPASTE)) {
      const repaste = slate.readRepaste();
      this.repasteAsTextEnabled = repaste.enabled;
    }

    if (slate.isDirty(DIRTY_REMARKS)) {
      this.remarkOverlays = slate.readRemarks();
    }
  }

  #handleFontRequired(family: string, weight: number, codepoints: number[]): void {
    const font = this.fontFamilies.find((f) => f.familyName === family)?.fonts.find((f) => f.weight === weight);
    if (!font) return;

    Promise.all([
      ensureRequiredFont(wasm, family, font, codepoints).then(() => {
        if (!this.readOnly) {
          preloadRemainingChunks(wasm, family, font);
        }
      }),
      filterUncoveredCodepoints(font, codepoints).then((uncovered) =>
        uncovered.length > 0 ? ensureRequiredFallbackFont(wasm, weight, uncovered) : undefined,
      ),
    ]).then(() => {
      this.dispatch({ type: 'fontsLoaded', family, weight });
    });
  }

  #updateActiveTrackedItems(): void {
    if (!this.selection?.collapsed) {
      return;
    }

    const anchor = this.selection.anchor;

    for (const item of this.trackedItems) {
      const target =
        item.group === 0 ? this.spellcheckErrors : item.group === 1 ? this.aiFeedbacks : item.group === 2 ? this.searchMatches : null;

      if (!target) {
        continue;
      }

      if (item.nodeId === anchor.nodeId && anchor.offset >= item.startOffset && anchor.offset <= item.endOffset) {
        const t = target.find((v) => v.id === item.id);
        if (t) {
          t.active = true;
        }
      } else {
        const t = target.find((v) => v.id === item.id);
        if (t) {
          t.active = false;
        }
      }
    }
  }

  scrollTrackedItemIntoView(id: string): void {
    const item = this.trackedItems.find((v) => v.id === id);
    if (item && item.bounds.length > 0) {
      const pageEl = this.pageContainerEls[item.pageIdx];
      const scroller = this.scrollContainerEl;
      if (pageEl && scroller && item.bounds.length > 0) {
        const pageRect = pageEl.getBoundingClientRect();
        const scrollerRect = scroller.getBoundingClientRect();
        const bound = item.bounds[0];
        const targetY = pageRect.top + bound.y - scrollerRect.top + scroller.scrollTop;
        const viewportCenter = scroller.clientHeight / 2;
        const targetScroll = targetY - viewportCenter + bound.height / 2;
        scroller.scrollTo({ top: Math.max(0, targetScroll), behavior: 'smooth' });
      }
    }
  }

  handleRepasteAsText(): void {
    if (!this.repasteAsTextEnabled) return;

    this.dispatch({ type: 'repasteAsText' }).scrollIntoView({ mode: 'typewriter' });
    this.focus();
  }

  settled(): Promise<void> {
    return new Promise((resolve) => {
      this.#settledResolvers.push(resolve);
    });
  }

  dispatch(message: Message): Editor {
    this.#wasmEditor?.enqueueMessage(message);
    this.#wakeUp();

    return this;
  }

  updatePageVisibility(pageIndex: number, ratio: number): void {
    if (ratio > 0) {
      this.pageVisibility.set(pageIndex, ratio);
    } else {
      this.pageVisibility.delete(pageIndex);
    }
  }

  get renderer(): WebGLRenderer | null {
    if (!this.#renderer) {
      try {
        this.#renderer = new WebGLRenderer(() => this.renderVersion++);
      } catch (err) {
        console.error('WebGL init failed:', err);
      }
    }
    return this.#renderer;
  }

  renderPage(pageIdx: number) {
    return this.#wasmEditor?.renderPage(pageIdx);
  }

  renderPageToCanvas(pageIdx: number, target: CanvasRenderingContext2D): boolean {
    const renderer = this.renderer;
    if (!renderer) return false;
    const info = this.renderPage(pageIdx);
    if (!info) return false;
    const offscreen = renderer.render(info.ptr, info.len, info.width, info.height);
    if (!offscreen) return false;
    target.canvas.width = info.width;
    target.canvas.height = info.height;
    target.drawImage(offscreen, 0, 0);
    return true;
  }

  export(mode: DocExportMode): Uint8Array | undefined {
    return this.#wasmEditor?.export(mode);
  }

  importUpdates(updates: Uint8Array): void {
    this.#wasmEditor?.importUpdates(updates);
    this.#wakeUp();
  }

  insertTemplateFragment(snapshot: Uint8Array): void {
    this.#wasmEditor?.insertTemplateFragment(snapshot);
    this.#wakeUp();
  }

  importUpdatesBatch(updatesBatch: Uint8Array[]): void {
    this.#wasmEditor?.importUpdatesBatch(updatesBatch);
    this.#wakeUp();
  }

  checkout(version: Uint8Array): void {
    this.#wasmEditor?.checkout(version);
    this.#wakeUp();
  }

  checkoutToLatest(): void {
    this.#wasmEditor?.checkoutToLatest();
    this.#wakeUp();
  }

  isDetached(): boolean {
    return this.#wasmEditor?.isDetached() ?? false;
  }

  getCharacterCountAtVersion(version: Uint8Array): number | undefined {
    return this.#wasmEditor?.getCharacterCountAtVersion(version);
  }

  revertTo(version: Uint8Array): void {
    this.#wasmEditor?.revertTo(version);
    this.#wakeUp();
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
      const coord = findNearestPageCoordinate(e, pageElements, this.layout.pages[0]?.width ?? 0);
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
    const pageRect = pageElement.getBoundingClientRect();

    // NOTE: continuous 모드에서 캔버스 경계에 걸친 table overlay 클릭 대응
    if (point.x < 0 || point.y < 0 || point.x > pageRect.width || point.y > pageRect.height) {
      return null;
    }

    return {
      pageIdx,
      x: point.x,
      y: point.y,
      pageElement,
      isExtensionArea: false,
    };
  }

  resolvePointerCoordinateFromClient(clientX: number, clientY: number): { pageIdx: number; x: number; y: number } | null {
    const pointEl = document.elementFromPoint(clientX, clientY);
    const targetEl = pointEl instanceof HTMLElement ? pointEl : this.extensionArea.containerEl;
    if (!targetEl) return null;

    const syntheticEvent = { clientX, clientY, target: targetEl } as unknown as MouseEvent;
    const resolved = this.#resolvePointerCoordinate(syntheticEvent, targetEl);
    if (!resolved) return null;

    return {
      pageIdx: resolved.pageIdx,
      x: resolved.x,
      y: resolved.y,
    };
  }

  openContextMenu(options: { x: number; y: number; source: 'mouse' | 'touch'; placement: Placement }): void {
    this.contextMenu.x = options.x;
    this.contextMenu.y = options.y;
    this.contextMenu.source = options.source;
    this.contextMenu.placement = options.placement;
    this.contextMenu.isOpen = true;
  }

  runAfterSettled(task: () => void): void {
    this.#wakeUp();
    void this.settled().then(task);
  }

  handlePointerDown(e: PointerEvent): void {
    if (!(e.target instanceof HTMLElement)) return;

    if (e.target.closest('[data-pointer-capture]')) return;

    const isReadOnlyTouch = this.readOnly && this.#isTouchLikePointer(e);

    if (isReadOnlyTouch) {
      const resolved = this.resolvePointerCoordinateFromClient(e.clientX, e.clientY);
      this.isDraggable = resolved ? this.isSelectionHit(resolved.pageIdx, resolved.x, resolved.y) : false;
      this.touchGesture.handlePointerDown(e, resolved);
      return;
    }

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) {
      this.isDraggable = false;
      return;
    }

    const { pageIdx, x, y, pageElement } = resolved;

    const rect = pageElement.getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const relY = e.clientY - rect.top;
    this.isDraggable = this.isSelectionHit(pageIdx, relX, relY);

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
    if (this.readOnly && this.#isTouchLikePointer(e)) {
      this.touchGesture.handlePointerMove(e);
      return;
    }

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

    const isReadOnlyTouch = this.readOnly && this.#isTouchLikePointer(e);
    if (isReadOnlyTouch) {
      const resolved = this.resolvePointerCoordinateFromClient(e.clientX, e.clientY);
      this.touchGesture.handlePointerUp(e, resolved);
      this.pointer.isPressed = false;
      return;
    }

    if (!(e.target instanceof HTMLElement)) return;

    if (e.target.hasPointerCapture(e.pointerId)) {
      e.target.releasePointerCapture(e.pointerId);
    }

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

  handlePointerCancel(e: PointerEvent): void {
    if (this.readOnly && this.#isTouchLikePointer(e)) {
      this.touchGesture.handlePointerCancel(e);
    }

    this.pointer.isPressed = false;
    this.isDraggable = false;
  }

  handleContextMenu(e: MouseEvent): void {
    if (this.readOnly && this.touchGesture.shouldSuppressNativeContextMenu()) {
      e.preventDefault();
      return;
    }

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

    this.openContextMenu({ x: e.clientX, y: e.clientY, source: 'mouse', placement: 'bottom-start' });
  }

  closeContextMenu(): void {
    this.touchGesture.cancelContextMenuRequest();
    this.contextMenu.isOpen = false;
  }

  async handleCopy(): Promise<void> {
    const data = this.getClipboardData();
    if (data) {
      await this.#writeToClipboard(data.html, data.text);
    }
    this.closeContextMenu();
  }

  async handleCut(): Promise<void> {
    const data = this.getClipboardData();
    if (data) {
      await this.#writeToClipboard(data.html, data.text);
      this.dispatch({ type: 'deleteSelection' }).scrollIntoView();
    }
    this.closeContextMenu();
  }

  async #writeToClipboard(html: string, text: string): Promise<void> {
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

  async handlePaste(): Promise<void> {
    try {
      const items = await navigator.clipboard.read();
      let html: string | undefined = undefined;
      let text = '';
      const imageFiles: File[] = [];

      for (const item of items) {
        const imageMime = item.types.find((type) => type.startsWith('image/'));
        if (imageMime) {
          const imageBlob = await item.getType(imageMime);
          imageFiles.push(new File([imageBlob], 'clipboard-image', { type: imageBlob.type }));
          continue;
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

      if (!this.insertImagesFromFiles(imageFiles)) {
        this.paste({ html, text });
      }
    } catch {
      const text = await navigator.clipboard.readText();
      this.paste({ text });
    } finally {
      this.closeContextMenu();
    }
  }

  paste({ html, text }: { html?: string; text: string }) {
    if (html) {
      this.dispatch({ type: 'pasteHtml', html, text }).scrollIntoView({ mode: 'typewriter' });
    } else if (text !== '') {
      this.dispatch({ type: 'pasteText', text }).scrollIntoView({ mode: 'typewriter' });
    }
  }

  async handlePasteTextOnly(): Promise<void> {
    try {
      const text = await navigator.clipboard.readText();
      this.dispatch({ type: 'pasteText', text }).scrollIntoView();
    } catch {
      // ignore
    }
    this.closeContextMenu();
  }

  handleSelectAll(): void {
    this.dispatch({ type: 'selectAll' });
    this.closeContextMenu();
  }

  markSelectAllShortcut(): void {
    this.#pendingSelectAllShortcut = true;
  }

  handleDragStart(e: DragEvent): void {
    if (!(e.target instanceof HTMLElement)) {
      if (this.readOnly) {
        e.preventDefault();
      }
      return;
    }

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) {
      if (this.readOnly) {
        e.preventDefault();
      }
      return;
    }

    const { pageIdx, x, y, pageElement } = resolved;

    const rect = pageElement.getBoundingClientRect();

    const canStartReadOnlyTouchDrag =
      this.readOnly && this.touchGesture.isReadOnlyTouchDragCandidate() && this.touchGesture.isReadOnlyTouchDragArmed();
    if ((!canStartReadOnlyTouchDrag && this.readOnly) || !this.isSelectionHit(pageIdx, x, y)) {
      e.preventDefault();
      return;
    }

    if (canStartReadOnlyTouchDrag) {
      this.touchGesture.handleNativeDragStart();
    }

    const data = this.getClipboardData();
    if (e.dataTransfer && data) {
      e.dataTransfer.setData('text/html', data.html);
      e.dataTransfer.setData('text/plain', data.text);
      e.dataTransfer.effectAllowed = canStartReadOnlyTouchDrag ? 'copy' : 'copyMove';

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

    this.dispatch({ type: 'dragStart', pageIdx, x, y }).scrollIntoView();
  }

  #renderDragImage(visiblePages: number[], pageIdx: number): { element: HTMLCanvasElement; offsetX: number; offsetY: number } | null {
    const dragImageInfo = this.#wasmEditor?.renderDragImage(new Uint32Array(visiblePages), pageIdx);
    if (!dragImageInfo) return null;

    const { ptr, len, width, height, offsetX, offsetY, scaleFactor } = dragImageInfo;

    const wasmMemory = wasm.getMemory() as WebAssembly.Memory;
    if (!wasmMemory) return null;

    const buffer = new Uint8ClampedArray(wasmMemory.buffer, ptr, len);
    const imageData = new ImageData(new Uint8ClampedArray(buffer), width, height);

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
    ctx.globalAlpha = 0.5;

    ctx.putImageData(imageData, 0, 0);

    const selectedImages = this.#getSelectedDragImages(visiblePages, pageIdx);
    for (const { image, rect } of selectedImages) {
      const destX = (rect.x - offsetX) * scaleFactor;
      const destY = (rect.y - offsetY) * scaleFactor;
      const destW = rect.width * scaleFactor;
      const destH = rect.height * scaleFactor;
      ctx.drawImage(image, destX, destY, destW, destH);
    }

    ctx.restore();

    return { element: canvas, offsetX, offsetY };
  }

  #getSelectedDragImages(
    visiblePages: number[],
    pageIdx: number,
  ): { image: HTMLImageElement; rect: { x: number; y: number; width: number; height: number } }[] {
    const selectedImages: { image: HTMLImageElement; rect: { x: number; y: number; width: number; height: number } }[] = [];

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
          dist += (this.layout.pages[i]?.height ?? 0) + PAGE_GAP;
        }
        relativePageY = el.pageIdx < pageIdx ? -dist : dist;
      }

      let imgElement = document.querySelector(`div[data-node-id="${el.nodeId}"] img:not([loading="lazy"])`);
      if (!imgElement) {
        imgElement = document.querySelector(`div[data-node-id="${el.nodeId}"] img`);
      }

      if (imgElement instanceof HTMLImageElement) {
        const imageId = el.data.id;
        const uploadId = el.data.uploadId;
        const asset = imageId ? this.imageAssets.get(imageId) : undefined;
        const inflight = uploadId ? this.inflightImages.get(uploadId) : undefined;
        const originalWidth = asset?.width ?? inflight?.width ?? 0;
        const originalHeight = asset?.height ?? inflight?.height ?? 0;
        const { displayWidth, xOffset } = calculateImageDisplaySize(el.bounds, originalWidth, originalHeight);

        const globalX = el.bounds.x + xOffset;
        const globalY = relativePageY + el.bounds.y;

        const rect = {
          x: globalX,
          y: globalY,
          width: displayWidth,
          height: el.bounds.height,
        };

        selectedImages.push({ image: imgElement, rect });
      }
    }
    return selectedImages;
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
        })
          .scrollIntoView()
          .focus();
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
        })
          .scrollIntoView()
          .focus();
      }

      if (imageFiles.length > 0 || otherFiles.length > 0) {
        return;
      }
    }

    let html: string | undefined;
    let text: string | undefined;

    if (e.dataTransfer) {
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
      modifier: this.#toModifier(e),
    }).scrollIntoView();
  }

  handleDragEnd(e: DragEvent): void {
    void e;
    this.touchGesture.handleNativeDragEnd();
    this.dispatch({ type: 'dragEnd' });
  }

  can(messageType: string): boolean {
    return this.enabledActions.has(messageType);
  }

  setReadOnly(readOnly: boolean): void {
    this.readOnly = readOnly;
    if (!readOnly) {
      this.touchGesture.resetReadOnlyTouchState();
      this.isDraggable = false;
    }
    this.#wasmEditor?.setReadOnly(readOnly);
    this.#wakeUp();
  }

  setRenderDebug(enabled: boolean): void {
    this.#renderDebugEnabled = enabled;
    this.#wasmEditor?.setRenderDebug(enabled);
    this.#wakeUp();
  }

  setLayoutDebug(enabled: boolean): void {
    this.#layoutDebugEnabled = enabled;
    this.#wasmEditor?.setLayoutDebug(enabled);
    this.#wakeUp();
  }

  isReadOnly(): boolean {
    return this.#wasmEditor?.isReadOnly() ?? this.readOnly;
  }

  setTrackedItems(group: number, items: { id: string; nodeId: string; startOffset: number; endOffset: number }[]): void {
    if (this.#wasmEditor) {
      this.#wasmEditor.setTrackedItems(group, items);
      this.#wakeUp();
    } else {
      this.ready.then(() => {
        this.#wasmEditor?.setTrackedItems(group, items);
        this.#wakeUp();
      });
    }
  }

  removeTrackedItems(group: number, ids: string[]): void {
    if (this.#wasmEditor) {
      this.#wasmEditor.removeTrackedItems(group, ids);
      this.#wakeUp();
    } else {
      this.ready.then(() => {
        this.#wasmEditor?.removeTrackedItems(group, ids);
        this.#wakeUp();
      });
    }
  }

  replaceTextInBlock(blockId: string, startOffset: number, endOffset: number, replacement: string): boolean {
    const result = this.#wasmEditor?.replaceTextInBlock(blockId, startOffset, endOffset, replacement) ?? false;
    this.#wakeUp();
    return result;
  }

  getTextWithMappings(): { text: string; mappings: { nodeId: string; textStart: number; textEnd: number; blockOffset: number }[] } | null {
    return this.#wasmEditor?.getTextWithMappings() ?? null;
  }

  isSelectionHit(pageIdx: number, x: number, y: number): boolean {
    return this.#wasmEditor?.isSelectionHit(pageIdx, x, y) ?? false;
  }

  getClipboardData(): { html: string; text: string } | null {
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
    if (!this.#wasmEditor) return;

    if (!query) {
      this.clearSearch();
      return;
    }

    const matches = this.#wasmEditor.performSearch(query, options?.matchWholeWord ?? false) as {
      nodeId: string;
      startOffset: number;
      endOffset: number;
    }[];

    const items = matches.map((m) => ({
      id: nanoid(),
      nodeId: m.nodeId,
      startOffset: m.startOffset,
      endOffset: m.endOffset,
    }));

    this.searchMatches = items.map((v) => ({ id: v.id, active: false }));
    if (this.searchMatches.length > 0) {
      this.searchMatches[0].active = true;
    }

    this.setTrackedItems(
      2,
      items.map((it) => ({ id: it.id, nodeId: it.nodeId, startOffset: it.startOffset, endOffset: it.endOffset })),
    );

    this.settled().then(() => {
      if (this.searchMatches.length > 0) {
        this.scrollTrackedItemIntoView(this.searchMatches[0].id);
      }
    });
  }

  clearSearch(): void {
    this.searchMatches = [];
    this.setTrackedItems(2, []);
  }

  findNext(): void {
    let found = false;
    for (const match of this.searchMatches) {
      const wasActive = match.active;
      match.active = found;

      if (found) {
        this.scrollTrackedItemIntoView(match.id);
        found = false;
      }

      if (wasActive) {
        found = true;
      }
    }

    if (found && this.searchMatches.length > 0) {
      this.searchMatches[0].active = true;
      this.scrollTrackedItemIntoView(this.searchMatches[0].id);
    }
  }

  findPrevious(): void {
    let found = false;
    for (const match of this.searchMatches.toReversed()) {
      const wasActive = match.active;
      match.active = found;

      if (found) {
        this.scrollTrackedItemIntoView(match.id);
        found = false;
      }

      if (wasActive) {
        found = true;
      }
    }

    if (found && this.searchMatches.length > 0) {
      const last = this.searchMatches.at(-1);
      if (last) {
        last.active = true;
        this.scrollTrackedItemIntoView(last.id);
      }
    }
  }

  replace(replacement: string): void {
    const matchIndex = this.searchMatches.findIndex((v) => v.active);
    if (matchIndex === -1 || !this.#wasmEditor) return;

    const match = this.searchMatches[matchIndex];
    const item = this.trackedItems.find((v) => v.group === 2 && v.id === match.id);
    if (!item) return;

    this.#wasmEditor.replaceTextInBlock(item.nodeId, item.startOffset, item.endOffset, replacement);
    this.#wasmEditor.removeTrackedItems(2, [item.id]);
    this.#wakeUp();

    this.settled().then(() => {
      if (this.searchMatches.length > 0 && !this.searchMatches.some((v) => v.active)) {
        const nextIndex = matchIndex < this.searchMatches.length ? matchIndex : 0;
        this.searchMatches[nextIndex].active = true;
        this.scrollTrackedItemIntoView(this.searchMatches[nextIndex].id);
      }
    });
  }

  replaceAll(replacement: string): void {
    if (this.searchMatches.length === 0 || !this.#wasmEditor) return;

    const ids = new SvelteSet(this.searchMatches.map((v) => v.id));
    const items = this.trackedItems.filter((v) => v.group === 2 && ids.has(v.id));

    this.#wasmEditor.replaceTextInBlocks(items.map((v) => [v.nodeId, v.startOffset, v.endOffset, replacement]));
    this.#wasmEditor.removeTrackedItems(2, [...ids]);
    this.#wakeUp();
  }

  scrollIntoView({ mode = 'auto' }: { mode?: 'auto' | 'typewriter' } = {}): Editor {
    if (mode === 'typewriter') {
      this.#pendingTypewriterRequest = true;
    } else {
      this.pendingScrollConsumer = 'cursor';
      this.#pendingTypewriterRequest = false;
    }
    return this;
  }

  setTypewriterAvailability(enabled: boolean, hasPosition: boolean): void {
    const nextAvailable = enabled && hasPosition;
    if (this.#typewriterAvailable === nextAvailable) {
      return;
    }

    this.#typewriterAvailable = nextAvailable;

    if (!nextAvailable && this.pendingScrollConsumer === 'typewriter') {
      this.pendingScrollConsumer = 'cursor';
    }
  }

  consumePendingScroll(consumer: 'cursor' | 'typewriter'): void {
    if (this.pendingScrollConsumer === consumer) {
      this.pendingScrollConsumer = null;
    }
  }

  focus(): Editor {
    this.inputElement?.focus({ preventScroll: true });

    return this;
  }

  destroy(): void {
    this.#stop();
    this.touchGesture.destroy();
    this.#renderer?.dispose();
    this.#renderer = null;
    this.#wasmEditor = null;
    this.#slateReader = null;
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

  #isTouchLikePointer(e: PointerEvent): boolean {
    return e.pointerType === 'touch';
  }
}
