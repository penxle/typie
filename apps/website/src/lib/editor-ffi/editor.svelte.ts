import { debounce } from '@typie/ui/utils';
import { createContext, tick, untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { match } from 'ts-pattern';
import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
import { IS_MAC } from './constants';
import { fontDataMissingHandler } from './fonts';
import { TouchGestureController } from './gesture.svelte';
import { readClipboardRich, writeClipboardPayload } from './handlers/clipboard';
import { encodeLengthPrefixedBlobs } from './length-prefix';
import { isMutatingMessage } from './message-gate';
import { register, snapshot, unregister } from './registry';
import { zoomDiffers } from './zoom';
import type {
  BlockState,
  ClipboardPayload,
  CursorMetrics,
  Editor as WasmEditor,
  EditorEvent,
  ExternalElement,
  HistoryTag,
  Ime,
  InteractiveHit,
  LinkRect,
  Message,
  ModifierState,
  ModifierType,
  PageRect,
  PlaceholderMetrics,
  PlainDoc,
  PlainRootNode,
  PointerStyle,
  Position,
  Selection,
  SelectionEndpoints,
  Size,
  StableSelection,
  StyleInfo,
  StyleRefValue,
  TableOverlay,
  ThemeVariant,
  TrackedRange,
  Tri,
  Viewport,
} from '@typie/editor-ffi/browser';
import type { ScrollViewport } from '@typie/ui/utils';
import type { EditorScrollIntoViewOptions, EditorScrollScope } from './scroll.svelte';
import type {
  ArchivedAsset,
  ContextMenuContributor,
  ContextMenuContributorContext,
  ContextMenuItem,
  ContextMenuPlacement,
  ContextMenuSource,
  EditorEventListener,
  EmbedAsset,
  FileAsset,
  ImageAsset,
} from './types';

export type SpellcheckError = {
  id: string;
  context: string;
  corrections: string[];
  explanation: string;
};

export type AiFeedback = {
  id: string;
  startText: string;
  endText: string;
  feedback: string;
  category: string | null;
};

export type TrackedRangePosition = Pick<TrackedRange, 'anchor' | 'head'>;

let wasmInitPromise: Promise<void> | null = null;
const VIEWPORT_RESIZE_DEBOUNCE_MS = 50;

/**
 * 검색 결과의 active(현재) 매치에 적용할 selection을 결정한다.
 *
 * 유효한 live tracked range가 주어지면 그 위치를 쓰고(문서 편집으로 밀린 위치 반영),
 * 없으면 검색으로 계산한 `match` 위치로 폴백한다. 호출부는 stale한 tracked range를
 * 넘기지 않도록 `live`를 가려서 전달한다. (TR-252)
 */
export function pickMatchSelection(match: Selection, live?: TrackedRangePosition): Selection {
  return live ? { anchor: live.anchor, head: live.head } : match;
}

function ensureWasmInitialized(): Promise<void> {
  return (wasmInitPromise ??= (async () => {
    await initWasm();
  })());
}

function sameViewport(a: Viewport, b: Viewport): boolean {
  return a.width === b.width && a.height === b.height && a.scale_factor === b.scale_factor;
}

export function browserScaleFactor(): number {
  if (typeof window === 'undefined') {
    return 1;
  }

  const scaleFactor = window.devicePixelRatio * (window.visualViewport?.scale ?? 1);
  return Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
}

function samePosition(a: Position, b: Position): boolean {
  return a.node === b.node && a.offset === b.offset && a.affinity === b.affinity;
}

export class EditorContext {
  editor = $state<Editor>();
  scroll = $state<EditorScrollScope>();
  liveEditor = $state<Editor>();
  fileAssets = $state(new SvelteMap<string, FileAsset>());
  // v1 chrome 호환 필드 — v2 sync 환경에선 갱신되지 않음
  user = $state<unknown>();
  paneFocused = $state(false);
  documentId = $state<string | null>(null);
  serverSnapshot = $state<Uint8Array | undefined>();
  serverVersion = $state<string | null>(null);
  serverGeneration = $state<number>(0);
  resetKey = $state<number>(0);
  pendingImageDrops: File[] = [];
  pendingFileDrops: File[] = [];
  findReplaceOpen = $state(false);
  linkEditorOpen = $state(false);
}

const [getEditorContext, setEditorContext] = createContext<EditorContext>();

export { getEditorContext };
export const setupEditorContext = () => setEditorContext(new EditorContext());

export class Editor {
  static async create(graph: Uint8Array, viewport: Viewport, themeVariant: ThemeVariant = 'light-white') {
    await ensureWasmInitialized();

    const self = new this();

    self.#wasm = wasm.create_editor_from_graph(graph, viewport);
    self.#initInstance(viewport);

    wasm.set_theme_variant(themeVariant);
    self.enqueue({ type: 'system', event: { type: 'theme_variant_changed' } });
    self.enqueue({ type: 'system', event: { type: 'initialize' } });

    self.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'search-match',
        style: {
          background: 'ui.search-match',
          background_radius: 2,
          background_inset: 2,
          underline: undefined,
        },
        enabled: true,
        z_index: 0,
      },
    });
    self.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'search-match-active',
        style: {
          background: 'ui.search-match-active',
          background_radius: 2,
          background_inset: 2,
          underline: undefined,
        },
        enabled: true,
        z_index: 1,
      },
    });

    return self;
  }

  static async createFromDoc(plain: PlainDoc, viewport: Viewport, themeVariant: ThemeVariant = 'light-white'): Promise<Editor> {
    await ensureWasmInitialized();

    const self = new this();

    self.#wasm = wasm.create_editor_from_doc(plain, viewport);
    self.#initInstance(viewport);

    wasm.set_theme_variant(themeVariant);
    self.enqueue({ type: 'system', event: { type: 'theme_variant_changed' } });
    self.enqueue({ type: 'system', event: { type: 'initialize' } });

    return self;
  }

  static async createWithPending(
    server: Uint8Array,
    pending: Uint8Array[],
    viewport: Viewport,
    themeVariant: ThemeVariant = 'light-white',
  ): Promise<Editor> {
    await ensureWasmInitialized();

    const self = new this();

    self.#wasm = wasm.create_editor_from_graph_with_pending(server, encodeLengthPrefixedBlobs(pending), viewport);
    self.#initInstance(viewport);

    wasm.set_theme_variant(themeVariant);
    self.enqueue({ type: 'system', event: { type: 'theme_variant_changed' } });
    self.enqueue({ type: 'system', event: { type: 'initialize' } });

    self.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'search-match',
        style: {
          background: 'ui.search-match',
          background_radius: 2,
          background_inset: 2,
          underline: undefined,
        },
        enabled: true,
        z_index: 0,
      },
    });
    self.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'search-match-active',
        style: {
          background: 'ui.search-match-active',
          background_radius: 2,
          background_inset: 2,
          underline: undefined,
        },
        enabled: true,
        z_index: 1,
      },
    });

    return self;
  }

  static setThemeVariant(variant: ThemeVariant): void {
    const changed = wasm.set_theme_variant(variant);
    if (!changed) return;
    for (const editor of snapshot()) {
      editor.enqueue({ type: 'system', event: { type: 'theme_variant_changed' } });
    }
  }

  #wasm!: WasmEditor;
  #destroyed = false;

  #queued = false;
  #rafId: number | null = null;

  #viewport = $state<Viewport>({ width: 0, height: 0, scale_factor: 1 });
  #appliedViewport: Viewport = { width: 0, height: 0, scale_factor: 1 };
  #firstResizeApplied = false;
  #applyViewportResize = debounce(() => {
    if (this.#destroyed || sameViewport(this.#appliedViewport, this.#viewport)) return;

    this.#applyViewport(this.#viewport);
  }, VIEWPORT_RESIZE_DEBOUNCE_MS);

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #listeners = new Map<EditorEvent['type'], Set<EditorEventListener<EditorEvent['type']>>>();

  #cursor = $state<CursorMetrics>();
  #placeholder = $state<PlaceholderMetrics | undefined>();
  #selection = $state<Selection | undefined>();
  #lastHistoryTag = $state<HistoryTag>();
  #pageSizes = $state<Size[]>([]);
  // Whole-document derived data is expensive (O(pages · N): each builds every
  // page's fragment). It is needed only by pointer/keyboard-driven consumers
  // (link tooltip), so compute it lazily and memoize per tick instead of eagerly
  // every keystroke. Per-page rendering uses the `page*` methods below.
  #externalElementsCache: { rev: number; value: ExternalElement[] } | undefined;
  #tableOverlaysCache: { rev: number; value: TableOverlay[] } | undefined;
  #linkRectsCache: { rev: number; value: LinkRect[] } | undefined;
  #rootAttrs = $state<PlainRootNode>();
  #modifierState = $state<ModifierState | undefined>();
  #blockState = $state<BlockState | undefined>();
  #styleEntries = $state<StyleInfo[]>([]);
  #appliedStyle = $state<Tri<StyleRefValue>>({ type: 'absent' });
  #styleDivergence = $state(false);
  #focused = $state(false);
  #nativeDragAdmissionRetainsFocus = false;
  #effectCleanup: (() => void) | null = null;
  #scrollIntoView: ((options: EditorScrollIntoViewOptions) => void) | null = null;

  #pointerStyle = $state<PointerStyle>('default');
  #lastPointerClient: { x: number; y: number } | null = null;
  #pointerStyleDomRefreshQueued = false;

  #linkHover = $state<{ link: LinkRect; page: number; clientX: number; clientY: number } | undefined>();
  #modifierHeld = $state(false);

  #searchInput = { query: '', matchWholeWord: false };
  #searchMatches = $state<{ id: string; selection: Selection }[]>([]);
  #searchActiveIdx = $state<number | undefined>();

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #contextMenuContributors = new Set<ContextMenuContributor>();

  #gesture!: TouchGestureController;

  #spellcheckDecorationsInstalled = false;

  #aiFeedbackDecorationsInstalled = false;

  #commentDecorationsInstalled = false;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #registeredCommentIds = new Set<string>();

  #characterCountsDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  #tick = (): void => {
    this.#rafId = null;

    if (this.#queued) {
      this.#queued = false;

      const events = this.#wasm.tick();
      for (const event of events) {
        this.#emit(event);
      }
      this.tickRevision += 1;
    }
  };

  #stateChangedHandler: EditorEventListener<'state_changed'> = (_, { fields }) => {
    if (fields.includes('last_history_tag')) {
      this.#lastHistoryTag = this.#wasm.last_history_tag();
    }

    if (fields.includes('cursor')) {
      this.#cursor = this.#wasm.cursor();
    }

    if (fields.includes('placeholder')) {
      this.#placeholder = this.#wasm.placeholder() ?? undefined;
    }

    if (fields.includes('selection')) {
      this.#selection = this.#wasm.selection();
      // null selection is the unfocused state; release DOM focus so OS caret and IME follow.
      if (this.#selection === undefined) {
        this.inputEl?.blur();
      }
      this.#syncActiveSpellcheckErrorFromSelection();
      this.#syncActiveAiFeedbackFromSelection();
    }

    if (fields.includes('doc') || fields.includes('selection')) {
      this.characterCountsVersion++;
    }

    if (fields.includes('doc')) {
      this.documentRevision++;
    }

    if (fields.includes('page_sizes')) {
      this.#pageSizes = this.#wasm.page_sizes();
    }

    // external_elements / table_overlays / link_rects are intentionally NOT
    // recomputed here. The per-tick `tickRevision` bump invalidates their lazy
    // getters and drives the per-visible-page overlay queries, so off-screen
    // pages never build their fragments on a keystroke.

    if (fields.includes('root_attrs')) {
      this.#rootAttrs = this.#wasm.root_attrs();
    }

    if (fields.includes('modifiers')) {
      this.#modifierState = this.#wasm.modifier_state();
    }

    if (fields.includes('block')) {
      this.#blockState = this.#wasm.block_state();
    }

    if (fields.includes('styles')) {
      this.#styleEntries = this.#wasm.style_entries();
      this.#appliedStyle = this.#wasm.applied_style();
    }

    if (fields.includes('styles') || fields.includes('modifiers')) {
      this.#styleDivergence = this.#wasm.style_divergence();
    }

    if (fields.includes('tracked_ranges')) {
      this.trackedRanges = this.#wasm.tracked_ranges();

      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const rangeById = new Map(this.trackedRanges.map((r) => [r.id, r]));

      const isStale = (e: SpellcheckError): boolean => {
        const r = rangeById.get(e.id);
        if (!r) return true;
        if (r.text !== e.context) return true;
        return false;
      };

      for (const e of this.spellcheckErrors) {
        const r = rangeById.get(e.id);
        if (r && r.text !== e.context) {
          this.enqueue({ type: 'tracked_range', op: { type: 'remove', id: e.id } });
        }
      }

      this.spellcheckErrors = this.spellcheckErrors.filter((e) => !isStale(e));

      if (this.activeSpellcheckErrorId !== null && this.spellcheckErrors.every((e) => e.id !== this.activeSpellcheckErrorId)) {
        this.activeSpellcheckErrorId = null;
      }

      this.aiFeedbacks = this.aiFeedbacks.filter((f) => {
        const r = rangeById.get(f.id);
        return r !== undefined;
      });

      if (this.activeAiFeedbackId !== null && this.aiFeedbacks.every((f) => f.id !== this.activeAiFeedbackId)) {
        this.activeAiFeedbackId = null;
      }
    }

    const pageDomChanged = fields.includes('root_attrs') || fields.includes('page_sizes');
    if (pageDomChanged) {
      this.refreshPointerStyleAfterDomUpdate();
    } else if (fields.some((field) => ['doc', 'external_elements', 'modifiers', 'block'].includes(field))) {
      this.refreshPointerStyle();
    }
  };

  tickRevision = $state(0);
  documentRevision = $state(0);

  inputEl = $state<HTMLTextAreaElement>();
  pageEls = $state<Record<number, HTMLDivElement | undefined>>({});
  surfaceEl = $state<HTMLDivElement>();
  scrollContainerEl = $state<HTMLDivElement>();
  scrollViewport = $state<ScrollViewport>();
  scrollRootEl = $state<HTMLElement | null>();
  displayZoom = $state(1);
  renderZoom = $state(1);

  readOnly = $state(false);
  protectContent = $state(false);

  imageAssets = $state(new SvelteMap<string, ImageAsset>());
  inflightImages = $state(new SvelteMap<string, { url: string; width: number; height: number }>());

  contextMenu = $state({
    isOpen: false,
    source: 'mouse' as ContextMenuSource,
    x: 0,
    y: 0,
    placement: 'bottom-start' as ContextMenuPlacement,
    extraItems: [] as ContextMenuItem[],
  });

  inflightFiles = $state(new SvelteMap<string, { name: string; size: number }>());

  spellcheckErrors = $state<SpellcheckError[]>([]);
  trackedRanges = $state<TrackedRange[]>([]);
  activeSpellcheckErrorId = $state<string | null>(null);

  aiFeedbacks = $state<AiFeedback[]>([]);
  activeAiFeedbackId = $state<string | null>(null);

  activeCommentId = $state<string | null>(null);
  commentClickHandler: ((id: string) => void) | null = null;
  requestCommentCompose: (() => void) | null = null;

  embedAssets = $state(new SvelteMap<string, EmbedAsset>());
  archivedAssets = $state(new SvelteMap<string, ArchivedAsset>());

  characterCounts = $state({
    docWithWhitespace: 0,
    docWithoutWhitespace: 0,
    docWithoutWhitespaceAndPunctuation: 0,
    selectionWithWhitespace: 0,
    selectionWithoutWhitespace: 0,
    selectionWithoutWhitespaceAndPunctuation: 0,
  });
  characterCountsVersion = $state(0);

  private constructor() {
    // no-op
  }

  #applyViewport(viewport: Viewport): void {
    this.#appliedViewport = viewport;
    this.enqueue({
      type: 'system',
      event: { type: 'resize', width: viewport.width, height: viewport.height, scale_factor: viewport.scale_factor },
    });
  }

  #setFocused(focused: boolean): void {
    if (this.#focused === focused) {
      return;
    }

    this.#focused = focused;
    this.enqueue({ type: 'system', event: { type: 'set_focused', focused } });
  }

  #currentCursorLineRect(): PageRect | null {
    const cursor = this.#cursor;
    if (cursor) {
      return { page_idx: cursor.page_idx, rect: cursor.line };
    }
    return null;
  }

  #scheduleTick(): void {
    if (this.#queued) {
      return;
    }

    this.#queued = true;

    if (this.#rafId === null) {
      this.#rafId = requestAnimationFrame(this.#tick);
    }
  }

  #moveActive(delta: number): void {
    const len = this.#searchMatches.length;
    if (len === 0) return;
    const current = this.#searchActiveIdx ?? 0;
    const next = (((current + delta) % len) + len) % len;
    this.#searchActiveIdx = next;
    this.#applyActiveMark();
    this.#scrollToActiveMatch();
  }

  #applyActiveMark(preferFresh = false): void {
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match-active' } });
    const idx = this.#searchActiveIdx;
    if (idx === undefined) return;
    const match = this.#searchMatches[idx];
    if (!match) return;
    // search() 직후엔 새 매치가 아직 flush되지 않아 tracked range가 이전 검색어의 위치를
    // 들고 있다(인덱스 기반 id 재사용으로 충돌). 이때는 조회를 건너뛰고 검색 결과를 그대로 쓴다.
    const live = preferFresh ? undefined : this.#wasm.tracked_ranges('search-match').find((r) => r.id === match.id);
    const selection = pickMatchSelection(match.selection, live);
    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'add',
        id: `search-match-active:${match.id}`,
        group: 'search-match-active',
        selection,
        metadata: '',
      },
    });
  }

  #scrollToActiveMatch(): void {
    const idx = this.#searchActiveIdx;
    if (idx === undefined) return;
    const match = this.#searchMatches[idx];
    if (!match) return;
    this.scrollIntoView({ target: { type: 'tracked_item', id: match.id } });
  }

  #handleSearchReplaceResult(id: string, outcome: string): void {
    if (outcome !== 'replaced') return;
    const idx = this.#searchMatches.findIndex((m) => m.id === id);
    if (idx === -1) return;
    this.#searchMatches = this.#searchMatches.filter((_, i) => i !== idx);
    this.enqueue({ type: 'tracked_range', op: { type: 'remove', id } });
    this.enqueue({
      type: 'tracked_range',
      op: { type: 'remove', id: `search-match-active:${id}` },
    });
    const len = this.#searchMatches.length;
    if (len === 0) {
      this.#searchActiveIdx = undefined;
    } else {
      this.#searchActiveIdx = Math.min(idx, len - 1);
      this.#applyActiveMark();
      this.#scrollToActiveMatch();
    }
  }

  #syncActiveSpellcheckErrorFromSelection(): void {
    const cursor = this.#cursor;
    if (!cursor) return;

    const cx = cursor.caret.x;
    const cy = cursor.line.y + cursor.line.height / 2;
    const pageIdx = cursor.page_idx;

    const hit =
      this.#wasm.tracked_ranges_at(pageIdx, cx, cy, 'spellcheck-active')[0] ??
      this.#wasm.tracked_ranges_at(pageIdx, cx, cy, 'spellcheck')[0];

    if (hit) {
      this.setActiveSpellcheckError(hit.id);
    } else if (this.activeSpellcheckErrorId !== null) {
      this.setActiveSpellcheckError(null);
    }
  }

  #syncActiveAiFeedbackFromSelection(): void {
    const cursor = this.#cursor;
    if (!cursor) return;

    const cx = cursor.caret.x;
    const cy = cursor.line.y + cursor.line.height / 2;
    const pageIdx = cursor.page_idx;

    const hit =
      this.#wasm.tracked_ranges_at(pageIdx, cx, cy, 'ai-feedback-active')[0] ??
      this.#wasm.tracked_ranges_at(pageIdx, cx, cy, 'ai-feedback')[0];

    if (hit) {
      this.setActiveAiFeedback(hit.id);
    } else if (this.activeAiFeedbackId !== null) {
      this.setActiveAiFeedback(null);
    }
  }

  #emit(event: EditorEvent): void {
    const set = this.#listeners.get(event.type);
    if (set) {
      for (const cb of set) {
        (cb as EditorEventListener<typeof event.type>)(this, event as never);
      }
    }
  }

  #handleSpellcheckReplaceResult(id: string): void {
    this.removeSpellcheckError(id);
  }

  #initInstance(viewport: Viewport): void {
    this.#viewport = viewport;
    this.#appliedViewport = viewport;
    this.#gesture = new TouchGestureController(this);

    this.on('state_changed', this.#stateChangedHandler);
    this.on('font_data_missing', fontDataMissingHandler);
    this.on('tracked_range_replace_result', (_, { id, outcome }) => {
      if (id.startsWith('search-match:')) {
        this.#handleSearchReplaceResult(id, outcome);
      } else if (this.spellcheckErrors.some((e) => e.id === id)) {
        this.#handleSpellcheckReplaceResult(id);
      }
    });

    register(this);

    this.#effectCleanup = $effect.root(() => {
      $effect(() => {
        const el = this.inputEl;
        if (!el) {
          untrack(() => this.#setFocused(false));
          return;
        }

        const onFocus = () => {
          this.#setFocused(true);
        };
        const onBlur = () => {
          if (this.#nativeDragAdmissionRetainsFocus) {
            return;
          }

          this.#setFocused(false);
        };

        el.addEventListener('focus', onFocus);
        el.addEventListener('blur', onBlur);
        untrack(() => this.#setFocused(document.activeElement === el));

        return () => {
          el.removeEventListener('focus', onFocus);
          el.removeEventListener('blur', onBlur);
        };
      });

      $effect(() => {
        const isHeld = (e: KeyboardEvent) => (IS_MAC ? e.metaKey : e.ctrlKey);
        const onKey = (e: KeyboardEvent) => {
          this.#modifierHeld = isHeld(e);
        };
        const onBlur = () => {
          this.#modifierHeld = false;
        };
        window.addEventListener('keydown', onKey);
        window.addEventListener('keyup', onKey);
        window.addEventListener('blur', onBlur);
        return () => {
          window.removeEventListener('keydown', onKey);
          window.removeEventListener('keyup', onKey);
          window.removeEventListener('blur', onBlur);
        };
      });
    });
  }

  get gesture(): TouchGestureController {
    return this.#gesture;
  }

  setDoc(plain: PlainDoc): void {
    if (this.#destroyed) return;
    this.#wasm.set_doc(plain);
    this.#scheduleTick();
  }

  materializeAt(heads: Uint8Array): PlainDoc {
    return this.#wasm.materialize_at(heads);
  }

  get cursor() {
    return this.#cursor;
  }

  get placeholder(): PlaceholderMetrics | undefined {
    return this.#placeholder;
  }

  get selection() {
    return this.#selection;
  }

  freezeSelection(selection: Selection): StableSelection | undefined {
    try {
      return this.#wasm.freeze_selection(selection);
    } catch {
      return undefined;
    }
  }

  get isSelectionCollapsed(): boolean {
    const sel = this.#selection;
    if (!sel) return true;
    return sel.anchor.node === sel.head.node && sel.anchor.offset === sel.head.offset && sel.anchor.affinity === sel.head.affinity;
  }

  get pageSizes() {
    return this.#pageSizes;
  }

  get externalElements(): ExternalElement[] {
    const rev = this.tickRevision;
    if (this.#externalElementsCache?.rev !== rev) {
      this.#externalElementsCache = { rev, value: this.#wasm.external_elements() };
    }
    return this.#externalElementsCache.value;
  }

  get tableOverlays(): TableOverlay[] {
    const rev = this.tickRevision;
    if (this.#tableOverlaysCache?.rev !== rev) {
      this.#tableOverlaysCache = { rev, value: this.#wasm.table_overlays() };
    }
    return this.#tableOverlaysCache.value;
  }

  get linkRects(): LinkRect[] {
    const rev = this.tickRevision;
    if (this.#linkRectsCache?.rev !== rev) {
      this.#linkRectsCache = { rev, value: this.#wasm.link_rects() };
    }
    return this.#linkRectsCache.value;
  }

  // Per-visible-page derived data — `O(N)` for one page instead of `O(pages · N)`
  // for the whole document. Drives the per-page overlay rendering.
  pageExternalElements(page: number): ExternalElement[] {
    return this.#wasm.page_external_elements(page);
  }

  pageTableOverlays(page: number): TableOverlay[] {
    return this.#wasm.page_table_overlays(page);
  }

  pageLinkRects(page: number): LinkRect[] {
    return this.#wasm.page_link_rects(page);
  }

  get rootAttrs() {
    return this.#rootAttrs;
  }

  get rootModifiers() {
    return this.#styleEntries.find((s) => s.id === 'base')?.modifiers;
  }

  get modifierState() {
    return this.#modifierState;
  }

  get blockState() {
    return this.#blockState;
  }

  get lastHistoryTag() {
    return this.#lastHistoryTag;
  }

  get styleEntries() {
    return this.#styleEntries;
  }

  get appliedStyle() {
    return this.#appliedStyle;
  }

  get styleDivergence() {
    return this.#styleDivergence;
  }

  get scaleFactor() {
    return this.#viewport.scale_factor;
  }

  get surfaceScaleFactor() {
    return this.#viewport.scale_factor * this.renderZoom;
  }

  get viewportResized(): boolean {
    return this.#firstResizeApplied;
  }

  resizeViewport(width: number, height: number, scaleFactor: number): void {
    if (!Number.isFinite(width) || !Number.isFinite(height) || !Number.isFinite(scaleFactor)) return;
    if (width <= 0 || height <= 0 || scaleFactor <= 0) return;

    const viewport = { width, height, scale_factor: scaleFactor };
    if (sameViewport(this.#viewport, viewport)) return;

    this.#viewport = viewport;
    if (sameViewport(this.#appliedViewport, viewport)) return;

    if (!this.#firstResizeApplied) {
      this.#firstResizeApplied = true;
      this.#applyViewport(viewport);
      return;
    }

    this.#applyViewportResize.call();
  }

  setRenderZoom(renderZoom: number): void {
    const safeRenderZoom = Number.isFinite(renderZoom) && renderZoom > 0 ? renderZoom : 1;
    if (zoomDiffers(this.renderZoom, safeRenderZoom)) {
      this.renderZoom = safeRenderZoom;
    }
  }

  focus() {
    this.inputEl?.focus({ preventScroll: true });
  }

  blur() {
    this.inputEl?.blur();
  }

  beginNativeDragAdmission() {
    this.#nativeDragAdmissionRetainsFocus = true;
    this.#setFocused(true);
  }

  endNativeDragAdmission({ restoreFocus }: { restoreFocus: boolean }) {
    const wasRetainingFocus = this.#nativeDragAdmissionRetainsFocus;
    this.#nativeDragAdmissionRetainsFocus = false;
    if (!wasRetainingFocus) {
      return;
    }

    if (restoreFocus && this.#selection !== undefined) {
      this.focus();
    } else if (!this.inputEl || document.activeElement !== this.inputEl) {
      this.#setFocused(false);
    }
  }

  openContextMenu(opts: {
    x: number;
    y: number;
    source: ContextMenuSource;
    placement: ContextMenuPlacement;
    extraItems?: ContextMenuItem[];
  }): void {
    this.contextMenu.x = opts.x;
    this.contextMenu.y = opts.y;
    this.contextMenu.source = opts.source;
    this.contextMenu.placement = opts.placement;
    this.contextMenu.extraItems = opts.extraItems ?? [];
    this.contextMenu.isOpen = true;
  }

  closeContextMenu(): void {
    if (!this.contextMenu.isOpen) return;
    this.contextMenu.isOpen = false;
    this.contextMenu.extraItems = [];
  }

  registerContextMenuContributor(fn: ContextMenuContributor): () => void {
    this.#contextMenuContributors.add(fn);
    return () => {
      this.#contextMenuContributors.delete(fn);
    };
  }

  collectContextMenuContributions(ctx: ContextMenuContributorContext): ContextMenuItem[] {
    const items: ContextMenuItem[] = [];
    for (const fn of this.#contextMenuContributors) {
      items.push(...fn(ctx));
    }
    return items;
  }

  requestSelectAll(): void {
    this.enqueue({ type: 'selection', op: { type: 'expand', unit: 'all' } });
  }

  async requestCopy(): Promise<void> {
    if (this.isSelectionCollapsed) return;
    if (this.readOnly && this.protectContent) return;
    const payload = this.copySelection();
    if (!payload) return;
    await writeClipboardPayload(payload.html, payload.text);
  }

  async requestCut(): Promise<void> {
    if (this.isSelectionCollapsed || this.readOnly) return;
    const payload = this.copySelection();
    if (!payload) return;
    await writeClipboardPayload(payload.html, payload.text);
    this.enqueue({ type: 'clipboard', op: { type: 'cut' } });
  }

  async requestPaste(): Promise<void> {
    if (this.readOnly) return;
    const result = await readClipboardRich();
    if (!result) return;
    // HTML-only clipboard is intentionally accepted here (the WASM Paste op handles text: '' with non-empty html); diverges from handlePaste which gates on text being present.
    const hasContent = result.html !== undefined || result.text !== '';
    if (!hasContent) return;
    this.enqueue({ type: 'clipboard', op: { type: 'paste', html: result.html, text: result.text } });
  }

  async requestPasteTextOnly(): Promise<void> {
    if (this.readOnly) return;
    const result = await readClipboardRich();
    if (!result || result.text === '') return;
    this.enqueue({ type: 'clipboard', op: { type: 'paste', html: undefined, text: result.text } });
  }

  insertTemplateFragment(changesets: Uint8Array): void {
    if (this.readOnly) return;
    this.#wasm.insert_template_fragment(changesets);
    this.#scheduleTick();
  }

  handleRepasteAsText(): void {
    if (this.readOnly) return;
    this.enqueue({ type: 'clipboard', op: { type: 'repaste_as_text' } });
    this.focus();
  }

  get focusable() {
    return !!this.inputEl;
  }

  get viewport() {
    return this.#viewport;
  }

  get hasQueuedTick() {
    return this.#queued;
  }

  get destroyed() {
    return this.#destroyed;
  }

  get focused() {
    return this.#focused;
  }

  get pointerStyle() {
    return this.#pointerStyle;
  }

  get linkHover() {
    return this.#linkHover;
  }

  get modifierHeld() {
    return this.#modifierHeld;
  }

  linkHitTestAtClient(clientX: number, clientY: number): { link: LinkRect; page: number } | undefined {
    const local = this.clientToLocal(clientX, clientY);
    if (!local) return undefined;
    const link = this.#wasm.link_hit_test(local.page, local.x, local.y);
    return link ? { link, page: local.page } : undefined;
  }

  safeDisplayZoom(): number {
    const zoom = this.#rootAttrs?.layout_mode.type === 'paginated' ? this.displayZoom : 1;
    return Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
  }

  clientDeltaToLocalDelta(delta: number): number {
    return delta / this.safeDisplayZoom();
  }

  selectionHeadRect(): PageRect | null {
    const selection = this.#selection;
    if (!selection) return this.#currentCursorLineRect();

    const endpoints = this.selectionEndpoints();
    if (!endpoints) return this.#currentCursorLineRect();
    return samePosition(selection.head, endpoints.to_position) ? endpoints.to : endpoints.from;
  }

  trackedItemFirstRect(id: string): PageRect | null {
    return this.trackedItemRects(id)?.[0] ?? null;
  }

  trackedItemRects(id: string): PageRect[] | null {
    const range = this.trackedRanges.find((r) => r.id === id);
    if (!range) return null;
    return range.rects.length > 0 ? range.rects : null;
  }

  registerScrollIntoView(handler: ((options: EditorScrollIntoViewOptions) => void) | null): void {
    this.#scrollIntoView = handler;
  }

  scrollIntoView(options: EditorScrollIntoViewOptions): void {
    this.#scrollIntoView?.(options);
  }

  clientToLocal(clientX: number, clientY: number) {
    const pages = this.#pageSizes;
    if (pages.length === 0) return null;
    const zoom = this.safeDisplayZoom();

    let lo = 0;
    let hi = pages.length - 1;

    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      const el = this.pageEls[mid];
      if (!el) return null;
      const rect = el.getBoundingClientRect();
      if (rect.bottom <= clientY) lo = mid + 1;
      else hi = mid;
    }

    const el = this.pageEls[lo];
    if (!el) return null;
    let rect = el.getBoundingClientRect();
    let localY = (clientY - rect.top) / zoom;

    if (localY < 0 && lo > 0) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const prevRect = this.pageEls[lo - 1]!.getBoundingClientRect();
      if (clientY < (prevRect.bottom + rect.top) / 2) {
        lo--;
        rect = prevRect;
        localY = pages[lo].height;
      } else {
        localY = 0;
      }
    }

    const size = pages[lo];
    const localX = Math.max(0, Math.min((clientX - rect.left) / zoom, size.width));
    localY = Math.max(0, Math.min(localY, size.height));
    return { page: lo, x: localX, y: localY };
  }

  interactiveHitTest(page: number, x: number, y: number): InteractiveHit | undefined {
    return this.#wasm.interactive_hit_test(page, x, y);
  }

  selectionEndpoints(): SelectionEndpoints | undefined {
    return this.#wasm.selection_endpoints();
  }

  modifierSpanSelection(pos: Position, modifierType: ModifierType): Selection | undefined {
    return this.#wasm.modifier_span_selection(pos, modifierType);
  }

  ime(beforeLimit: number, afterLimit: number): Ime {
    return this.#wasm.ime(beforeLimit, afterLimit);
  }

  selectionHitTest(page: number, x: number, y: number): boolean {
    return this.#wasm.selection_hit_test(page, x, y);
  }

  updatePointerHover(clientX: number, clientY: number): void {
    this.#lastPointerClient = { x: clientX, y: clientY };
    this.refreshPointerStyle();
  }

  refreshPointerStyle(): void {
    if (this.#destroyed) {
      return;
    }

    const pointer = this.#lastPointerClient;
    if (!pointer) {
      return;
    }

    const local = this.clientToLocal(pointer.x, pointer.y);
    if (local) {
      this.#pointerStyle = this.#wasm.pointer_style(local.page, local.x, local.y, this.readOnly);
      const link = this.#wasm.link_hit_test(local.page, local.x, local.y);
      this.#linkHover = link ? { link, page: local.page, clientX: pointer.x, clientY: pointer.y } : undefined;
    } else {
      this.#pointerStyle = 'default';
      this.#linkHover = undefined;
    }
  }

  clearLinkHover(): void {
    this.#lastPointerClient = null;
    this.#pointerStyle = 'default';
    this.#linkHover = undefined;
  }

  refreshPointerStyleAfterDomUpdate(): void {
    if (this.#destroyed || !this.#lastPointerClient || this.#pointerStyleDomRefreshQueued) {
      return;
    }

    this.#pointerStyleDomRefreshQueued = true;
    void tick().then(() => {
      this.#pointerStyleDomRefreshQueued = false;
      if (!this.#destroyed) {
        this.refreshPointerStyle();
      }
    });
  }

  on<K extends EditorEvent['type']>(event: K, callback: EditorEventListener<K>): () => void {
    let set = this.#listeners.get(event);
    if (!set) {
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      set = new Set();
      this.#listeners.set(event, set);
    }

    set.add(callback as never);
    return () => set.delete(callback as never);
  }

  enqueue(message: Message) {
    if (this.readOnly && isMutatingMessage(message)) {
      return;
    }
    this.#wasm.enqueue(message);
    this.#scheduleTick();
  }

  flush(): void {
    if (this.#rafId !== null) {
      cancelAnimationFrame(this.#rafId);
      this.#rafId = null;
    }
    if (!this.#queued) {
      return;
    }
    this.#queued = false;
    const events = this.#wasm.tick();
    for (const event of events) {
      this.#emit(event);
    }
    this.tickRevision += 1;
  }

  attachSurface(page: number, canvas: HTMLCanvasElement, width: number, height: number): void {
    if (this.#destroyed) return;
    this.#wasm.attach_surface(page, canvas, width, height, this.surfaceScaleFactor);
  }

  detachSurface(page: number): void {
    if (this.#destroyed) return;
    this.#wasm.detach_surface(page);
  }

  renderSurface(page: number): void {
    if (this.#destroyed) return;
    this.#wasm.render_surface(page);
  }

  resizeSurface(page: number, width: number, height: number): void {
    if (this.#destroyed) return;
    this.#wasm.resize_surface(page, width, height, this.surfaceScaleFactor);
  }

  setExternalElementHeight(nodeId: string, height: number): void {
    this.enqueue({ type: 'system', event: { type: 'set_external_height', node_id: nodeId, height } });
  }

  setThemeVariant(variant: ThemeVariant): void {
    Editor.setThemeVariant(variant);
  }

  currentHeads(): Uint8Array {
    return this.#wasm.current_heads();
  }

  localChangesetsSince(remoteHeads: Uint8Array): Uint8Array {
    return this.#wasm.local_changesets_since(remoteHeads);
  }

  missingChangesetsFor(confirmedHeads: Uint8Array): Uint8Array {
    return this.#wasm.missing_changesets_tolerant(confirmedHeads);
  }

  partitionRemoteChangesets(payload: Uint8Array): { ready: Uint8Array; blocked: Uint8Array } {
    const result = this.#wasm.partition_remote_changesets(payload);
    return { ready: new Uint8Array(result.ready), blocked: new Uint8Array(result.blocked) };
  }

  splitChangesets(payload: Uint8Array): { id: string; bytes: Uint8Array }[] {
    return this.#wasm.split_changesets(payload).map((e) => ({ id: e.id, bytes: new Uint8Array(e.bytes) }));
  }

  receiveRemoteChangeset(payload: Uint8Array): void {
    this.#wasm.receive_remote_changeset(payload);
    this.#scheduleTick();
  }

  copySelection(): ClipboardPayload | undefined {
    return this.#wasm.copy_selection();
  }

  get searchMatches(): { active: boolean }[] {
    const matches = this.#searchMatches;
    const active = this.#searchActiveIdx;
    return matches.map((_, i) => ({ active: i === active }));
  }

  search(query: string, options?: { matchWholeWord?: boolean }): void {
    const matchWholeWord = options?.matchWholeWord ?? false;
    const previousInput = this.#searchInput;
    const searchInputChanged = previousInput.query !== query || previousInput.matchWholeWord !== matchWholeWord;
    const previousActiveIndex = this.#searchActiveIdx;
    this.#searchInput = { query, matchWholeWord };

    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match' } });
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match-active' } });

    if (query.length === 0) {
      this.#searchMatches = [];
      this.#searchActiveIdx = undefined;
      return;
    }

    const selections = this.#wasm.find_matches(query, { match_whole_word: matchWholeWord });
    const matches = selections.map((selection, i) => ({ id: `search-match:${i}`, selection }));

    for (const m of matches) {
      this.enqueue({
        type: 'tracked_range',
        op: { type: 'add', id: m.id, group: 'search-match', selection: m.selection, metadata: '' },
      });
    }

    this.#searchMatches = matches;
    this.#searchActiveIdx =
      matches.length === 0
        ? undefined
        : searchInputChanged || previousActiveIndex === undefined
          ? 0
          : Math.min(previousActiveIndex, matches.length - 1);
    if (this.#searchActiveIdx !== undefined) {
      this.#applyActiveMark(true);
      if (searchInputChanged) {
        this.#scrollToActiveMatch();
      }
    }
  }

  clearSearch(): void {
    this.#searchInput = { query: '', matchWholeWord: false };
    this.#searchMatches = [];
    this.#searchActiveIdx = undefined;
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match' } });
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match-active' } });
  }

  findNext(): void {
    this.#moveActive(1);
  }

  findPrevious(): void {
    this.#moveActive(-1);
  }

  replace(replacement: string): void {
    if (replacement.includes('\n') || replacement.includes('\r')) return;
    if (this.#searchActiveIdx === undefined) return;
    const match = this.#searchMatches[this.#searchActiveIdx];
    if (!match) return;
    this.enqueue({
      type: 'tracked_range',
      op: { type: 'replace_text', id: match.id, expected_text: this.#searchInput.query, replacement },
    });
  }

  replaceAll(replacement: string): void {
    if (replacement.includes('\n') || replacement.includes('\r')) return;
    const query = this.#searchInput.query;
    if (query.length === 0) return;
    for (const m of this.#searchMatches) {
      this.enqueue({
        type: 'tracked_range',
        op: { type: 'replace_text', id: m.id, expected_text: query, replacement },
      });
    }
    this.#searchMatches = [];
    this.#searchActiveIdx = undefined;
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match' } });
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'search-match-active' } });
  }

  proseText(): string {
    this.flush();
    return this.#wasm.prose_text();
  }

  proseToSelection(start: number, end: number): Selection | undefined {
    this.flush();
    return this.#wasm.prose_to_selection(start, end) ?? undefined;
  }

  updateCharacterCounts(): void {
    if (this.#characterCountsDebounceTimer) {
      clearTimeout(this.#characterCountsDebounceTimer);
    }

    this.#characterCountsDebounceTimer = setTimeout(() => {
      this.#characterCountsDebounceTimer = null;
      if (this.#destroyed) {
        return;
      }

      const counts = this.#wasm.character_counts();
      this.characterCounts = {
        docWithWhitespace: counts.doc_with_whitespace,
        docWithoutWhitespace: counts.doc_without_whitespace,
        docWithoutWhitespaceAndPunctuation: counts.doc_without_whitespace_and_punctuation,
        selectionWithWhitespace: counts.selection_with_whitespace,
        selectionWithoutWhitespace: counts.selection_without_whitespace,
        selectionWithoutWhitespaceAndPunctuation: counts.selection_without_whitespace_and_punctuation,
      };
    }, 150);
  }

  installSpellcheckDecorations(): void {
    if (this.#spellcheckDecorationsInstalled) return;
    this.#spellcheckDecorationsInstalled = true;

    const underline = { color: 'text.red', style: 'wavy' as const, thickness: 1.5 };

    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'spellcheck',
        style: { background: undefined, underline },
        enabled: true,
      },
    });

    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'spellcheck-active',
        style: { background: 'bg.red', underline },
        enabled: true,
      },
    });
  }

  installAiFeedbackDecorations(): void {
    if (this.#aiFeedbackDecorationsInstalled) return;
    this.#aiFeedbackDecorationsInstalled = true;

    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'ai-feedback',
        style: { background: 'bg.blue', background_radius: 2, background_inset: 2, underline: undefined },
        enabled: true,
        z_index: 0,
      },
    });

    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'ai-feedback-active',
        style: { background: 'bg.blue', background_radius: 2, background_inset: 0, underline: undefined },
        enabled: true,
        z_index: 1,
      },
    });
  }

  setSpellcheckErrors(
    items: {
      id: string;
      selection: Selection;
      context: string;
      corrections: string[];
      explanation: string;
    }[],
  ): void {
    for (const item of items) {
      this.enqueue({
        type: 'tracked_range',
        op: {
          type: 'add',
          id: item.id,
          group: 'spellcheck',
          selection: item.selection,
          metadata: '',
        },
      });
    }
    this.spellcheckErrors = items.map((item) => ({
      id: item.id,
      context: item.context,
      corrections: item.corrections,
      explanation: item.explanation,
    }));
  }

  removeSpellcheckError(id: string): void {
    this.enqueue({ type: 'tracked_range', op: { type: 'remove', id } });
    this.spellcheckErrors = this.spellcheckErrors.filter((e) => e.id !== id);
    if (this.activeSpellcheckErrorId === id) {
      this.activeSpellcheckErrorId = null;
    }
  }

  applySpellcheckCorrection(id: string, replacement: string): void {
    const err = this.spellcheckErrors.find((e) => e.id === id);
    if (!err) return;

    this.enqueue({
      type: 'tracked_range',
      op: { type: 'replace_text', id, expected_text: err.context, replacement },
    });
    this.enqueue({
      type: 'tracked_range',
      op: { type: 'remove', id },
    });

    this.spellcheckErrors = this.spellcheckErrors.filter((e) => e.id !== id);
    if (this.activeSpellcheckErrorId === id) {
      this.activeSpellcheckErrorId = null;
    }
  }

  setActiveSpellcheckError(id: string | null): void {
    if (this.activeSpellcheckErrorId === id) return;

    const restoreToNormalGroup = (errorId: string) => {
      if (this.trackedRanges.every((r) => r.id !== errorId)) return;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: errorId, group: 'spellcheck' } });
    };

    const promoteToActiveGroup = (errorId: string): boolean => {
      if (this.trackedRanges.every((r) => r.id !== errorId)) return false;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: errorId, group: 'spellcheck-active' } });
      return true;
    };

    if (this.activeSpellcheckErrorId !== null) {
      restoreToNormalGroup(this.activeSpellcheckErrorId);
    }

    this.activeSpellcheckErrorId = id;

    if (id !== null) {
      const ok = promoteToActiveGroup(id);
      if (!ok) {
        this.activeSpellcheckErrorId = null;
        return;
      }
      this.scrollIntoView({ target: { type: 'tracked_item', id } });
    }
  }

  removeSpellcheckErrorsByContext(context: string): void {
    const targets = this.spellcheckErrors.filter((e) => e.context === context).map((e) => e.id);
    if (targets.length === 0) return;

    for (const id of targets) {
      this.enqueue({ type: 'tracked_range', op: { type: 'remove', id } });
    }
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const targetSet = new Set(targets);
    this.spellcheckErrors = this.spellcheckErrors.filter((e) => !targetSet.has(e.id));
    if (this.activeSpellcheckErrorId !== null && targetSet.has(this.activeSpellcheckErrorId)) {
      this.activeSpellcheckErrorId = null;
    }
  }

  clearSpellcheckErrors(): void {
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'spellcheck' } });
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'spellcheck-active' } });
    this.spellcheckErrors = [];
    this.activeSpellcheckErrorId = null;
  }

  addAiFeedback(item: {
    id: string;
    selection: Selection;
    startText: string;
    endText: string;
    feedback: string;
    category: string | null;
  }): void {
    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'add',
        id: item.id,
        group: 'ai-feedback',
        selection: item.selection,
        metadata: '',
      },
    });
    this.aiFeedbacks = [
      ...this.aiFeedbacks,
      {
        id: item.id,
        startText: item.startText,
        endText: item.endText,
        feedback: item.feedback,
        category: item.category,
      },
    ];
  }

  removeAiFeedback(id: string): void {
    this.enqueue({ type: 'tracked_range', op: { type: 'remove', id } });
    this.aiFeedbacks = this.aiFeedbacks.filter((f) => f.id !== id);
    if (this.activeAiFeedbackId === id) {
      this.activeAiFeedbackId = null;
    }
  }

  clearAiFeedbacks(): void {
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'ai-feedback' } });
    this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group: 'ai-feedback-active' } });
    this.aiFeedbacks = [];
    this.activeAiFeedbackId = null;
  }

  installCommentDecorations(): void {
    if (this.#commentDecorationsInstalled) return;
    this.#commentDecorationsInstalled = true;

    const underline = { color: 'text.amber', style: 'solid' as const, thickness: 2 };

    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'comment',
        style: {
          background: 'ui.comment-highlight',
          background_radius: 2,
          background_inset: 2,
          underline,
        },
        enabled: true,
        z_index: 0,
      },
    });

    this.enqueue({
      type: 'tracked_range',
      op: {
        type: 'set_group_decoration',
        group: 'comment-active',
        style: {
          background: 'ui.comment-highlight-active',
          background_radius: 2,
          background_inset: 2,
          underline,
        },
        enabled: true,
        z_index: 1,
      },
    });
  }

  addFrozenComment(id: string, selection: StableSelection): void {
    if (this.#registeredCommentIds.has(id)) return;
    this.#registeredCommentIds.add(id);
    this.enqueue({ type: 'tracked_range', op: { type: 'add_frozen', id, group: 'comment', selection, metadata: '' } });
  }

  setCommentComposeRange(selection: StableSelection | null): void {
    const id = '__comment_compose__';
    this.enqueue({ type: 'tracked_range', op: { type: 'remove', id } });
    if (selection) {
      this.enqueue({ type: 'tracked_range', op: { type: 'add_frozen', id, group: 'comment', selection, metadata: '' } });
    }
  }

  removeComment(id: string): void {
    if (!this.#registeredCommentIds.has(id)) return;
    this.#registeredCommentIds.delete(id);
    this.enqueue({ type: 'tracked_range', op: { type: 'remove', id } });
    if (this.activeCommentId === id) this.activeCommentId = null;
  }

  hasComment(id: string): boolean {
    return this.#registeredCommentIds.has(id);
  }

  registeredCommentIds(): string[] {
    return [...this.#registeredCommentIds];
  }

  isCommentLocatable(id: string): boolean {
    return this.trackedRanges.some((x) => x.id === id);
  }

  commentIdsAt(page: number, x: number, y: number): string[] {
    return this.#wasm
      .tracked_ranges_at(page, x, y, null)
      .filter((hit) => this.#registeredCommentIds.has(hit.id))
      .map((hit) => hit.id);
  }

  setActiveComment(id: string | null): void {
    if (this.activeCommentId === id) return;

    const move = (cid: string, group: 'comment' | 'comment-active'): boolean => {
      if (this.trackedRanges.every((r) => r.id !== cid)) return false;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: cid, group } });
      return true;
    };

    if (this.activeCommentId !== null) move(this.activeCommentId, 'comment');
    this.activeCommentId = id;
    if (id !== null) {
      const ok = move(id, 'comment-active');
      if (ok) {
        this.scrollIntoView({ target: { type: 'tracked_item', id } });
      } else {
        this.activeCommentId = null;
      }
    }
  }

  setActiveAiFeedback(id: string | null): void {
    if (this.activeAiFeedbackId === id) return;

    const restoreToNormalGroup = (feedbackId: string) => {
      if (this.trackedRanges.every((r) => r.id !== feedbackId)) return;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: feedbackId, group: 'ai-feedback' } });
    };

    const promoteToActiveGroup = (feedbackId: string): boolean => {
      if (this.trackedRanges.every((r) => r.id !== feedbackId)) return false;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: feedbackId, group: 'ai-feedback-active' } });
      return true;
    };

    if (this.activeAiFeedbackId !== null) {
      restoreToNormalGroup(this.activeAiFeedbackId);
    }

    this.activeAiFeedbackId = id;

    if (id !== null) {
      const ok = promoteToActiveGroup(id);
      if (!ok) {
        this.activeAiFeedbackId = null;
        return;
      }
      this.scrollIntoView({ target: { type: 'tracked_item', id } });
    }
  }

  inspect(mode: 'state' | 'state-with-node-id' | 'state-as-macro') {
    const output = match(mode)
      .with('state', () => this.#wasm.inspect_state())
      .with('state-with-node-id', () => this.#wasm.inspect_state({ show_node_ids: true }))
      .with('state-as-macro', () => this.#wasm.inspect_state_as_macro())
      .exhaustive();

    console.log(output);
  }

  destroy(): void {
    this.#destroyed = true;

    unregister(this);

    this.#effectCleanup?.();
    this.#effectCleanup = null;

    if (this.#rafId !== null) {
      cancelAnimationFrame(this.#rafId);
      this.#rafId = null;
    }

    if (this.#characterCountsDebounceTimer) {
      clearTimeout(this.#characterCountsDebounceTimer);
      this.#characterCountsDebounceTimer = null;
    }

    this.#applyViewportResize.cancel();

    this.#gesture?.destroy();
    this.#wasm?.free();
  }
}
