import * as Sentry from '@sentry/sveltekit';
import { debounce } from '@typie/ui/utils';
import { createContext, tick, untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { match } from 'ts-pattern';
import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
import { EditorAttachmentImporter } from './attachment-importer';
import { IS_MAC } from './constants';
import { EditorRequest, EditorUpdate } from './editor-update';
import { fontDataMissingHandler } from './fonts';
import { isSelectionCollapsed, presentedPageElement } from './geometry';
import { TouchGestureController } from './gesture.svelte';
import { readClipboardRich, writeClipboardPayload } from './handlers/clipboard';
import { encodeLengthPrefixedBlobs } from './length-prefix';
import { isMutatingMessage } from './message-gate';
import { proofSatisfies, satisfiesWaiter } from './publication';
import { fanOutResourceUpdate, register, snapshot, unregister } from './registry';
import { probeEvent, probeRendered } from './surface-probe';
import { selectTrackedRangeMember, semanticMembershipForStateChange, trackedRangeMembershipIds } from './tracked-range-membership';
import { zoomDiffers } from './zoom';
import type {
  BlockState,
  CapturedViewportAnchor,
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
  Modifier,
  ModifierState,
  ModifierType,
  PlaceholderMetrics,
  PlainDoc,
  PlainRootNode,
  PointerStyle,
  Position,
  ResourceUpdate,
  Selection,
  SelectionEndpoints,
  Size,
  StableSelection,
  StateField,
  TableOverlay,
  ThemeVariant,
  TickResult,
  TrackedRange,
  TrackedRangeEndpoints,
  Viewport,
  ViewportAnchor,
  ViewportAnchorPoint,
  ViewportAnchorResolution,
} from '@typie/editor-ffi/browser';
import type { ScrollViewport } from '@typie/ui/utils';
import type { EditorPublicationResult } from './editor-update';
import type { FrameProof } from './publication';
import type { EditorScrollBehavior, EditorScrollIntoViewOptions, EditorScrollScope } from './scroll.svelte';
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

type PageSnapshot = Readonly<{
  externalElements: ExternalElement[];
  tableOverlays: TableOverlay[];
  linkRects: LinkRect[];
}>;

export type EditorSnapshot = Readonly<{
  revision: number;
  cursor: CursorMetrics | undefined;
  placeholder: PlaceholderMetrics | undefined;
  selection: Selection | undefined;
  selectionEndpoints: SelectionEndpoints | undefined;
  lastHistoryTag: HistoryTag | undefined;
  pageSizes: Size[];
  pageBackingSizes: Size[];
  externalElements: ExternalElement[];
  tableOverlays: TableOverlay[];
  linkRects: LinkRect[];
  pageData: ReadonlyMap<number, PageSnapshot>;
  rootAttrs: PlainRootNode | undefined;
  rootModifiers: Modifier[];
  trackedRanges: TrackedRange[];
}>;

type ToolbarState = Readonly<{
  modifierState: ModifierState | undefined;
  blockState: BlockState | undefined;
}>;

type PublishedFrame = Readonly<{
  revision: number;
  surfaceKey: number;
  frameKey: number;
  canvas: HTMLCanvasElement;
}>;

const HIDDEN_TICK = Symbol('hidden-tick');
const SPELLCHECK_MEMBERSHIP_GROUPS = new Set(['spellcheck', 'spellcheck-active']);
const AI_FEEDBACK_MEMBERSHIP_GROUPS = new Set(['ai-feedback', 'ai-feedback-active']);
const COMMENT_MEMBERSHIP_GROUPS = new Set(['comment', 'comment-active']);
const sameIds = (a: readonly string[] | null, b: readonly string[]): boolean =>
  a !== null && a.length === b.length && a.every((id, index) => id === b[index]);

export type PublishedBundle = Readonly<{
  snapshot: EditorSnapshot;
  frames: ReadonlyMap<number, PublishedFrame>;
}>;

export type { EditorPublicationResult } from './editor-update';

type UpdateReceipt = {
  resolve: (update: EditorUpdate) => void;
  reject: (error: unknown) => void;
  request: EditorRequest;
};

type TryEnqueueResult = { type: 'enqueued' } | { type: 'ignored' } | { type: 'failed'; error: unknown };

type PublicationWaiter = {
  revision: number;
  requireFrame: boolean;
  resolve: (result: EditorPublicationResult) => void;
  reject: (error: unknown) => void;
  signal: AbortSignal | undefined;
  onAbort: (() => void) | undefined;
};

type AppliedWaiter = {
  resolve: (revision: number) => void;
  reject: (error: unknown) => void;
};

type SurfaceTarget = {
  page: number;
  key: number;
  canvas: HTMLCanvasElement;
  width: number;
  height: number;
  scaleFactor: number;
  requiredRevision: number | undefined;
  proof: FrameProof | undefined;
  failedRevision: number | undefined;
  available: boolean;
  replace: (() => void) | undefined;
};

type VisualHost = {
  token: object;
  targets: Map<number, SurfaceTarget>;
  onPublicationFailure: (revision: number) => void;
};

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

function isContinuousLayout(rootAttrs: PlainRootNode | undefined): boolean {
  return rootAttrs?.layout_mode.type === 'continuous';
}

export function browserScaleFactor(): number {
  if (typeof window === 'undefined') {
    return 1;
  }

  const scaleFactor = window.devicePixelRatio * (window.visualViewport?.scale ?? 1);
  return Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
}

export class EditorContext {
  readonly attachmentImporter = new EditorAttachmentImporter(this);
  editor = $state<Editor>();
  scroll = $state<EditorScrollScope>();
  liveEditor = $state<Editor>();
  fileAssets = $state(new SvelteMap<string, FileAsset>());
  // v1 chrome 호환 필드 — v2 sync 환경에선 갱신되지 않음
  user = $state<unknown>();
  paneFocused = $state(false);
  editorAreaEl = $state<HTMLDivElement>();
  documentId = $state<string | null>(null);
  serverSnapshot = $state<Uint8Array | undefined>();
  serverVersion = $state<string | null>(null);
  serverGeneration = $state<number>(0);
  resetKey = $state<number>(0);
  attachmentDropTargetNodeId = $state<string | null>(null);
  findReplaceOpen = $state(false);
  linkEditorOpen = $state(false);
}

const [getEditorContext, setEditorContext] = createContext<EditorContext>();

export { getEditorContext };
export const setupEditorContext = () => setEditorContext(new EditorContext());

let recoveryRegistered = false;

function registerSurfaceRecovery() {
  if (recoveryRegistered) return;
  recoveryRegistered = true;
  const recoverAll = (source: string) => {
    probeEvent(`recover-all scheduled source=${source}`);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        probeEvent(`recover-all applied source=${source} editors=${snapshot().length}`);
        for (const editor of snapshot()) {
          editor.recoverSurfaces();
        }
      });
    });
  };
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') recoverAll('visibilitychange');
  });
  window.addEventListener('pageshow', () => recoverAll('pageshow'));
}

export class Editor {
  static async create(graph: Uint8Array, viewport: Viewport, themeVariant: ThemeVariant = 'light-white') {
    await ensureWasmInitialized();
    fanOutResourceUpdate(wasm.set_theme_variant(themeVariant));

    const self = new this();
    try {
      self.#installCore(wasm.create_editor_from_graph(graph, viewport));
      self.#initInstance(viewport);
      self.#initialize(true);
      self.#finishCreation();
      return self;
    } catch (err) {
      self.#abortCreation();
      throw err;
    }
  }

  static async createFromDoc(plain: PlainDoc, viewport: Viewport, themeVariant: ThemeVariant = 'light-white'): Promise<Editor> {
    await ensureWasmInitialized();
    fanOutResourceUpdate(wasm.set_theme_variant(themeVariant));

    const self = new this();
    try {
      self.#installCore(wasm.create_editor_from_doc(plain, viewport));
      self.#initInstance(viewport);
      self.#initialize(false);
      self.#finishCreation();
      return self;
    } catch (err) {
      self.#abortCreation();
      throw err;
    }
  }

  static async createWithPending(
    server: Uint8Array,
    pending: Uint8Array[],
    viewport: Viewport,
    themeVariant: ThemeVariant = 'light-white',
  ): Promise<Editor> {
    await ensureWasmInitialized();
    fanOutResourceUpdate(wasm.set_theme_variant(themeVariant));

    const self = new this();
    try {
      self.#installCore(wasm.create_editor_from_graph_with_pending(server, encodeLengthPrefixedBlobs(pending), viewport));
      self.#initInstance(viewport);
      self.#initialize(true);
      self.#finishCreation();
      return self;
    } catch (err) {
      self.#abortCreation();
      throw err;
    }
  }

