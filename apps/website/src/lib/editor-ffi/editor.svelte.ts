import { createContext, tick, untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { match } from 'ts-pattern';
import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
import { fontDataMissingHandler } from './fonts';
import { TouchGestureController } from './gesture.svelte';
import { readClipboardRich, writeClipboardPayload } from './handlers/clipboard';
import { register, snapshot, unregister } from './registry';
import type {
  BlockState,
  ClipboardPayload,
  CursorMetrics,
  Editor as WasmEditor,
  EditorEvent,
  ExternalElement,
  InteractiveHit,
  Message,
  Modifier,
  ModifierState,
  PlainRootNode,
  PointerStyle,
  Selection,
  SelectionEndpoints,
  Size,
  TableOverlay,
  ThemeVariant,
  Viewport,
} from '@typie/editor-ffi/browser';
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

let wasmInitPromise: Promise<void> | null = null;

function ensureWasmInitialized(): Promise<void> {
  return (wasmInitPromise ??= (async () => {
    await initWasm();
  })());
}

class EditorContext {
  editor = $state<Editor>();
  fileAssets = $state(new SvelteMap<string, FileAsset>());
}

const [getEditorContext, setEditorContext] = createContext<EditorContext>();

export { getEditorContext };
export const setupEditorContext = () => setEditorContext(new EditorContext());

export class Editor {
  #wasm!: WasmEditor;
  #destroyed = false;

  #queued = false;
  #rafId: number | null = null;

  #viewport!: Viewport;

  inputEl = $state<HTMLInputElement>();
  pageEls = $state<Record<number, HTMLDivElement | undefined>>({});
  scrollContainerEl = $state<HTMLDivElement>();

  readOnly = $state(false);

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #listeners = new Map<EditorEvent['type'], Set<EditorEventListener<EditorEvent['type']>>>();

  #cursor = $state<CursorMetrics>();
  #selection = $state<Selection | undefined>();
  #pageSizes = $state<Size[]>([]);
  #externalElements = $state<ExternalElement[]>([]);
  #tableOverlays = $state<TableOverlay[]>([]);
  #rootAttrs = $state<PlainRootNode>();
  #modifierState = $state<ModifierState | undefined>();
  #rootModifiers = $state<Modifier[]>();
  #blockState = $state<BlockState | undefined>();
  #focused = $state(false);
  #effectCleanup: (() => void) | null = null;

  #pointerStyle = $state<PointerStyle>('default');
  #lastPointerClient: { x: number; y: number } | null = null;
  #pointerStyleDomRefreshQueued = false;

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

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #contextMenuContributors = new Set<ContextMenuContributor>();

  #gesture!: TouchGestureController;

  get gesture(): TouchGestureController {
    return this.#gesture;
  }

  inflightFiles = $state(new SvelteMap<string, { name: string; size: number }>());

  embedAssets = $state(new SvelteMap<string, EmbedAsset>());
  archivedAssets = $state(new SvelteMap<string, ArchivedAsset>());

  private constructor() {
    // no-op
  }

  static async create(graph: Uint8Array, viewport: Viewport, themeVariant: ThemeVariant = 'light-white') {
    await ensureWasmInitialized();

    const self = new this();

    self.#wasm = wasm.create_editor_from_graph(graph, viewport);
    self.#viewport = viewport;
    self.#gesture = new TouchGestureController(self);

    self.on('state_changed', self.#stateChangedHandler);
    self.on('font_data_missing', fontDataMissingHandler);

    register(self);

    self.#effectCleanup = $effect.root(() => {
      $effect(() => {
        const el = self.inputEl;
        if (!el) {
          untrack(() => self.#setFocused(false));
          return;
        }

        const onFocus = () => {
          self.#setFocused(true);
        };
        const onBlur = () => {
          self.#setFocused(false);
        };

        el.addEventListener('focus', onFocus);
        el.addEventListener('blur', onBlur);
        untrack(() => self.#setFocused(document.activeElement === el));

        return () => {
          el.removeEventListener('focus', onFocus);
          el.removeEventListener('blur', onBlur);
        };
      });
    });

    wasm.set_theme_variant(themeVariant);
    self.enqueue({ type: 'system', event: { type: 'theme_variant_changed' } });
    self.enqueue({ type: 'system', event: { type: 'initialize' } });

    return self;
  }

  get cursor() {
    return this.#cursor;
  }

  get selection() {
    return this.#selection;
  }

  get isSelectionCollapsed(): boolean {
    const sel = this.#selection;
    if (!sel) return true;
    return sel.anchor.node_id === sel.head.node_id && sel.anchor.offset === sel.head.offset && sel.anchor.affinity === sel.head.affinity;
  }

  get pageSizes() {
    return this.#pageSizes;
  }

  get externalElements() {
    return this.#externalElements;
  }

  get tableOverlays() {
    return this.#tableOverlays;
  }

  get rootAttrs() {
    return this.#rootAttrs;
  }

  get rootModifiers() {
    return this.#rootModifiers;
  }

  get modifierState() {
    return this.#modifierState;
  }

  get blockState() {
    return this.#blockState;
  }

  get scaleFactor() {
    return this.#viewport.scale_factor;
  }

  focus() {
    this.inputEl?.focus({ preventScroll: true });
  }

  blur() {
    this.inputEl?.blur();
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

  get focusable() {
    return !!this.inputEl;
  }

  get focused() {
    return this.#focused;
  }

  #setFocused(focused: boolean): void {
    if (this.#focused === focused) {
      return;
    }

    this.#focused = focused;
    this.enqueue({ type: 'system', event: { type: 'set_focused', focused } });
  }

  get pointerStyle() {
    return this.#pointerStyle;
  }

  localToOffset(page: number, x: number, y: number) {
    const el = this.pageEls[page];
    if (!el) {
      return null;
    }

    return {
      x: el.offsetLeft + x,
      y: el.offsetTop + y,
    };
  }

  clientToLocal(clientX: number, clientY: number) {
    const pages = this.#pageSizes;
    if (pages.length === 0) return null;

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
    let localY = clientY - rect.top;

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
    const localX = Math.max(0, Math.min(clientX - rect.left, size.width));
    localY = Math.max(0, Math.min(localY, size.height));
    return { page: lo, x: localX, y: localY };
  }

  interactiveHitTest(page: number, x: number, y: number): InteractiveHit | undefined {
    return this.#wasm.interactive_hit_test(page, x, y);
  }

  selectionEndpoints(): SelectionEndpoints | undefined {
    return this.#wasm.selection_endpoints();
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
    this.#pointerStyle = local ? this.#wasm.pointer_style(local.page, local.x, local.y, this.readOnly) : 'default';
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
    this.#wasm.enqueue(message);
    this.#scheduleTick();
  }

  #scheduleTick(): void {
    if (!this.#queued) {
      this.#queued = true;

      if (this.#rafId === null) {
        this.#rafId = requestAnimationFrame(this.#tick);
      }
    }
  }

  attachSurface(page: number, canvas: HTMLCanvasElement, width: number, height: number): void {
    this.#wasm.attach_surface(page, canvas, width, height, this.#viewport.scale_factor);
  }

  detachSurface(page: number): void {
    this.#wasm.detach_surface(page);
  }

  renderSurface(page: number): void {
    this.#wasm.render_surface(page);
  }

  resizeSurface(page: number, width: number, height: number): void {
    this.#wasm.resize_surface(page, width, height, this.#viewport.scale_factor);
  }

  setExternalElementHeight(nodeId: string, height: number): void {
    this.enqueue({ type: 'system', event: { type: 'set_external_height', node_id: nodeId, height } });
  }

  setThemeVariant(variant: ThemeVariant): void {
    Editor.setThemeVariant(variant);
  }

  static setThemeVariant(variant: ThemeVariant): void {
    const changed = wasm.set_theme_variant(variant);
    if (!changed) return;
    for (const editor of snapshot()) {
      editor.enqueue({ type: 'system', event: { type: 'theme_variant_changed' } });
    }
  }

  currentHeads(): Uint8Array {
    return this.#wasm.current_heads();
  }

  localChangesetsSince(remoteHeads: Uint8Array): Uint8Array {
    return this.#wasm.local_changesets_since(remoteHeads);
  }

  receiveRemoteChangeset(payload: Uint8Array): void {
    this.#wasm.receive_remote_changeset(payload);
    this.#scheduleTick();
  }

  copySelection(): ClipboardPayload | undefined {
    return this.#wasm.copy_selection();
  }

  inspect(mode: 'state' | 'state-with-node-id' | 'state-as-macro') {
    const output = match(mode)
      .with('state', () => this.#wasm.inspect_state())
      .with('state-with-node-id', () => this.#wasm.inspect_state({ show_node_ids: true }))
      .with('state-as-macro', () => this.#wasm.inspect_state_as_macro())
      .exhaustive();

    console.log(output);
  }

  #tick = (): void => {
    this.#rafId = null;

    if (this.#queued) {
      this.#queued = false;

      const events = this.#wasm.tick();
      for (const event of events) {
        this.#emit(event);
      }
    }
  };

  #emit(event: EditorEvent): void {
    const set = this.#listeners.get(event.type);
    if (set) {
      for (const cb of set) {
        (cb as EditorEventListener<typeof event.type>)(this, event as never);
      }
    }
  }

  #stateChangedHandler: EditorEventListener<'state_changed'> = (_, { fields }) => {
    if (fields.includes('cursor')) {
      this.#cursor = this.#wasm.cursor();
    }

    if (fields.includes('selection')) {
      this.#selection = this.#wasm.selection();
      // null selection is the unfocused state; release DOM focus so OS caret and IME follow.
      if (this.#selection === undefined) {
        this.inputEl?.blur();
      }
    }

    if (fields.includes('page_sizes')) {
      this.#pageSizes = this.#wasm.page_sizes();
    }

    if (fields.includes('external_elements')) {
      this.#externalElements = this.#wasm.external_elements();
    }

    if (fields.includes('table_overlays')) {
      this.#tableOverlays = this.#wasm.table_overlays();
    }

    if (fields.includes('root_attrs')) {
      this.#rootAttrs = this.#wasm.root_attrs();
    }

    if (fields.includes('modifiers')) {
      this.#modifierState = this.#wasm.modifier_state();
      this.#rootModifiers = this.#wasm.root_modifiers();
    }

    if (fields.includes('block')) {
      this.#blockState = this.#wasm.block_state();
    }

    const pageDomChanged = fields.includes('root_attrs') || fields.includes('page_sizes');
    if (pageDomChanged) {
      this.refreshPointerStyleAfterDomUpdate();
    } else if (fields.some((field) => ['doc', 'external_elements', 'modifiers', 'block'].includes(field))) {
      this.refreshPointerStyle();
    }
  };

  destroy(): void {
    this.#destroyed = true;

    unregister(this);

    this.#effectCleanup?.();
    this.#effectCleanup = null;

    if (this.#rafId !== null) {
      cancelAnimationFrame(this.#rafId);
      this.#rafId = null;
    }

    this.#gesture?.destroy();
    this.#wasm?.free();
  }
}
