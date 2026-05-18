import { createContext, tick, untrack } from 'svelte';
import { match } from 'ts-pattern';
import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
import { fontDataMissingHandler } from './fonts';
import { register, unregister } from './registry';
import type {
  BlockState,
  CursorMetrics,
  Editor as WasmEditor,
  EditorEvent,
  ExternalElement,
  InteractiveHit,
  Message,
  ModifierState,
  PlainRootNode,
  PointerStyle,
  Selection,
  Size,
  ThemeVariant,
  Viewport,
} from '@typie/editor-ffi/browser';
import type { EditorEventListener } from './types';

let wasmInitPromise: Promise<void> | null = null;

function ensureWasmInitialized(): Promise<void> {
  return (wasmInitPromise ??= (async () => {
    await initWasm();
  })());
}

class EditorContext {
  editor = $state<Editor>();
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

  readOnly = false;

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #listeners = new Map<EditorEvent['type'], Set<EditorEventListener<EditorEvent['type']>>>();

  #cursor = $state<CursorMetrics>();
  #selection = $state<Selection>();
  #pageSizes = $state<Size[]>([]);
  #externalElements = $state<ExternalElement[]>([]);
  #rootAttrs = $state<PlainRootNode>();
  #modifierState = $state<ModifierState>();
  #blockState = $state<BlockState>();
  #focused = $state(false);
  #effectCleanup: (() => void) | null = null;

  #pointerStyle = $state<PointerStyle>('default');
  #lastPointerClient: { x: number; y: number } | null = null;
  #pointerStyleDomRefreshQueued = false;

  private constructor() {
    // no-op
  }

  static async create(graph: Uint8Array, viewport: Viewport, themeVariant: ThemeVariant = 'light-white') {
    await ensureWasmInitialized();

    const self = new this();

    self.#wasm = wasm.create_editor_from_graph(graph, viewport);
    self.#viewport = viewport;

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

    self.enqueue({ type: 'system', event: { type: 'set_theme_variant', variant: themeVariant } });
    self.enqueue({ type: 'system', event: { type: 'initialize' } });

    return self;
  }

  get cursor() {
    return this.#cursor;
  }

  get selection() {
    return this.#selection;
  }

  get pageSizes() {
    return this.#pageSizes;
  }

  get externalElements() {
    return this.#externalElements;
  }

  get rootAttrs() {
    return this.#rootAttrs;
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
    this.enqueue({ type: 'system', event: { type: 'set_theme_variant', variant } });
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
    }

    if (fields.includes('page_sizes')) {
      this.#pageSizes = this.#wasm.page_sizes();
    }

    if (fields.includes('external_elements')) {
      this.#externalElements = this.#wasm.external_elements();
    }

    if (fields.includes('root_attrs')) {
      this.#rootAttrs = this.#wasm.root_attrs();
    }

    if (fields.includes('modifiers')) {
      this.#modifierState = this.#wasm.modifier_state();
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

    this.#wasm?.free();
  }
}