  static setThemeVariant(variant: ThemeVariant): void {
    fanOutResourceUpdate(wasm.set_theme_variant(variant));
  }

  #wasm!: WasmEditor;
  #destroyed = false;
  #creationActive = true;
  #coreAvailable = false;
  #failed = $state(false);
  #failure: unknown;

  #frameId: number | typeof HIDDEN_TICK | null = null;
  #tickScheduled = false;
  #transitionActive = false;
  #admission: EditorRequest | undefined;
  #surfaceKey = 0;
  #visualHost: VisualHost | undefined;
  #publishedHostToken: object | undefined;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #receipts = new Map<number, UpdateReceipt>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #appliedWaiters = new Set<AppliedWaiter>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #publicationWaiters = new Set<PublicationWaiter>();
  #freshRanges: TrackedRange[] = [];
  #freshRangesFor: EditorSnapshot | undefined;

  #applied = $state.raw<EditorSnapshot>({
    revision: 0,
    cursor: undefined,
    placeholder: undefined,
    selection: undefined,
    selectionEndpoints: undefined,
    lastHistoryTag: undefined,
    pageSizes: [],
    pageBackingSizes: [],
    externalElements: [],
    tableOverlays: [],
    linkRects: [],
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    pageData: new Map(),
    rootAttrs: undefined,
    rootModifiers: [],
    trackedRanges: [],
  });
  // Snapshot trackedRanges preserve semantic registry changes, while their rects can move on
  // unrelated document edits. Resolve only active consumers and retain their revision geometry.
  #resolvedTrackedRanges = new WeakMap<EditorSnapshot, Map<string, TrackedRange | undefined>>();

  #viewport = $state<Viewport>({ width: 0, height: 0, scale_factor: 1 });
  #appliedViewport: Viewport = { width: 0, height: 0, scale_factor: 1 };
  #applyViewportResize = debounce(() => {
    if (this.#destroyed || sameViewport(this.#appliedViewport, this.#viewport)) return;

    this.#applyViewport(this.#viewport);
  }, VIEWPORT_RESIZE_DEBOUNCE_MS);

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #listeners = new Map<EditorEvent['type'], Set<EditorEventListener<EditorEvent['type']>>>();

  // While a pointer selection drag is active, the toolbar-state queries
  // (modifier_state/block_state — expensive on range selections) are
  // deferred: the toolbar only needs the final selection, so we pull once on
  // release instead of every drag frame.
  #toolbarState = $state.raw<ToolbarState>();
  #toolbarSyncSuspended = false;
  #toolbarSyncDirty = false;
  #focused = $state(false);
  #nativeDragAdmissionRetainsFocus = false;
  #effectCleanup: (() => void) | null = null;
  #scrollIntoView: ((options: EditorScrollIntoViewOptions, request: EditorRequest | undefined) => Promise<void> | undefined) | null = null;
  #viewportScrollObserver: (() => void) | null = null;

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
  #spellcheckMembershipIds: string[] | null = null;

  #aiFeedbackDecorationsInstalled = false;
  #aiFeedbackMembershipIds: string[] | null = null;

  #commentDecorationsInstalled = false;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #registeredCommentIds = new Set<string>();

  #characterCountsDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  #runFrame = (): void => {
    this.#frameId = null;
    if (this.terminal || !this.#tickScheduled) return;

    this.#tickScheduled = false;
    let publicEvents: EditorEvent[] = [];
    try {
      this.#transitionActive = true;
      const result = this.#invokeCore((core) => core.tick());
      if (result) publicEvents = this.#installTick(result);
    } catch (err) {
      this.fail(err);
    } finally {
      this.#transitionActive = false;
    }

    for (const event of publicEvents) this.#emit(event);
    if (!this.terminal && this.#tickScheduled) this.#requestFrame();
  };

  published = $state.raw<PublishedBundle>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- replaced atomically; the Set itself is never mutated
  surfacePageRequirements = $state.raw<ReadonlySet<number>>(new Set());
  publicationVersion = $state(0);
  documentRevision = $state(0);
  appliedImeRevision = $state(0);
  appliedSelectionRevision = $state(0);

  inputEl = $state<HTMLTextAreaElement>();
  pageEls = $state<Record<number, HTMLDivElement | undefined>>({});
  extensionAreaEl = $state<HTMLDivElement>();
  scrollContainerEl = $state<HTMLDivElement>();
  scrollViewport = $state<ScrollViewport>();
  scrollRootEl = $state<HTMLElement | null>();
  displayZoom = $state(1);
  renderZoom = $state(1);

  readOnly = $state(false);
  protectContent = $state(false);
  editBlockedHandler: (() => void) | null = null;

  imageAssets = $state(new SvelteMap<string, ImageAsset>());
  inflightImages = $state(new SvelteMap<string, { uploadId: string; url?: string; width: number; height: number }>());

  contextMenu = $state({
    isOpen: false,
    source: 'mouse' as ContextMenuSource,
    x: 0,
    y: 0,
    placement: 'bottom-start' as ContextMenuPlacement,
    extraItems: [] as ContextMenuItem[],
  });

  inflightFiles = $state(new SvelteMap<string, { uploadId: string; name: string; size: number }>());

  spellcheckErrors = $state<SpellcheckError[]>([]);
  activeSpellcheckErrorId = $state<string | null>(null);

  aiFeedbacks = $state<AiFeedback[]>([]);
  activeAiFeedbackId = $state<string | null>(null);

  activeCommentId = $state<string | null>(null);
  commentClickHandler: ((id: string) => void) | null = null;
  requestCommentCompose: (() => void) | null = null;

  embedAssets = $state(new SvelteMap<string, EmbedAsset>());
  inflightEmbeds = $state(new SvelteMap<string, { uploadId: string; url: string }>());
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
    registerSurfaceRecovery();
  }

  #installCore(core: WasmEditor): void {
    this.#wasm = core;
    this.#coreAvailable = true;
  }

  #finishCreation(): void {
    this.#creationActive = false;
    register(this);
  }

  #abortCreation(): void {
    unregister(this);
    this.#stopRuntime();
    this.#discardCore();
  }

  #invokeCore<T>(operation: (core: WasmEditor) => T): T {
    if (this.#destroyed) throw new Error('Editor is disposed');
    if (this.#failed) throw this.#failure;
    if (!this.#coreAvailable) throw new Error('Editor core is unavailable');

    try {
      return operation(this.#wasm);
    } catch (err) {
      if (!this.#creationActive) this.fail(err);
      throw err;
    }
  }

  #tryEnqueue(message: Message): TryEnqueueResult {
    if (this.terminal) return { type: 'ignored' };
    if (this.#admission) {
      this.#admission.enqueue(message);
      return { type: 'enqueued' };
    }
    if (this.readOnly && isMutatingMessage(message)) {
      this.editBlockedHandler?.();
      return { type: 'ignored' };
    }
    const result = this.#invokeCore((core): TryEnqueueResult => {
      try {
        core.enqueue_request([message]);
        return { type: 'enqueued' };
      } catch (err) {
        return { type: 'failed', error: err };
      }
    });
    if (result.type === 'enqueued') this.#requestWasmTick();
    return result;
  }

  #reportError(error: unknown, message: string): void {
    console.error(message, error);
    Sentry.captureException(error);
  }

  #discardCore(): void {
    if (!this.#coreAvailable) return;
    const core = this.#wasm;
    this.#coreAvailable = false;
    try {
      // Disposal must not reenter the guarded core boundary.
      core.free();
    } catch {
      // The wrapper is discarded even when its destructor reports an error.
    }
  }

  #stopRuntime(): void {
    this.#clearScheduledTick();
    if (this.#characterCountsDebounceTimer !== null) {
      clearTimeout(this.#characterCountsDebounceTimer);
      this.#characterCountsDebounceTimer = null;
    }
    this.#applyViewportResize.cancel();
    this.#effectCleanup?.();
    this.#effectCleanup = null;
    this.#gesture?.destroy();
  }

  #clearScheduledTick(): void {
    this.#tickScheduled = false;
    if (this.#frameId !== null) {
      if (typeof this.#frameId === 'number') cancelAnimationFrame(this.#frameId);
      this.#frameId = null;
    }
  }

  #materializeSnapshot(revision: number, fields: ReadonlySet<StateField>): EditorSnapshot {
    return this.#invokeCore((core) => {
      const previous = this.#applied;
      const pageGeometryDataChanged =
        fields.has('doc') || fields.has('external_elements') || fields.has('table_overlays') || fields.has('link_rects');
      const rootAttrs = fields.has('root_attrs') ? core.root_attrs() : previous.rootAttrs;
      const continuous = isContinuousLayout(rootAttrs);
      let pageData = previous.pageData;

      if (pageGeometryDataChanged) {
        // eslint-disable-next-line svelte/prefer-svelte-reactivity
        const nextPageData = new Map(previous.pageData);
        for (const page of this.#visualHost?.targets.keys() ?? []) {
          nextPageData.set(page, {
            externalElements: core.page_external_elements(page),
            tableOverlays: continuous ? [] : core.page_table_overlays(page),
            linkRects: core.page_link_rects(page),
          });
        }
        pageData = nextPageData;
      }

      return {
        revision,
        cursor: fields.has('cursor') ? core.cursor() : previous.cursor,
        placeholder: fields.has('placeholder') ? (core.placeholder() ?? undefined) : previous.placeholder,
        selection: fields.has('selection') || fields.has('doc') ? core.selection() : previous.selection,
        selectionEndpoints:
          fields.has('selection') || fields.has('cursor') || fields.has('doc') || fields.has('page_sizes')
            ? core.selection_endpoints()
            : previous.selectionEndpoints,
        lastHistoryTag: fields.has('last_history_tag') ? core.last_history_tag() : previous.lastHistoryTag,
        pageSizes: fields.has('page_sizes') ? core.page_sizes() : previous.pageSizes,
        pageBackingSizes: fields.has('page_sizes') ? core.page_backing_sizes() : previous.pageBackingSizes,
        externalElements: fields.has('external_elements') ? core.external_elements() : previous.externalElements,
        tableOverlays: pageGeometryDataChanged
          ? continuous
            ? core.table_overlays()
            : [...pageData.values()].flatMap((page) => page.tableOverlays)
          : previous.tableOverlays,
        linkRects: pageGeometryDataChanged ? [...pageData.values()].flatMap((page) => page.linkRects) : previous.linkRects,
        pageData,
        rootAttrs,
        rootModifiers: fields.has('block') || fields.has('modifiers') ? core.root_modifiers() : previous.rootModifiers,
        trackedRanges: fields.has('tracked_ranges') ? core.tracked_ranges() : previous.trackedRanges,
      };
    });
  }

  #installAppliedSideEffects(fields: ReadonlySet<StateField>): void {
    const snapshot = this.#applied;

    if (fields.has('selection') && snapshot.selection === undefined) {
      this.inputEl?.blur();
      this.closeContextMenu();
    }

    if (fields.has('doc') || fields.has('selection')) this.characterCountsVersion++;
    if (fields.has('ime')) this.appliedImeRevision++;
    if (fields.has('selection')) this.appliedSelectionRevision++;
    if (fields.has('doc')) this.documentRevision++;

    if (fields.has('tracked_ranges')) {
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const rangeIds = new Set(snapshot.trackedRanges.map((range) => range.id));
      this.spellcheckErrors = this.spellcheckErrors.filter((error) => rangeIds.has(error.id));
      if (this.activeSpellcheckErrorId !== null && !rangeIds.has(this.activeSpellcheckErrorId)) {
        this.activeSpellcheckErrorId = null;
      }
      this.aiFeedbacks = this.aiFeedbacks.filter((feedback) => rangeIds.has(feedback.id));
      if (this.activeAiFeedbackId !== null && !rangeIds.has(this.activeAiFeedbackId)) this.activeAiFeedbackId = null;
    }

    const hasMembershipConsumers = this.spellcheckErrors.length > 0 || this.aiFeedbacks.length > 0;
    if (!hasMembershipConsumers || !snapshot.selection) {
      this.#spellcheckMembershipIds = null;
      this.#aiFeedbackMembershipIds = null;
    }

    if (hasMembershipConsumers) {
      const membership = semanticMembershipForStateChange(fields, snapshot.selection, (selection) =>
        this.#invokeCore((core) => core.tracked_ranges_containing_selection(selection, null)),
      );
      if (membership !== undefined) {
        this.#syncActiveSpellcheckErrorFromMembership(membership);
        this.#syncActiveAiFeedbackFromMembership(membership);
      }
    }

    if (fields.has('root_attrs') || fields.has('page_sizes')) {
      this.refreshPointerStyleAfterDomUpdate();
    } else if (['doc', 'external_elements', 'modifiers', 'block'].some((field) => fields.has(field as StateField))) {
      this.refreshPointerStyle();
    }
  }

  #installTick(result: TickResult): EditorEvent[] {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const fields = new Set<StateField>();
    let renderInvalidated = false;
    for (const event of result.events) {
      if (event.type === 'state_changed') {
        for (const field of event.fields) fields.add(field);
      } else if (event.type === 'render_invalidated') {
        renderInvalidated = true;
      }
    }

    if (fields.has('modifiers') || fields.has('block')) this.#toolbarSyncDirty = true;
    this.#applied = this.#materializeSnapshot(result.revision.value, fields);
    const publicEvents = result.events.filter((event) => event.type !== 'state_changed' && event.type !== 'render_invalidated');

    const completedReceiptIds: number[] = [];
    const completedReceipts: { receipt: UpdateReceipt; update: EditorUpdate }[] = [];
    for (const outcome of result.request_outcomes) {
      const receipt = this.#receipts.get(outcome.request_id.value);
      if (!receipt) continue;
      const update = new EditorUpdate(result.revision.value, this.#applied, outcome.command_outcomes, publicEvents, (signal) =>
        this.#awaitPublished(result.revision.value, signal),
      );
      receipt.request.runBeforePublish(update);
      completedReceiptIds.push(outcome.request_id.value);
      completedReceipts.push({ receipt, update });
    }

    const replaceChangedTargets = fields.has('page_sizes') ? this.#reconcileSurfaceTargets() : [];
    this.#installAppliedSideEffects(fields);

    if (renderInvalidated) {
      for (const target of this.#visualHost?.targets.values() ?? []) {
        target.requiredRevision = result.revision.value;
      }
    }

    for (const replace of replaceChangedTargets) replace();
    this.#renderRequiredTargets();
    this.#publicationChanged();
    if ([...(this.#visualHost?.targets.values() ?? [])].some((target) => !target.available)) {
      this.#rejectAllPublicationWaitersForSurface();
    }

    for (const waiter of this.#appliedWaiters) waiter.resolve(result.revision.value);
    this.#appliedWaiters.clear();
    for (const requestId of completedReceiptIds) this.#receipts.delete(requestId);
    for (const { receipt, update } of completedReceipts) {
      receipt.resolve(update);
    }
    return publicEvents;
  }

  #reconcileSurfaceTargets(): (() => void)[] {
    const host = this.#visualHost;
    if (!host) return [];
    const replaceChangedTargets: (() => void)[] = [];

    for (const [page, target] of host.targets) {
      const pageSize = this.#applied.pageSizes[page];
      if (!pageSize) {
        this.#invokeCore((core) => core.detach_surface(page));
        host.targets.delete(page);
        this.#removeAppliedPageData([page]);
        continue;
      }

      const width = pageSize.width;
      const height = this.#applied.pageBackingSizes[page]?.height ?? pageSize.height;
      const scaleFactor = this.surfaceScaleFactor;
      if (target.width === width && target.height === height && target.scaleFactor === scaleFactor) continue;

      target.proof = undefined;
      target.requiredRevision = this.#applied.revision;
      target.failedRevision = undefined;
      target.available = false;
      if (target.replace) replaceChangedTargets.push(target.replace);
    }
    return replaceChangedTargets;
  }

  #renderRequiredTargets(): void {
    const host = this.#visualHost;
    if (!host) return;

    for (const target of host.targets.values()) {
      if (!target.available || target.requiredRevision === undefined || proofSatisfies(target)) continue;

      const requestedRevision = this.#applied.revision;
      if ((target.failedRevision ?? -1) >= requestedRevision) continue;
      const surfaceKey = target.key;
      const frameKey = this.#invokeCore((core) => core.render_surface(target.page, { value: requestedRevision }));
      const current = host.targets.get(target.page);
      if (current !== target || current.key !== surfaceKey || !current.available) continue;
      if (frameKey) {
        target.proof = { revision: requestedRevision, surfaceKey, frameKey: frameKey.value };
        probeRendered(this, target.page);
      } else {
        target.failedRevision = requestedRevision;
        this.#rejectPublicationWaitersThrough(requestedRevision);
        host.onPublicationFailure(requestedRevision);
      }
    }
  }

  #publicationChanged(): void {
    this.publicationVersion += 1;
  }

  #completePublicationWaiters(bundle: PublishedBundle): void {
    if (this.published !== bundle) return;
    for (const waiter of this.#publicationWaiters) {
      if (!this.isPublished(waiter.revision, { requireFrame: waiter.requireFrame })) continue;
      this.#publicationWaiters.delete(waiter);
      if (waiter.signal && waiter.onAbort) waiter.signal.removeEventListener('abort', waiter.onAbort);
      waiter.resolve({ type: 'published', revision: bundle.snapshot.revision });
    }
  }

  #pullToolbarStateIfReady(): void {
    if (this.#toolbarSyncSuspended || !this.#toolbarSyncDirty || this.published?.snapshot.revision !== this.#applied.revision) return;

    const state = this.#invokeCore((core) => ({
      modifierState: core.modifier_state(),
      blockState: core.block_state(),
    }));
    this.#toolbarState = state;
    this.#toolbarSyncDirty = false;
  }

  #awaitPublished(revision: number, signal?: AbortSignal, requireFrame = false): Promise<EditorPublicationResult> {
    if (signal?.aborted) return Promise.reject(signal.reason ?? new DOMException('Cancelled', 'AbortError'));
    if (this.#destroyed) return Promise.reject(new Error('Editor is disposed'));
    if (this.#failed) return Promise.reject(this.#failure);
    if (!this.#visualHost) return Promise.resolve({ type: 'no_host' });
    const published = this.published;
    if (published && this.isPublished(revision, { requireFrame })) {
      return Promise.resolve({ type: 'published', revision: published.snapshot.revision });
    }
    for (const target of this.#visualHost.targets.values()) {
      if ((target.failedRevision ?? -1) >= revision) {
        return Promise.reject(new DOMException('The editor surface failed to render.', 'OperationError'));
      }
    }

    return new Promise((resolve, reject) => {
      const waiter: PublicationWaiter = { revision, requireFrame, resolve, reject, signal, onAbort: undefined };
      if (signal) {
        waiter.onAbort = () => {
          this.#publicationWaiters.delete(waiter);
          reject(signal.reason ?? new DOMException('Cancelled', 'AbortError'));
        };
        signal.addEventListener('abort', waiter.onAbort, { once: true });
      }
      this.#publicationWaiters.add(waiter);
    });
  }

  #rejectPublicationWaitersThrough(revision: number): void {
    for (const waiter of this.#publicationWaiters) {
      if (waiter.revision > revision) continue;
      this.#publicationWaiters.delete(waiter);
      if (waiter.signal && waiter.onAbort) waiter.signal.removeEventListener('abort', waiter.onAbort);
      waiter.reject(new DOMException('The editor surface failed to render.', 'OperationError'));
    }
  }

  #rejectAllPublicationWaitersForSurface(): void {
    for (const waiter of this.#publicationWaiters) {
      if (waiter.signal && waiter.onAbort) waiter.signal.removeEventListener('abort', waiter.onAbort);
      waiter.reject(new DOMException('The editor surface target must be replaced.', 'OperationError'));
    }
    this.#publicationWaiters.clear();
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

  #requestFrame(): void {
    if (this.#frameId !== null) return;
    if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
      this.#frameId = HIDDEN_TICK;
      queueMicrotask(() => {
        if (this.#frameId === HIDDEN_TICK) this.#runFrame();
      });
      return;
    }
    this.#frameId = requestAnimationFrame(this.#runFrame);
  }

  #requestWasmTick(): void {
    if (this.terminal) return;
    this.#tickScheduled = true;
    this.#requestFrame();
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
    const live = preferFresh
      ? undefined
      : this.#invokeCore((core) => core.tracked_ranges('search-match')).find((range) => range.id === match.id);
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
    this.revealTrackedItem(match.id, { behavior: 'instant' });
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

  #syncActiveSpellcheckErrorFromMembership(membership: readonly TrackedRangeEndpoints[]): void {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const ownedIds = new Set(this.spellcheckErrors.map((error) => error.id));
    const membershipIds = trackedRangeMembershipIds(membership, SPELLCHECK_MEMBERSHIP_GROUPS, ownedIds);
    if (sameIds(this.#spellcheckMembershipIds, membershipIds)) return;
    this.#spellcheckMembershipIds = membershipIds;
    const member = selectTrackedRangeMember(membership, SPELLCHECK_MEMBERSHIP_GROUPS, this.activeSpellcheckErrorId, ownedIds);

    if (member) {
      this.setActiveSpellcheckError(member.id);
    } else if (this.activeSpellcheckErrorId !== null) {
      this.setActiveSpellcheckError(null);
    }
  }

  #syncActiveAiFeedbackFromMembership(membership: readonly TrackedRangeEndpoints[]): void {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const ownedIds = new Set(this.aiFeedbacks.map((feedback) => feedback.id));
    const membershipIds = trackedRangeMembershipIds(membership, AI_FEEDBACK_MEMBERSHIP_GROUPS, ownedIds);
    if (sameIds(this.#aiFeedbackMembershipIds, membershipIds)) return;
    this.#aiFeedbackMembershipIds = membershipIds;
    const member = selectTrackedRangeMember(membership, AI_FEEDBACK_MEMBERSHIP_GROUPS, this.activeAiFeedbackId, ownedIds);

    if (member) {
      this.setActiveAiFeedback(member.id);
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

    this.on('font_data_missing', fontDataMissingHandler);
    this.on('tracked_range_replace_result', (_, { id, outcome }) => {
      if (id.startsWith('search-match:')) {
        this.#handleSearchReplaceResult(id, outcome);
      } else if (this.spellcheckErrors.some((e) => e.id === id)) {
        this.#handleSpellcheckReplaceResult(id);
      }
    });
    this.on('tracked_ranges_stale', (_, { ids }) => {
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const stale = new Set(ids);
      this.spellcheckErrors = this.spellcheckErrors.filter((e) => !stale.has(e.id));
      if (this.activeSpellcheckErrorId !== null && stale.has(this.activeSpellcheckErrorId)) {
        this.activeSpellcheckErrorId = null;
      }
    });

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

          // DOM 제거 중 blur가 Svelte 렌더 안에서 $state를 갱신하지 않도록 다음 microtask로 미룬다.
          queueMicrotask(() => {
            if (this.#destroyed) return;
            this.#setFocused(document.activeElement === this.inputEl);
          });
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

  #initialize(installSearchDecorations: boolean): void {
    const update = this.updateNow((request) => {
      request.enqueue({ type: 'system', event: { type: 'initialize' } });
      if (!installSearchDecorations) return;

      request.enqueue({
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
      request.enqueue({
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
    });
    if (!update || update.commandOutcomes.some((outcome) => outcome.type === 'rejected')) {
      throw new Error('Editor initialization was rejected');
    }
  }

  #buildRequest(build: (request: EditorRequest) => void): EditorRequest {
    if (this.#destroyed) throw new Error('Editor is disposed');
    if (this.#failed) throw this.#failure;
    if (this.#admission) throw new Error('Editor update admission cannot be nested');
    const request = new EditorRequest();
    this.#admission = request;
    try {
      build(request);
    } catch (err) {
      request.discardAfterFailure(err);
      throw err;
    } finally {
      this.#admission = undefined;
    }
    if (!this.readOnly) return request;

    if (request.removeMessagesWhere(isMutatingMessage)) this.editBlockedHandler?.();
    return request;
  }

  #refreshAppliedPageData(pages: Iterable<number>): void {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const pageData = new Map(this.#applied.pageData);
    const continuous = isContinuousLayout(this.#applied.rootAttrs);
    let changed = false;
    this.#invokeCore((core) => {
      for (const page of pages) {
        pageData.set(page, {
          externalElements: core.page_external_elements(page),
          tableOverlays: continuous ? [] : core.page_table_overlays(page),
          linkRects: core.page_link_rects(page),
        });
        changed = true;
      }
    });
    if (!changed) return;
    this.#applied = {
      ...this.#applied,
      pageData,
      tableOverlays: this.#tableOverlaysFor(pageData),
      linkRects: [...pageData.values()].flatMap((page) => page.linkRects),
    };
  }

  // Continuous overlays are document-scoped: the engine merges the page fragments of a
  // split table into one overlay per table, so page mounts must not re-derive them from
  // page-local fragments (which repeat the same table_id).
  #tableOverlaysFor(pageData: ReadonlyMap<number, PageSnapshot>): TableOverlay[] {
    if (isContinuousLayout(this.#applied.rootAttrs)) return this.#applied.tableOverlays;
    return [...pageData.values()].flatMap((page) => page.tableOverlays);
  }

  #removeAppliedPageData(pages: Iterable<number>): void {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const pageData = new Map(this.#applied.pageData);
    let changed = false;
    for (const page of pages) changed = pageData.delete(page) || changed;
    if (changed) {
      this.#applied = {
        ...this.#applied,
        pageData,
        tableOverlays: this.#tableOverlaysFor(pageData),
        linkRects: [...pageData.values()].flatMap((page) => page.linkRects),
      };
    }
  }

  get gesture(): TouchGestureController {
    return this.#gesture;
  }

  setDoc(plain: PlainDoc): void {
    if (this.#destroyed) return;
    this.#invokeCore((core) => core.set_doc(plain));
    this.#requestWasmTick();
  }

  materializeAt(heads: Uint8Array, sweepTombstones: string[]): PlainDoc {
    return this.#invokeCore((core) => core.materialize_at(heads, sweepTombstones));
  }

  get cursor() {
    return this.published?.snapshot.cursor;
  }

  get placeholder(): PlaceholderMetrics | undefined {
    return this.published?.snapshot.placeholder;
  }

  get selection() {
    return this.published?.snapshot.selection;
  }

  freezeSelection(selection: Selection): StableSelection | undefined {
    return this.#invokeCore((core) => core.freeze_selection(selection));
  }

  get isSelectionCollapsed(): boolean {
    return isSelectionCollapsed(this.selection);
  }

  get pageSizes() {
    return this.published?.snapshot.pageSizes ?? [];
  }

  get pageBackingSizes() {
    return this.published?.snapshot.pageBackingSizes ?? [];
  }

  get externalElements(): ExternalElement[] {
    return this.published?.snapshot.externalElements ?? [];
  }

  get tableOverlays(): TableOverlay[] {
    return this.published?.snapshot.tableOverlays ?? [];
  }

  get linkRects(): LinkRect[] {
    return this.published?.snapshot.linkRects ?? [];
  }

  pageExternalElements(page: number): ExternalElement[] {
    return this.published?.snapshot.pageData.get(page)?.externalElements ?? [];
  }

  pageTableOverlays(page: number): TableOverlay[] {
    return this.published?.snapshot.pageData.get(page)?.tableOverlays ?? [];
  }

  pageLinkRects(page: number): LinkRect[] {
    return this.published?.snapshot.pageData.get(page)?.linkRects ?? [];
  }

  get rootAttrs() {
    return this.published?.snapshot.rootAttrs;
  }

  get rootModifiers() {
    return this.published?.snapshot.rootModifiers;
  }

  get modifierState() {
    return this.#toolbarState?.modifierState;
  }

  get blockState() {
    return this.#toolbarState?.blockState;
  }

  get lastHistoryTag() {
    return this.published?.snapshot.lastHistoryTag;
  }

  get scaleFactor() {
    return this.#viewport.scale_factor;
  }

  get surfaceScaleFactor() {
    return this.#viewport.scale_factor * this.renderZoom;
  }

  isPublished(revision: number, options?: { requireFrame?: boolean }): boolean {
    const published = this.published;
    const host = this.#visualHost;
    return (
      published !== undefined &&
      host !== undefined &&
      this.#publishedHostToken === host.token &&
      satisfiesWaiter(revision, published.snapshot.revision, published.frames, options?.requireFrame)
    );
  }

  resizeViewport(width: number, height: number, scaleFactor: number): void {
    if (!Number.isFinite(width) || !Number.isFinite(height) || !Number.isFinite(scaleFactor)) return;
    if (width <= 0 || height <= 0 || scaleFactor <= 0) return;

    const viewport = { width, height, scale_factor: scaleFactor };
    if (sameViewport(this.#viewport, viewport)) return;

    this.#viewport = viewport;
    if (sameViewport(this.#appliedViewport, viewport)) return;

    this.#applyViewportResize.call();
  }

  resizeViewportNow(width: number, height: number, scaleFactor: number): void {
    if (!Number.isFinite(width) || !Number.isFinite(height) || !Number.isFinite(scaleFactor)) return;
    if (width <= 0 || height <= 0 || scaleFactor <= 0) return;

    const viewport = { width, height, scale_factor: scaleFactor };
    this.#viewport = viewport;
    this.#applyViewportResize.cancel();
    if (sameViewport(this.#appliedViewport, viewport)) return;

    this.#appliedViewport = viewport;
    this.updateNow((request) => {
      request.enqueue({
        type: 'system',
        event: { type: 'resize', width: viewport.width, height: viewport.height, scale_factor: viewport.scale_factor },
      });
    });
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

    if (restoreFocus && this.selection !== undefined) {
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
    if (this.isSelectionCollapsed) return;
    if (this.readOnly) {
      this.editBlockedHandler?.();
      return;
    }
    const payload = this.copySelection();
    if (!payload) return;
    await writeClipboardPayload(payload.html, payload.text);
    if (this.readOnly || this.terminal) return;
    this.updateNow(() => {
      this.enqueue({ type: 'clipboard', op: { type: 'cut' } });
      this.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });
  }

  async requestPasteTextOnly(): Promise<void> {
    if (this.readOnly) {
      this.editBlockedHandler?.();
      return;
    }
    const result = await readClipboardRich();
    if (!result || result.text === '') return;
    if (this.readOnly || this.terminal) return;
    this.updateNow(() => {
      this.enqueue({ type: 'clipboard', op: { type: 'paste', html: undefined, text: result.text } });
      this.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'typewriter' });
    });
  }

  insertTemplateFragment(changesets: Uint8Array): void {
    if (this.readOnly) {
      this.editBlockedHandler?.();
      return;
    }
    this.#invokeCore((core) => core.insert_template_fragment(changesets));
    this.#requestWasmTick();
  }

  handleRepasteAsText(): void {
    if (this.readOnly) {
      this.editBlockedHandler?.();
      return;
    }
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
    return this.#tickScheduled;
  }

  get destroyed() {
    return this.#destroyed;
  }

  get terminal() {
    return this.#destroyed || this.#failed;
  }

  get failure(): unknown {
    void this.#failed;
    return this.#failure;
  }

  get appliedRevision(): number {
    return this.#applied.revision;
  }

  get appliedSnapshot(): EditorSnapshot {
    return this.#applied;
  }

  trackedRangeForSnapshot(id: string, snapshot: EditorSnapshot): TrackedRange | undefined {
    let resolved = this.#resolvedTrackedRanges.get(snapshot);
    if (resolved?.has(id)) return resolved.get(id);
    if (snapshot === this.#applied) {
      const range = this.#invokeCore((core) => core.tracked_range(id));
      if (!resolved) {
        // eslint-disable-next-line svelte/prefer-svelte-reactivity
        resolved = new Map();
        this.#resolvedTrackedRanges.set(snapshot, resolved);
      }
      resolved.set(id, range);
      return range;
    }
    return snapshot.trackedRanges.find((range) => range.id === id);
  }

  get publishedRevision(): number | undefined {
    return this.published?.snapshot.revision;
  }

  awaitPublishedRevision(revision: number, options?: { requireFrame?: boolean }): Promise<EditorPublicationResult> {
    return this.#awaitPublished(revision, undefined, options?.requireFrame);
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
    const link = this.#invokeCore((core) => core.link_hit_test(local.page, local.x, local.y));
    return link ? { link, page: local.page } : undefined;
  }

  safeDisplayZoom(): number {
    const zoom = this.rootAttrs?.layout_mode.type === 'paginated' ? this.displayZoom : 1;
    return Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
  }

  captureSelectionViewportAnchor(revision: number): CapturedViewportAnchor | undefined {
    return this.#invokeCore((core) => core.capture_selection_viewport_anchor({ value: revision }));
  }

  captureViewportAnchorAt(revision: number, point: ViewportAnchorPoint): CapturedViewportAnchor | undefined {
    return this.#invokeCore((core) => core.capture_viewport_anchor_at({ value: revision }, point));
  }

  resolveViewportAnchor(revision: number, anchor: ViewportAnchor): ViewportAnchorResolution {
    return this.#invokeCore((core) => core.resolve_viewport_anchor({ value: revision }, anchor));
  }

  clientDeltaToLocalDelta(delta: number): number {
    return delta / this.safeDisplayZoom();
  }

  registerScrollIntoView(
    handler: ((options: EditorScrollIntoViewOptions, request: EditorRequest | undefined) => Promise<void> | undefined) | null,
  ): void {
    this.#scrollIntoView = handler;
  }

  scrollIntoView(options: EditorScrollIntoViewOptions): Promise<void> | undefined {
    return this.#scrollIntoView?.(options, this.#admission);
  }

  registerViewportScrollObserver(handler: (() => void) | null): void {
    this.#viewportScrollObserver = handler;
  }

  notifyViewportScrolled(): void {
    this.#viewportScrollObserver?.();
  }

  revealTrackedItem(
    id: string,
    { behavior = 'smooth', minimumHeight }: { behavior?: EditorScrollBehavior; minimumHeight?: number } = {},
  ): Promise<void> | undefined {
    let presentation: Promise<void> | undefined;
    void this.update((request) => {
      request.enqueue({ type: 'view', op: { type: 'expand_folds_for_tracked_range', id } });
      presentation = this.scrollIntoView({ target: { type: 'tracked_item', id, minimumHeight }, policy: 'reveal', behavior });
    }).catch((err: unknown) => this.fail(err));
    return presentation;
  }

  clientToLocal(clientX: number, clientY: number) {
    const pages = this.pageSizes;
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

    const el = presentedPageElement(this, lo);
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
    return this.#invokeCore((core) => core.interactive_hit_test(page, x, y));
  }

  selectionEndpoints(): SelectionEndpoints | undefined {
    return this.published?.snapshot.selectionEndpoints;
  }

  modifierSpanSelection(pos: Position, modifierType: ModifierType): Selection | undefined {
    return this.#invokeCore((core) => core.modifier_span_selection(pos, modifierType));
  }

  ime(beforeLimit: number, afterLimit: number): Ime | undefined {
    return this.#invokeCore((core) => core.ime(beforeLimit, afterLimit));
  }

  selectionHitTest(page: number, x: number, y: number): boolean {
    return this.#invokeCore((core) => core.selection_hit_test(page, x, y));
  }

  cursorHitTest(page: number, x: number, y: number): boolean {
    return this.#invokeCore((core) => core.cursor_hit_test(page, x, y));
  }

  updatePointerHover(clientX: number, clientY: number): void {
    this.#lastPointerClient = { x: clientX, y: clientY };
    this.refreshPointerStyle();
  }

  refreshPointerStyle(): void {
    if (this.terminal) {
      return;
    }

    const pointer = this.#lastPointerClient;
    if (!pointer) {
      return;
    }

    const local = this.clientToLocal(pointer.x, pointer.y);
    if (local) {
      const { pointerStyle, link } = this.#invokeCore((core) => ({
        pointerStyle: core.pointer_style(local.page, local.x, local.y, this.readOnly),
        link: core.link_hit_test(local.page, local.x, local.y),
      }));
      this.#pointerStyle = pointerStyle;
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
    if (this.terminal || !this.#lastPointerClient || this.#pointerStyleDomRefreshQueued) {
      return;
    }

    this.#pointerStyleDomRefreshQueued = true;
    void tick().then(() => {
      this.#pointerStyleDomRefreshQueued = false;
      if (!this.terminal) {
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

  enqueue(message: Message): void {
    const result = this.#tryEnqueue(message);
    if (result.type !== 'failed') return;
    if (!this.#creationActive) this.fail(result.error);
    throw result.error;
  }

  async update(build: (request: EditorRequest) => void): Promise<EditorUpdate | null> {
    if (this.terminal) return null;

    const request = this.#buildRequest(build);
    if (request.empty) {
      request.discard();
      return null;
    }

    const requestId = (() => {
      try {
        return this.#invokeCore((core) => core.enqueue_request([...request.messages]));
      } catch (err) {
        request.discardAfterFailure(err);
        throw err;
      }
    })();
    const promise = new Promise<EditorUpdate>((resolve, reject) => {
      this.#receipts.set(requestId.value, { resolve, reject, request });
    });
    this.#requestWasmTick();
    return promise;
  }

  updateNow(build: (request: EditorRequest) => void): EditorUpdate | null {
    if (this.#transitionActive) throw new Error('updateNow cannot run reentrant-ly');
    if (this.terminal) return null;

    let publicEvents: EditorEvent[];
    let update: EditorUpdate | undefined;
    let receiptFailure: { error: unknown } | undefined;
    let admitted = false;
    let request: EditorRequest | undefined;
    this.#transitionActive = true;
    try {
      const builtRequest = this.#buildRequest(build);
      request = builtRequest;
      if (builtRequest.empty) {
        builtRequest.discard();
        return null;
      }

      const requestId = this.#invokeCore((core) => core.enqueue_request([...builtRequest.messages]));
      admitted = true;
      this.#receipts.set(requestId.value, {
        request: builtRequest,
        resolve: (value) => {
          update = value;
        },
        reject: (error) => {
          receiptFailure = { error };
        },
      });
      const result = this.#invokeCore((core) => core.tick_through(requestId));
      // tickThrough has consumed every entry through this synchronously admitted request.
      // Retire an older queued RAF so Host work does not wait for a tick that is already installed.
      this.#clearScheduledTick();
      publicEvents = this.#installTick(result);
      if (receiptFailure) throw receiptFailure.error;
      if (!update) throw new Error(`tickThrough omitted request outcome ${requestId.value}`);
    } catch (err) {
      request?.discardAfterFailure(err);
      if (admitted && !this.#creationActive) this.fail(err);
      throw err;
    } finally {
      this.#transitionActive = false;
    }

    for (const event of publicEvents) this.#emit(event);
    return update;
  }

  suspendToolbarSync(): void {
    this.#toolbarSyncSuspended = true;
  }

  resumeToolbarSync(): void {
    if (this.terminal) return;
    this.#toolbarSyncSuspended = false;
    this.#pullToolbarStateIfReady();
  }

  requestSurfacePages(pages: ReadonlySet<number>): void {
    if (this.terminal) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable value installed into raw reactive state
    const valid = new Set([...pages].filter((page) => page >= 0 && page < this.#applied.pageSizes.length));
    if (valid.size === this.surfacePageRequirements.size && [...valid].every((page) => this.surfacePageRequirements.has(page))) {
      return;
    }
    const previous = this.surfacePageRequirements;
    this.#refreshAppliedPageData([...valid].filter((page) => !previous.has(page)));
    this.#removeAppliedPageData([...previous].filter((page) => !valid.has(page)));
    this.surfacePageRequirements = valid;
    this.#publicationChanged();
  }

  requestPublication(): void {
    if (!this.terminal) this.publicationVersion += 1;
  }

  get activeSurfacePages(): ReadonlySet<number> {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable caller snapshot
    return new Set(this.#visualHost?.targets.keys());
  }

  publishIfReady(requiredPages: ReadonlySet<number>): PublishedBundle | undefined {
    const host = this.#visualHost;
    if (
      !host ||
      requiredPages.size !== this.surfacePageRequirements.size ||
      [...requiredPages].some((page) => !this.surfacePageRequirements.has(page))
    ) {
      return undefined;
    }
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable bundle snapshot
    const frames = new Map<number, PublishedFrame>();
    for (const page of requiredPages) {
      const target = host.targets.get(page);
      if (!target || !proofSatisfies(target) || !target.proof) return undefined;
      frames.set(page, {
        revision: target.proof.revision,
        surfaceKey: target.key,
        frameKey: target.proof.frameKey,
        canvas: target.canvas,
      });
    }
    const current = this.published;
    if (
      current?.snapshot === this.#applied &&
      current.frames.size === frames.size &&
      [...frames].every(([page, frame]) => {
        const previous = current.frames.get(page);
        return (
          previous?.canvas === frame.canvas &&
          previous.revision === frame.revision &&
          previous.surfaceKey === frame.surfaceKey &&
          previous.frameKey === frame.frameKey
        );
      })
    ) {
      return current;
    }
    return { snapshot: this.#applied, frames };
  }

  acceptPublication(bundle: PublishedBundle): boolean {
    const host = this.#visualHost;
    if (this.published === bundle && this.#publishedHostToken === host?.token) return true;
    if (
      !host ||
      this.#applied !== bundle.snapshot ||
      bundle.frames.size !== this.surfacePageRequirements.size ||
      [...bundle.frames.keys()].some((page) => !this.surfacePageRequirements.has(page))
    ) {
      return false;
    }
    const targets: SurfaceTarget[] = [];
    for (const [page, frame] of bundle.frames) {
      const target = host.targets.get(page);
      if (
        !target ||
        target.canvas !== frame.canvas ||
        target.key !== frame.surfaceKey ||
        target.proof?.revision !== frame.revision ||
        target.proof?.frameKey !== frame.frameKey ||
        !proofSatisfies(target)
      ) {
        return false;
      }
      targets.push(target);
    }
    const previousRevision = this.published?.snapshot.revision;
    const nextRevision = bundle.snapshot.revision;
    if (
      previousRevision !== nextRevision &&
      !this.#invokeCore((core) => core.replace_viewport_anchor_presentation({ value: nextRevision }))
    ) {
      return false;
    }
    for (const target of targets) {
      target.requiredRevision = undefined;
      target.failedRevision = undefined;
    }
    this.published = bundle;
    this.#publishedHostToken = host.token;
    this.#pullToolbarStateIfReady();
    return true;
  }

  completePresentation(bundle: PublishedBundle): void {
    this.#completePublicationWaiters(bundle);
  }

  activateVisualHost(
    onPublicationFailure: (revision: number) => void = () => {
      // A visual Host may be activated without a reveal owner.
    },
  ): () => void {
    return untrack(() => {
      // eslint-disable-next-line @typescript-eslint/no-empty-function
      if (this.terminal) return () => {};
      if (this.#visualHost) throw new Error('Editor already has an active visual host');

      const token = {};
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      this.#visualHost = { token, targets: new Map(), onPublicationFailure };
      this.#publicationChanged();
      return () => {
        untrack(() => {
          if (this.#visualHost?.token !== token) return;
          if (this.#coreAvailable) {
            for (const page of this.#visualHost.targets.keys()) this.#invokeCore((core) => core.detach_surface(page));
          }
          this.#removeAppliedPageData(this.surfacePageRequirements);
          this.#visualHost = undefined;
          this.#publishedHostToken = undefined;
          // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable empty snapshot
          this.surfacePageRequirements = new Set();
          for (const waiter of this.#publicationWaiters) {
            if (waiter.signal && waiter.onAbort) waiter.signal.removeEventListener('abort', waiter.onAbort);
            waiter.resolve({ type: 'no_host' });
          }
          this.#publicationWaiters.clear();
        });
      };
    });
  }

  attachSurface(page: number, canvas: HTMLCanvasElement, width: number, height: number, replace?: () => void): string {
    return untrack(() => {
      if (this.terminal) return 'none';
      const host = this.#visualHost;
      if (!host) throw new Error('Cannot attach a surface without an active visual host');

      const appliedPageSize = this.#applied.pageSizes[page];
      const targetWidth = appliedPageSize?.width ?? width;
      const targetHeight = this.#applied.pageBackingSizes[page]?.height ?? appliedPageSize?.height ?? height;
      const scaleFactor = this.surfaceScaleFactor;
      const current = host.targets.get(page);
      if (
        current?.canvas === canvas &&
        current.width === targetWidth &&
        current.height === targetHeight &&
        current.scaleFactor === scaleFactor &&
        current.available
      ) {
        const backend = this.#invokeCore((core) => core.surface_backend(page));
        if (backend !== 'cpu') {
          current.proof = undefined;
          current.requiredRevision = this.#applied.revision;
          current.failedRevision = undefined;
          current.available = false;
          this.#rejectAllPublicationWaitersForSurface();
        }
        return backend;
      }

      const backend = this.#invokeCore((core) => {
        if (current) core.detach_surface(page);
        core.attach_surface(page, canvas, targetWidth, targetHeight, scaleFactor);
        return core.surface_backend(page);
      });
      const target: SurfaceTarget = {
        page,
        key: ++this.#surfaceKey,
        canvas,
        width: targetWidth,
        height: targetHeight,
        scaleFactor,
        requiredRevision: this.#applied.revision,
        proof: undefined,
        failedRevision: undefined,
        available: backend === 'cpu',
        replace,
      };
      host.targets.set(page, target);
      this.#renderRequiredTargets();
      this.#publicationChanged();
      if (!target.available) this.#rejectAllPublicationWaitersForSurface();
      return backend;
    });
  }

  detachSurface(page: number): void {
    untrack(() => {
      if (this.terminal) return;
      const target = this.#visualHost?.targets.get(page);
      if (!target) return;
      this.#invokeCore((core) => core.detach_surface(page));
      this.#visualHost?.targets.delete(page);
      this.#publicationChanged();
    });
  }

  invalidateSurface(page: number): void {
    untrack(() => {
      if (this.terminal) return;
      const target = this.#visualHost?.targets.get(page);
      if (!target) return;
      const backend = this.#invokeCore((core) => {
        core.invalidate_surface(page);
        return core.surface_backend(page);
      });
      target.key = ++this.#surfaceKey;
      target.proof = undefined;
      target.requiredRevision = this.#applied.revision;
      target.failedRevision = undefined;
      target.available = backend === 'cpu';
      this.#renderRequiredTargets();
      this.#publicationChanged();
      if (!target.available) this.#rejectAllPublicationWaitersForSurface();
    });
  }

  fail(error: unknown): void {
    if (this.terminal) return;
    const reason = error ?? new Error('Editor operation failed');
    this.#failure = reason;
    this.#failed = true;
    try {
      Sentry.captureException(reason);
    } catch {
      // Telemetry must not interrupt terminal cleanup.
    }
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable empty snapshot
    this.surfacePageRequirements = new Set();
    this.#stopRuntime();
    unregister(this);
    for (const receipt of this.#receipts.values()) {
      receipt.request.discardAfterFailure(reason);
      receipt.reject(reason);
    }
    this.#receipts.clear();
    for (const waiter of this.#appliedWaiters) waiter.reject(reason);
    this.#appliedWaiters.clear();
    for (const waiter of this.#publicationWaiters) {
      if (waiter.signal && waiter.onAbort) waiter.signal.removeEventListener('abort', waiter.onAbort);
      waiter.reject(reason);
    }
    this.#publicationWaiters.clear();
    this.#listeners.clear();
    this.#contextMenuContributors.clear();
    this.#discardCore();
  }

  surfaceReplacementFailed(page: number): void {
    this.fail(new Error(`Editor surface replacement failed for page ${page}`));
  }

  publishedSurfaceCanvas(page: number): HTMLCanvasElement | undefined {
    return this.published?.frames.get(page)?.canvas;
  }

  recoverSurfaces(): void {
    untrack(() => {
      if (this.terminal) return;
      for (const target of this.#visualHost?.targets.values() ?? []) {
        const backend = this.#invokeCore((core) => {
          core.refresh_surface(target.page);
          return core.surface_backend(target.page);
        });
        target.key = ++this.#surfaceKey;
        target.proof = undefined;
        target.requiredRevision = this.#applied.revision;
        target.failedRevision = undefined;
        target.available = backend === 'cpu';
      }
      this.#renderRequiredTargets();
      this.#publicationChanged();
      if ([...(this.#visualHost?.targets.values() ?? [])].some((target) => !target.available)) {
        this.#rejectAllPublicationWaitersForSurface();
      }
    });
  }

  surfaceConfigMatches(page: number, width: number, height: number): boolean {
    return untrack(() => {
      if (this.terminal) return false;
      const target = this.#visualHost?.targets.get(page);
      if (!target) return false;
      const scaleFactor = this.surfaceScaleFactor;
      return target.width === width && target.height === height && target.scaleFactor === scaleFactor;
    });
  }

  setExternalElementHeight(nodeId: string, height: number): void {
    this.enqueue({ type: 'system', event: { type: 'set_external_height', node_id: nodeId, height } });
  }

  setThemeVariant(variant: ThemeVariant): void {
    Editor.setThemeVariant(variant);
  }

  currentHeads(): Uint8Array {
    return this.#invokeCore((core) => core.current_heads());
  }

  localChangesetsSince(remoteHeads: Uint8Array): Uint8Array {
    return this.#invokeCore((core) => core.local_changesets_since(remoteHeads));
  }

  changesetIds(): string[] {
    return this.#invokeCore((core) => core.changeset_ids());
  }

  missingChangesetsFor(confirmedHeads: Uint8Array): { bytes: Uint8Array; withheld: number } {
    const result = this.#invokeCore((core) => core.missing_changesets_tolerant(confirmedHeads));
    return { bytes: new Uint8Array(result.bytes), withheld: result.withheld };
  }

  partitionRemoteChangesets(payload: Uint8Array): { ready: Uint8Array; blocked: Uint8Array } {
    const result = this.#invokeCore((core) => core.partition_remote_changesets(payload));
    return { ready: new Uint8Array(result.ready), blocked: new Uint8Array(result.blocked) };
  }

  splitChangesets(payload: Uint8Array): { id: string; bytes: Uint8Array }[] {
    return this.#invokeCore((core) => core.split_changesets(payload)).map((entry) => ({
      id: entry.id,
      bytes: new Uint8Array(entry.bytes),
    }));
  }

  receiveRemoteChangeset(payload: Uint8Array): Promise<number> {
    if (this.#destroyed) return Promise.reject(new Error('Editor is disposed'));
    if (this.#failed) return Promise.reject(this.#failure);
    if (payload.byteLength === 0) return Promise.resolve(this.#applied.revision);
    try {
      this.#invokeCore((core) => core.receive_remote_changeset(payload));
    } catch (err) {
      return Promise.reject(err);
    }
    const promise = new Promise<number>((resolve, reject) => {
      this.#appliedWaiters.add({ resolve, reject });
    });
    this.#requestWasmTick();
    return promise;
  }

  receiveResourceUpdate(update: ResourceUpdate): void {
    if (this.terminal) return;
    this.#invokeCore((core) => core.receive_resource_update(update));
    this.#requestWasmTick();
  }

  copySelection(): ClipboardPayload | undefined {
    return this.#invokeCore((core) => core.copy_selection());
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

    const selections = this.#invokeCore((core) => core.find_matches(query, { match_whole_word: matchWholeWord }));
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
    return this.#invokeCore((core) => core.prose_text());
  }

  proseToSelection(start: number, end: number): Selection | undefined {
    return this.#invokeCore((core) => core.prose_to_selection(start, end)) ?? undefined;
  }

  proseTextAnnotated(): string {
    return this.#invokeCore((core) => core.prose_text_annotated());
  }

  proseToSelectionAnnotated(start: number, end: number): Selection | undefined {
    return this.#invokeCore((core) => core.prose_to_selection_annotated(start, end)) ?? undefined;
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

      const counts = this.#invokeCore((core) => core.character_counts());
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
        style: { background: 'bg.purple', background_radius: 2, background_inset: 2, underline: undefined },
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
          invalidate_on_text_change: true,
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
      if (this.#applied.trackedRanges.every((r) => r.id !== errorId)) return;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: errorId, group: 'spellcheck' } });
    };

    const promoteToActiveGroup = (errorId: string): boolean => {
      if (this.#applied.trackedRanges.every((r) => r.id !== errorId)) return false;
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
      this.revealTrackedItem(id);
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

  setPrismReviewRanges(items: { id: string; selection: Selection; tone: 'issue' | 'strength' }[]): void {
    if (this.terminal) return;

    this.clearPrismReviewRanges();

    for (const item of items) {
      // 해소되지 않는 selection은 tick에서 터져 에디터를 종료시킨다.
      // freezeSelection이 그 판정(양 끝 resolve)을 큐를 거치지 않고 내주고, 그 결과가 곧 앵커다.
      const frozen = this.freezeSelection(item.selection);
      if (!frozen) continue;

      const result = this.#tryEnqueue({
        type: 'tracked_range',
        // 코멘트와 같은 결의 앵커다 — 그 대목을 고치는 것은 피드백을 처리하는 중이지 무효화가 아니다.
        // 맞춤법이 가리키는 것은 그 단어라 고치면 사라지는 것이 맞지만, 여기가 가리키는 것은 대목이다.
        op: {
          type: 'add_frozen',
          id: item.id,
          group: item.tone === 'issue' ? 'prism-issue' : 'prism-strength',
          selection: frozen,
          metadata: '',
        },
      });

      if (result.type === 'failed') {
        this.#reportError(result.error, `Failed to add prism review range: ${item.id}`);
        continue;
      }
      if (result.type === 'ignored') continue;
    }
  }

  // 스냅숏의 trackedRanges는 코어가 tracked_ranges 필드를 낼 때만 갈린다(materializeSnapshot) — 본문이
  // 리플로우해도 그 필드는 안 서므로 rects가 옛 자리에 굳는다. 여백은 rects로 좌표를 잡으니 지금 것을 직접 읽는다.
  freshTrackedRanges(): TrackedRange[] {
    // 레지스트리는 tick에서만 갈리고 그때마다 #applied가 새 객체가 된다 — 한 판에 한 번만 마샬한다
    if (this.#freshRangesFor !== this.#applied) {
      this.#freshRanges = this.#invokeCore((core) => core.tracked_ranges());
      this.#freshRangesFor = this.#applied;
      // 이 snapshot이 게시된 뒤 다음 판이 먼저 적용되어도, 호스트는 그 캔버스와 짝인 지금 좌표를 써야 한다.
      // snapshot 사본에만 남은 stale id는 undefined로 고정해 옛 rect로 되돌아가지 않게 한다.
      // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable snapshot cache installed into a WeakMap
      const resolved = new Map<string, TrackedRange | undefined>(this.#applied.trackedRanges.map((range) => [range.id, undefined]));
      for (const range of this.#freshRanges) resolved.set(range.id, range);
      this.#resolvedTrackedRanges.set(this.#applied, resolved);
    }
    return this.#freshRanges;
  }

  clearPrismReviewRanges(): void {
    for (const group of ['prism-issue', 'prism-strength']) {
      this.enqueue({ type: 'tracked_range', op: { type: 'clear_group', group } });
    }
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
    const result = this.#tryEnqueue({
      type: 'tracked_range',
      op: { type: 'add_frozen', id, group: 'comment', selection, metadata: '' },
    });
    if (result.type === 'failed') {
      this.#reportError(result.error, `Failed to add frozen comment: ${id}`);
      return;
    }
    if (result.type === 'ignored') return;
    this.#registeredCommentIds.add(id);
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
    return this.#applied.trackedRanges.some((x) => x.id === id);
  }

  commentIdAt(page: number, x: number, y: number): string | null {
    const selection = this.#applied.selection;
    if (!selection) return null;

    return this.#invokeCore((core) => {
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const geometryHitIds = new Set(core.tracked_ranges_at(page, x, y, null).map((hit) => hit.id));
      const membership = core.tracked_ranges_containing_selection(selection, null).filter((range) => geometryHitIds.has(range.id));
      return selectTrackedRangeMember(membership, COMMENT_MEMBERSHIP_GROUPS, this.activeCommentId, this.#registeredCommentIds)?.id ?? null;
    });
  }

  setActiveComment(id: string | null): void {
    if (this.activeCommentId === id) return;

    const move = (cid: string, group: 'comment' | 'comment-active'): boolean => {
      if (this.#applied.trackedRanges.every((r) => r.id !== cid)) return false;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: cid, group } });
      return true;
    };

    if (this.activeCommentId !== null) move(this.activeCommentId, 'comment');
    this.activeCommentId = id;
    if (id !== null) {
      const ok = move(id, 'comment-active');
      if (ok) {
        this.revealTrackedItem(id);
      } else {
        this.activeCommentId = null;
      }
    }
  }

  setActiveAiFeedback(id: string | null): void {
    if (this.activeAiFeedbackId === id) return;

    const restoreToNormalGroup = (feedbackId: string) => {
      if (this.#applied.trackedRanges.every((r) => r.id !== feedbackId)) return;
      this.enqueue({ type: 'tracked_range', op: { type: 'set_group', id: feedbackId, group: 'ai-feedback' } });
    };

    const promoteToActiveGroup = (feedbackId: string): boolean => {
      if (this.#applied.trackedRanges.every((r) => r.id !== feedbackId)) return false;
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
      this.revealTrackedItem(id);
    }
  }

  inspect(mode: 'state' | 'state-with-node-id' | 'state-as-macro' | 'selection-as-slice-macro') {
    const output = this.#invokeCore((core) =>
      match(mode)
        .with('state', () => core.inspect_state())
        .with('state-with-node-id', () => core.inspect_state({ show_node_ids: true }))
        .with('state-as-macro', () => core.inspect_state_as_macro())
        .with('selection-as-slice-macro', () => core.inspect_selection_as_slice_macro())
        .exhaustive(),
    );

    console.log(output);
  }

  destroy(): void {
    if (this.#destroyed) return;
    this.#destroyed = true;
    unregister(this);

    this.#stopRuntime();

    const disposed = new Error('Editor is disposed');
    for (const receipt of this.#receipts.values()) {
      receipt.request.discardAfterFailure(disposed);
      receipt.reject(disposed);
    }
    this.#receipts.clear();
    for (const waiter of this.#appliedWaiters) waiter.reject(disposed);
    this.#appliedWaiters.clear();
    for (const waiter of this.#publicationWaiters) {
      if (waiter.signal && waiter.onAbort) waiter.signal.removeEventListener('abort', waiter.onAbort);
      waiter.reject(disposed);
    }
    this.#publicationWaiters.clear();
    this.#visualHost = undefined;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable empty snapshot
    this.surfacePageRequirements = new Set();
    this.#listeners.clear();
    this.#contextMenuContributors.clear();

    this.#discardCore();
  }
}

export { EditorRequest, EditorUpdate } from './editor-update';
