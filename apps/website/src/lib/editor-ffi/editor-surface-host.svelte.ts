import { roundToScale } from './geometry';
import { createSurfaceDriver } from './surface-driver';
import { probeAttach, probeDetach, probeEvent } from './surface-probe';
import type { Editor, PublishedBundle } from './editor.svelte';
import type { SurfaceDriverEffects } from './surface-driver';

type PageProducer = {
  width: number;
  height: number;
  driver: ReturnType<typeof createSurfaceDriver<HTMLElement>>;
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
      if (!this.#editor.publishedSurfaceElement(page)) {
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
    this.#producers.get(page)?.driver.syncPublished(this.#editor.publishedSurfaceElement(page));
    return () => {
      if (this.#containers.get(page) === container) this.#containers.delete(page);
    };
  }

  syncPublished(bundle: PublishedBundle | undefined = this.#editor.published): void {
    for (const [page, producer] of this.#producers) {
      const surface = bundle?.frames.get(page)?.surface;
      producer.driver.syncPublished(surface);
      if (!surface && !this.#editor.surfacePageRequirements.has(page)) {
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
    const effects: SurfaceDriverEffects<HTMLElement> = {
      createSurface: () => {
        const surface = document.createElement('div');
        surface.dataset.pageSurface = String(page);
        surface.style.position = 'absolute';
        surface.style.inset = '0';
        surface.style.width = '100%';
        return surface;
      },
      styleSurface: (surface) => {
        const scaleFactor = this.#editor.scaleFactor;
        surface.style.height = `${roundToScale(producer.height, scaleFactor)}px`;
      },
      attach: (surface) => {
        const backend = this.#editor.attachSurface(page, surface, producer.width, producer.height, () => producer.driver.replace());
        probeAttach(this.#editor, page, surface);
        if (backend === 'cpu') return 'cpu';
        return backend === 'cpu-oversized' ? 'cpu-oversized' : 'none';
      },
      detach: () => {
        probeDetach(this.#editor, page);
        this.#editor.detachSurface(page);
      },
      recover: () => this.#editor.invalidateSurface(page),
      addContextListeners: (surface, isCurrent) => {
        const onContextRestored = () => {
          probeEvent(`contextrestored page=${page}`);
          if (isCurrent()) this.#editor.invalidateSurface(page);
        };
        surface.addEventListener('contextrestored', onContextRestored, { capture: true });
        return () => surface.removeEventListener('contextrestored', onContextRestored, true);
      },
      releaseCpuBacking: (surface) => {
        surface.replaceChildren();
      },
      promote: (surface) => {
        const container = this.#containers.get(page);
        if (container && surface.parentNode !== container) container.append(surface);
      },
      removeNode: (surface) => surface.remove(),
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
