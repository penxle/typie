import { roundToScale } from './geometry';
import { createSurfaceDriver } from './surface-driver';
import { probeAttach, probeDetach, probeEvent } from './surface-probe';
import type { Editor, PublishedBundle } from './editor.svelte';
import type { SurfaceDriverEffects } from './surface-driver';

type PageProducer = {
  width: number;
  height: number;
  driver: ReturnType<typeof createSurfaceDriver<HTMLCanvasElement>>;
};

export class EditorSurfaceHost {
  readonly #editor: Editor;
  readonly #releaseVisualHost: () => void;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperative producer registry; mutations are not render signals
  readonly #producers = new Map<number, PageProducer>();
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- imperative DOM registry; mutations are not render signals
  readonly #containers = new Map<number, HTMLElement>();

  constructor(editor: Editor, onPublicationFailure: (revision: number) => void) {
    this.#editor = editor;
    this.#releaseVisualHost = editor.activateVisualHost(onPublicationFailure);
    document.addEventListener('visibilitychange', this.#resumeWhenVisible);
    window.addEventListener('pageshow', this.#resume);
  }

  reconcile(requiredPages: ReadonlySet<number>): void {
    if (this.#editor.terminal) {
      for (const producer of this.#producers.values()) producer.driver.freeze();
      return;
    }

    for (const [page, producer] of this.#producers) {
      if (requiredPages.has(page)) continue;
      producer.driver.setActive(false);
      if (!this.#editor.publishedSurfaceCanvas(page)) {
        producer.driver.destroy();
        this.#producers.delete(page);
      }
    }

    const snapshot = this.#editor.appliedSnapshot;
    for (const page of requiredPages) {
      const size = snapshot.pageSizes[page];
      if (!size) continue;
      const width = size.width;
      const height = snapshot.pageBackingSizes[page]?.height ?? size.height;
      const current = this.#producers.get(page);
      if (!current) {
        const producer = this.#createProducer(page, width, height);
        this.#producers.set(page, producer);
        producer.driver.setActive(true);
        continue;
      }

      current.width = width;
      current.height = height;
      current.driver.setActive(true);
      if (this.#editor.surfaceConfigMatches(page, width, height)) current.driver.restyle();
      else current.driver.replace();
    }
  }

  registerPageContainer(page: number, container: HTMLElement): () => void {
    this.#containers.set(page, container);
    this.#producers.get(page)?.driver.syncPublished(this.#editor.publishedSurfaceCanvas(page));
    return () => {
      if (this.#containers.get(page) === container) this.#containers.delete(page);
    };
  }

  syncPublished(bundle: PublishedBundle | undefined = this.#editor.published): void {
    for (const [page, producer] of this.#producers) {
      const canvas = bundle?.frames.get(page)?.canvas;
      producer.driver.syncPublished(canvas);
      if (!canvas && !this.#editor.surfacePageRequirements.has(page)) {
        producer.driver.destroy();
        this.#producers.delete(page);
      }
    }
  }

  destroy(): void {
    document.removeEventListener('visibilitychange', this.#resumeWhenVisible);
    window.removeEventListener('pageshow', this.#resume);
    for (const producer of this.#producers.values()) producer.driver.destroy();
    this.#producers.clear();
    this.#containers.clear();
    this.#releaseVisualHost();
  }

  // eslint-disable-next-line unicorn/consistent-class-member-order -- private factory is kept beside the public lifecycle methods it implements
  #createProducer(page: number, width: number, height: number): PageProducer {
    // eslint-disable-next-line prefer-const -- callbacks close over the carrier before its driver can be constructed
    let producer: PageProducer;
    const effects: SurfaceDriverEffects<HTMLCanvasElement> = {
      createCanvas: () => {
        const canvas = document.createElement('canvas');
        canvas.dataset.pageCanvas = String(page);
        canvas.style.position = 'absolute';
        canvas.style.inset = '0';
        canvas.style.width = '100%';
        canvas.style.imageRendering = 'pixelated';
        return canvas;
      },
      styleCanvas: (canvas) => {
        const scaleFactor = this.#editor.scaleFactor;
        canvas.style.height = `${roundToScale(producer.height, scaleFactor)}px`;
      },
      attach: (canvas) => {
        const backend = this.#editor.attachSurface(page, canvas, producer.width, producer.height, () => producer.driver.replace());
        probeAttach(this.#editor, page, canvas);
        if (backend === 'cpu') return 'cpu';
        return backend === 'cpu-oversized' ? 'cpu-oversized' : 'none';
      },
      detach: () => {
        probeDetach(this.#editor, page);
        this.#editor.detachSurface(page);
      },
      recover: () => this.#editor.invalidateSurface(page),
      addContextListeners: (canvas, isCurrent) => {
        const onContextRestored = () => {
          probeEvent(`contextrestored page=${page}`);
          if (isCurrent()) this.#editor.invalidateSurface(page);
        };
        canvas.addEventListener('contextrestored', onContextRestored);
        return () => canvas.removeEventListener('contextrestored', onContextRestored);
      },
      releaseCpuBacking: (canvas) => {
        canvas.width = 0;
        canvas.height = 0;
      },
      promote: (canvas) => {
        const container = this.#containers.get(page);
        if (container && canvas.parentNode !== container) container.append(canvas);
      },
      removeNode: (canvas) => canvas.remove(),
      replacementFailed: () => this.#editor.surfaceReplacementFailed(page),
    };
    producer = { width, height, driver: createSurfaceDriver(effects) };
    return producer;
  }

  #resumeWhenVisible = (): void => {
    if (document.visibilityState === 'visible') this.#resume();
  };

  #resume = (): void => {
    for (const producer of this.#producers.values()) producer.driver.resume();
  };
}
