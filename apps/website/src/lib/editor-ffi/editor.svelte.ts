import icuUrl from '@typie/editor-ffi/browser/icu.zst?url';
import { createContext } from 'svelte';
import { match } from 'ts-pattern';
import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
import { fontDataMissingHandler, fontManifestMissingHandler, initFonts } from './fonts';
import type { Doc, Editor as WasmEditor, EditorEvent, Message, PageRect, Selection, Size, Viewport } from '@typie/editor-ffi/browser';
import type { EditorEventListener } from './types';

let initPromise: Promise<void> | null = null;

const initIcu = async () => {
  const resp = await fetch(icuUrl);
  const data = await resp.arrayBuffer();
  wasm.load_icu_data(new Uint8Array(data));
};

function ensureInitialized(): Promise<void> {
  return (initPromise ??= (async () => {
    await initWasm();
    await initIcu();
    await initFonts();
    wasm.set_font_families([{ name: 'Pretendard', weights: [400] }]);
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

  #queued = false;
  #rafId: number | null = null;

  #viewport!: Viewport;

  inputEl = $state<HTMLInputElement>();
  pageEls = $state<Record<number, HTMLDivElement | undefined>>({});

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #listeners = new Map<EditorEvent['type'], Set<EditorEventListener<EditorEvent['type']>>>();

  #cursor = $state<PageRect>();
  #selection = $state<Selection>();
  #pageSizes = $state<Size[]>([]);

  private constructor() {
    // no-op
  }

  static async create(doc: Doc, selection: Selection, viewport: Viewport) {
    await ensureInitialized();

    const self = new this();

    self.#wasm = wasm.create_editor(doc, selection, viewport);
    self.#viewport = viewport;

    self.on('state_changed', self.#stateChangedHandler);
    self.on('font_manifest_missing', fontManifestMissingHandler);
    self.on('font_data_missing', fontDataMissingHandler);

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

  focus() {
    this.inputEl?.focus();
  }

  blur() {
    this.inputEl?.blur();
  }

  get focusable() {
    return !!this.inputEl;
  }

  getCursorRect(page: number) {
    const el = this.pageEls[page];
    const size = this.pageSizes[page];

    if (!el || !size) {
      return null;
    }

    return {
      x: el.offsetLeft,
      y: el.offsetTop,
      ...size,
    };
  }

  localToGlobal(page: number, x: number, y: number) {
    const el = this.pageEls[page];
    if (!el) {
      return null;
    }

    return {
      x: el.offsetLeft + x,
      y: el.offsetTop + y,
    };
  }

  globalToLocal(x: number, y: number) {
    const pages = this.#pageSizes;
    if (pages.length === 0) return null;

    let lo = 0;
    let hi = pages.length - 1;

    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      const el = this.pageEls[mid];
      if (!el) return null;
      if (el.offsetTop + pages[mid].height <= y) lo = mid + 1;
      else hi = mid;
    }

    const el = this.pageEls[lo];
    if (!el) return null;
    let localY = y - el.offsetTop;

    if (localY < 0 && lo > 0) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const prevBottom = this.pageEls[lo - 1]!.offsetTop + pages[lo - 1].height;
      if (y < (prevBottom + el.offsetTop) / 2) {
        lo--;
        localY = pages[lo].height;
      } else {
        localY = 0;
      }
    }

    const size = pages[lo];
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const localX = Math.max(0, Math.min(x - this.pageEls[lo]!.offsetLeft, size.width));
    localY = Math.max(0, Math.min(localY, size.height));
    return { page: lo, x: localX, y: localY };
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
  };

  destroy(): void {
    if (this.#rafId !== null) {
      cancelAnimationFrame(this.#rafId);
      this.#rafId = null;
    }

    this.#wasm?.free();
  }
}
