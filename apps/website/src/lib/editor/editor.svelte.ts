import icuPostcardUrl from '@typie/editor/icu/data.postcard?url';
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
  DIRTY_HTML_PASTED,
  DIRTY_LINK_OVERLAYS,
  DIRTY_PAGES,
  DIRTY_PLACEHOLDER,
  DIRTY_POINTER,
  DIRTY_RENDER_REQUIRED,
  DIRTY_SELECTION,
  DIRTY_SETTINGS,
  DIRTY_TABLE_OVERLAYS,
  DIRTY_TRACKED_ITEMS,
  SlateReader,
} from './slate';
import { calculateImageDisplaySize, calculateRelativePosition, findNearestPageCoordinate, getPageElement, idleCallback } from './utils';
import type { DocExportMode, Editor as WasmEditor, Modifier, PointerButton } from '@typie/editor';
import type { ScrollViewport } from '@typie/ui/utils';
import type { FontFamily } from './fonts';
import type { TableOverlay, TrackedItemOverlay } from './slate';
import type { ThemeColors } from './theme';
import type {
  AiFeedbackData,
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
  SpellcheckErrorData,
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
  #needsTick = false;
  #renderDebugEnabled = false;
  #layoutDebugEnabled = false;
  #onDocChanged?: () => void;
  #onExitedDocumentStart?: () => void;
  #searchQuery = '';
  #searchMatchWholeWord = false;
  #onSelectionChanged?: (anchor: Position, head: Position) => void;
  #readyResolve?: () => void;
  ready: Promise<void>;

  onPaste?: (html: string, text: string) => boolean;

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

  pendingScrollMode = $state<'auto' | 'typewriter' | null>(null);
  #pendingTypewriterRequest = false;

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
  pointerState = $state(0);
  readOnly = $state(false);
  protectContent = $state(false);

  placeholder = $state({
    visible: false,
    bounds: null as Rect | null,
  });

  linkOverlays = $state<{ pageIdx: number; href: string; bounds: Rect[] }[]>([]);

  spellcheckOverlays = $state<TrackedItemOverlay[]>([]);
  activeSpellcheckErrorId = $state<string | null>(null);
  fullSpellcheckErrors = $state<SpellcheckErrorData[]>([]);

  aiFeedbackOverlays = $state<TrackedItemOverlay[]>([]);
  activeAiFeedbackItemId = $state<string | null>(null);
  fullAiFeedbackItems = $state<AiFeedbackData[]>([]);

  searchMatches = $state<{ id: string; nodeId: string; startOffset: number; endOffset: number }[]>([]);
  activeSearchMatchId = $state<string | null>(null);
  searchOverlays = $state<TrackedItemOverlay[]>([]);

  tableOverlays = $state<TableOverlay[]>([]);

  pasteOptions = $state<{
    text: string;
    from: Position;
    to: Position;
  } | null>(null);

  pageVisibility = new SvelteMap<number, number>();

  extensionArea = $state({
    containerEl: null as HTMLElement | null,
    pageElements: [] as HTMLElement[],
  });

  scrollContainerEl = $state<HTMLElement | null>(null);

  scrollViewport = $state<ScrollViewport | null>(null);

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

    if (this.#wasmEditor && this.#slateReader && this.#needsTick) {
      this.#needsTick = false;
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

    this.#rafId = requestAnimationFrame(this.#tick);
  };

  #readSlate(slate: SlateReader): void {
    if (slate.isDirty(DIRTY_DOC_CHANGED)) {
      this.#onDocChanged?.();
      this.pendingScrollMode = 'typewriter';
      this.characterCountsVersion++;
      if (this.#searchQuery) {
        this.#performSearchAndUpdateTrackedItems();
      }
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
          this.pendingScrollMode = 'typewriter';
        }
      } else {
        this.cursor.pageIdx = -1;
        this.cursor.bounds = null;
        this.cursor.visible = false;
        this.pendingScrollMode = null;
      }
    }

    if (slate.isDirty(DIRTY_SELECTION)) {
      if (this.pasteOptions) {
        this.pasteOptions = null;
      }

      const sel = slate.readSelection();
      this.selection = sel;
      this.characterCountsVersion++;
      this.#onSelectionChanged?.(sel.anchor, sel.head);
      this.#updateActiveTrackedItems(sel.head);
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
      const items = slate.readTrackedItems();

      const spellcheckItems: TrackedItemOverlay[] = [];
      const feedbackItems: TrackedItemOverlay[] = [];
      const searchItems: TrackedItemOverlay[] = [];
      for (const item of items) {
        if (item.group === 0) {
          spellcheckItems.push(item);
        } else if (item.group === 1) {
          feedbackItems.push(item);
        } else if (item.group === 2) {
          searchItems.push(item);
        }
      }

      this.spellcheckOverlays = spellcheckItems;
      const spellcheckValidIds = new SvelteSet(spellcheckItems.map((o) => o.id));
      if (this.fullSpellcheckErrors.length > 0) {
        this.fullSpellcheckErrors = this.fullSpellcheckErrors.filter((e: SpellcheckErrorData) => spellcheckValidIds.has(e.id));
      }

      if (this.activeSpellcheckErrorId && !spellcheckValidIds.has(this.activeSpellcheckErrorId)) {
        this.activeSpellcheckErrorId = null;
      }

      const activeSpellcheckOverlay = this.activeSpellcheckErrorId
        ? spellcheckItems.find((o) => o.id === this.activeSpellcheckErrorId)
        : null;
      if (activeSpellcheckOverlay && activeSpellcheckOverlay.bounds.length > 0) {
        this.#scrollOverlayIntoView(activeSpellcheckOverlay);
      }

      this.aiFeedbackOverlays = feedbackItems;
      const feedbackValidIds = new SvelteSet(feedbackItems.map((o) => o.id));
      if (this.fullAiFeedbackItems.length > 0) {
        this.fullAiFeedbackItems = this.fullAiFeedbackItems.filter((e: AiFeedbackData) => feedbackValidIds.has(e.id));
      }

      if (this.activeAiFeedbackItemId && !feedbackValidIds.has(this.activeAiFeedbackItemId)) {
        this.activeAiFeedbackItemId = null;
      }

      const activeFeedbackOverlay = this.activeAiFeedbackItemId ? feedbackItems.find((o) => o.id === this.activeAiFeedbackItemId) : null;
      if (activeFeedbackOverlay && activeFeedbackOverlay.bounds.length > 0) {
        this.#scrollOverlayIntoView(activeFeedbackOverlay);
      }

      this.searchOverlays = searchItems;
      const activeSearchOverlay = this.activeSearchMatchId ? searchItems.find((o) => o.id === this.activeSearchMatchId) : null;
      if (activeSearchOverlay && activeSearchOverlay.bounds.length > 0) {
        this.#scrollOverlayIntoView(activeSearchOverlay);
      }
    }

    if (slate.isDirty(DIRTY_TABLE_OVERLAYS)) {
      this.tableOverlays = slate.readTableOverlays();
    }

    if (slate.isDirty(DIRTY_HTML_PASTED)) {
      const pasted = slate.readHtmlPasted();
      this.pasteOptions = {
        text: pasted.text,
        from: pasted.from,
        to: pasted.to,
      };
    }
  }

  #handleFontRequired(family: string, weight: number, codepoints: number[]): void {
    const font = this.fontFamilies.find((f) => f.familyName === family)?.fonts.find((f) => f.weight === weight);
    if (!font) return;

    ensureRequiredFont(wasm, family, font, codepoints).then(() => {
      this.dispatch({ type: 'fontsLoaded' });
      if (!this.readOnly) {
        preloadRemainingChunks(wasm, family, font);
      }
    });

    filterUncoveredCodepoints(font, codepoints).then((uncovered) => {
      if (uncovered.length > 0) {
        ensureRequiredFallbackFont(wasm, weight, uncovered).then(() => {
          this.dispatch({ type: 'fontsLoaded' });
        });
      }
    });
  }

  #updateActiveTrackedItems(head: Position): void {
    const newSpellcheckId =
      this.spellcheckOverlays.find((o) => o.nodeId === head.nodeId && head.offset >= o.startOffset && head.offset <= o.endOffset)?.id ??
      null;
    if (this.activeSpellcheckErrorId !== newSpellcheckId) {
      this.activeSpellcheckErrorId = newSpellcheckId;
    }

    const newFeedbackId =
      this.aiFeedbackOverlays.find((o) => o.nodeId === head.nodeId && head.offset >= o.startOffset && head.offset <= o.endOffset)?.id ??
      null;
    if (this.activeAiFeedbackItemId !== newFeedbackId) {
      this.activeAiFeedbackItemId = newFeedbackId;
    }
  }

  #scrollOverlayIntoView(overlay: TrackedItemOverlay): void {
    const pageEl = this.pageContainerEls[overlay.pageIdx];
    const scroller = this.scrollContainerEl;
    if (pageEl && scroller && overlay.bounds.length > 0) {
      const pageRect = pageEl.getBoundingClientRect();
      const scrollerRect = scroller.getBoundingClientRect();
      const bound = overlay.bounds[0];
      const targetY = pageRect.top + bound.y - scrollerRect.top + scroller.scrollTop;
      const viewportCenter = scroller.clientHeight / 2;
      const targetScroll = targetY - viewportCenter + bound.height / 2;
      scroller.scrollTo({ top: Math.max(0, targetScroll), behavior: 'smooth' });
    }
  }

  handleRepasteAsText(): void {
    if (!this.pasteOptions) return;

    const { text, from, to } = this.pasteOptions;

    this.dispatch({
      type: 'setSelection',
      anchorNodeId: from.nodeId,
      anchorOffset: from.offset,
      anchorAffinity: from.affinity,
      headNodeId: to.nodeId,
      headOffset: to.offset,
      headAffinity: to.affinity,
    });

    this.dispatch({
      type: 'pasteText',
      text,
    }).scrollIntoView();

    this.pasteOptions = null;
  }

  dispatch(message: Message): Editor {
    this.#wasmEditor?.enqueueMessage(message);
    this.#needsTick = true;

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

  export(mode: DocExportMode): Uint8Array | undefined {
    return this.#wasmEditor?.export(mode);
  }

  importUpdates(updates: Uint8Array): void {
    this.#wasmEditor?.importUpdates(updates);
    this.#needsTick = true;
  }

  insertTemplateFragment(snapshot: Uint8Array): void {
    this.#wasmEditor?.insertTemplateFragment(snapshot);
    this.#needsTick = true;
  }

  importUpdatesBatch(updatesBatch: Uint8Array[]): void {
    this.#wasmEditor?.importUpdatesBatch(updatesBatch);
    this.#needsTick = true;
  }

  checkout(version: Uint8Array): void {
    this.#wasmEditor?.checkout(version);
    this.#needsTick = true;
  }

  checkoutToLatest(): void {
    this.#wasmEditor?.checkoutToLatest();
    this.#needsTick = true;
  }

  isDetached(): boolean {
    return this.#wasmEditor?.isDetached() ?? false;
  }

  getCharacterCountAtVersion(version: Uint8Array): number | undefined {
    return this.#wasmEditor?.getCharacterCountAtVersion(version);
  }

  revertTo(version: Uint8Array): void {
    this.#wasmEditor?.revertTo(version);
    this.#needsTick = true;
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
    this.isDraggable = !this.readOnly && this.isSelectionHit(pageIdx, relX, relY);

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

      for (const item of items) {
        if (item.types.includes('text/html')) {
          const blob = await item.getType('text/html');
          html = await blob.text();
        }
        if (item.types.includes('text/plain')) {
          const blob = await item.getType('text/plain');
          text = await blob.text();
        }
      }

      if (html) {
        if (this.onPaste?.(html, text)) {
          this.closeContextMenu();
          return;
        }
        this.dispatch({ type: 'pasteHtml', html, text }).scrollIntoView();
      } else {
        this.dispatch({ type: 'pasteText', text }).scrollIntoView();
      }
    } catch {
      const text = await navigator.clipboard.readText();
      this.dispatch({ type: 'pasteText', text }).scrollIntoView();
    }
    this.closeContextMenu();
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

  handleDragStart(e: DragEvent): void {
    if (!(e.target instanceof HTMLElement)) return;

    const resolved = this.#resolvePointerCoordinate(e, e.target);
    if (!resolved) return;

    const { pageIdx, x, y, pageElement } = resolved;

    const rect = pageElement.getBoundingClientRect();

    if (this.readOnly || !this.isSelectionHit(pageIdx, x, y)) {
      e.preventDefault();
      return;
    }

    const data = this.getClipboardData();
    if (e.dataTransfer && data) {
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
    this.dispatch({ type: 'dragEnd' });
  }

  can(messageType: string): boolean {
    return this.enabledActions.has(messageType);
  }

  setReadOnly(readOnly: boolean): void {
    this.readOnly = readOnly;
    this.#wasmEditor?.setReadOnly(readOnly);
  }

  setRenderDebug(enabled: boolean): void {
    this.#renderDebugEnabled = enabled;
    this.#wasmEditor?.setRenderDebug(enabled);
    this.#needsTick = true;
  }

  setLayoutDebug(enabled: boolean): void {
    this.#layoutDebugEnabled = enabled;
    this.#wasmEditor?.setLayoutDebug(enabled);
    this.#needsTick = true;
  }

  isReadOnly(): boolean {
    return this.#wasmEditor?.isReadOnly() ?? this.readOnly;
  }

  setTrackedItems(group: number, items: { id: string; nodeId: string; startOffset: number; endOffset: number }[]): void {
    this.ready.then(() => {
      this.#wasmEditor?.setTrackedItems(group, items);
      this.#needsTick = true;
    });
  }

  replaceTextInBlock(blockId: string, startOffset: number, endOffset: number, replacement: string): boolean {
    const result = this.#wasmEditor?.replaceTextInBlock(blockId, startOffset, endOffset, replacement) ?? false;
    this.#needsTick = true;
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

  #performSearchAndUpdateTrackedItems(): void {
    if (!this.#wasmEditor) return;

    const matches = this.#wasmEditor.performSearch(this.#searchQuery, this.#searchMatchWholeWord) as {
      nodeId: string;
      startOffset: number;
      endOffset: number;
    }[];

    const items = matches.map((m, i) => ({
      id: `search-${i}`,
      nodeId: m.nodeId,
      startOffset: m.startOffset,
      endOffset: m.endOffset,
    }));

    this.searchMatches = items;

    if (items.length > 0) {
      const prevId = this.activeSearchMatchId;
      if (!prevId || !items.some((it) => it.id === prevId)) {
        this.activeSearchMatchId = items[0].id;
      }
    } else {
      this.activeSearchMatchId = null;
    }

    this.#wasmEditor.setTrackedItems(
      2,
      items.map((it) => ({ id: it.id, nodeId: it.nodeId, startOffset: it.startOffset, endOffset: it.endOffset })),
    );
  }

  search(query: string, options?: { matchWholeWord?: boolean }): void {
    this.#searchQuery = query;
    this.#searchMatchWholeWord = options?.matchWholeWord ?? false;

    if (!query) {
      this.clearSearch();
      return;
    }

    this.#performSearchAndUpdateTrackedItems();
    this.activeSearchMatchId = this.searchMatches.length > 0 ? this.searchMatches[0].id : null;
    this.#scrollActiveSearchIntoView();
  }

  clearSearch(): void {
    this.#searchQuery = '';
    this.searchMatches = [];
    this.activeSearchMatchId = null;
    this.searchOverlays = [];
    this.#wasmEditor?.setTrackedItems(2, []);
  }

  findNext(): void {
    if (this.searchMatches.length === 0) return;
    const currentIdx = this.searchMatches.findIndex((m) => m.id === this.activeSearchMatchId);
    const nextIdx = (currentIdx + 1) % this.searchMatches.length;
    this.activeSearchMatchId = this.searchMatches[nextIdx].id;
    this.#scrollActiveSearchIntoView();
  }

  findPrevious(): void {
    if (this.searchMatches.length === 0) return;
    const currentIdx = this.searchMatches.findIndex((m) => m.id === this.activeSearchMatchId);
    const prevIdx = currentIdx <= 0 ? this.searchMatches.length - 1 : currentIdx - 1;
    this.activeSearchMatchId = this.searchMatches[prevIdx].id;
    this.#scrollActiveSearchIntoView();
  }

  replace(replacement: string): void {
    const match = this.searchMatches.find((m) => m.id === this.activeSearchMatchId);
    if (!match || !this.#wasmEditor) return;
    this.#wasmEditor.replaceTextInBlock(match.nodeId, match.startOffset, match.endOffset, replacement);
    this.#needsTick = true;
    this.#performSearchAndUpdateTrackedItems();
    this.#scrollActiveSearchIntoView();
  }

  replaceAll(replacement: string): void {
    if (this.searchMatches.length === 0 || !this.#wasmEditor) return;
    const items = this.searchMatches.map((m) => [m.nodeId, m.startOffset, m.endOffset, replacement]);
    this.#wasmEditor.replaceTextInBlocks(items);
    this.#needsTick = true;
    this.#performSearchAndUpdateTrackedItems();
  }

  #scrollActiveSearchIntoView(): void {
    const overlay = this.searchOverlays.find((o) => o.id === this.activeSearchMatchId);
    if (overlay && overlay.bounds.length > 0) {
      this.#scrollOverlayIntoView(overlay);
    }
  }

  scrollIntoView({ mode = 'auto' }: { mode?: 'auto' | 'typewriter' } = {}): Editor {
    if (mode === 'typewriter') {
      this.#pendingTypewriterRequest = true;
    } else {
      this.pendingScrollMode = mode;
      this.#pendingTypewriterRequest = false;
    }
    return this;
  }

  focus(): Editor {
    this.inputElement?.focus({ preventScroll: true });

    return this;
  }

  destroy(): void {
    this.#stop();
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
}
