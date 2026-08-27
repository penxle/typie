import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';

export type OpenDocument = {
  kind: 'document';
  documentId: string;
  entityId: string;
  title: string | null;
  subtitle: string | null;
  icon: string;
  iconColor: string;
  active: boolean;
};

const key: unique symbol = Symbol('OpenDocuments');

export class OpenDocumentRegistry {
  #entries = new SvelteMap<string, OpenDocument | null>();
  #expectedPaneIds: Set<string> | null = null;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- callbacks are notified imperatively when readiness changes
  #readyWaiters = new Set<() => void>();

  #accepts(paneId: string) {
    return this.#expectedPaneIds === null || this.#expectedPaneIds.has(paneId);
  }

  #ready() {
    return this.#expectedPaneIds !== null && [...this.#expectedPaneIds].every((paneId) => this.#entries.has(paneId));
  }

  #notifyReady() {
    if (!this.#ready()) return;

    for (const waiter of this.#readyWaiters) {
      waiter();
    }
  }

  setExpectedPanes(paneIds: readonly string[]) {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- authoritative input is synchronized imperatively by the pane host
    const next = new Set(paneIds);
    this.#expectedPaneIds = next;

    for (const paneId of this.#entries.keys()) {
      if (!next.has(paneId)) {
        this.#entries.delete(paneId);
      }
    }

    this.#notifyReady();
  }

  invalidate() {
    this.#expectedPaneIds = null;
    this.#entries.clear();
  }

  expectPane(paneId: string) {
    if (!this.#accepts(paneId)) return;
    this.#entries.delete(paneId);
  }

  resolvePane(paneId: string) {
    if (!this.#accepts(paneId)) return;
    this.#entries.set(paneId, null);
    this.#notifyReady();
  }

  upsert(paneId: string, doc: OpenDocument) {
    if (!this.#accepts(paneId)) return;
    this.#entries.set(paneId, doc);
    this.#notifyReady();
  }

  snapshot(): { documents: OpenDocument[] } {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- transient accumulator local to this call; never a render signal
    const byId = new Map<string, OpenDocument>();
    for (const doc of this.#entries.values()) {
      if (doc === null) continue;
      const prev = byId.get(doc.documentId);
      byId.set(doc.documentId, prev ? { ...prev, active: prev.active || doc.active } : { ...doc });
    }
    return { documents: [...byId.values()].toSorted((a, b) => a.documentId.localeCompare(b.documentId)) };
  }

  snapshotWhenReady(timeoutMs = 2000): Promise<{ documents: OpenDocument[] }> {
    if (this.#ready()) {
      return Promise.resolve(this.snapshot());
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.#readyWaiters.delete(onReady);
        reject(new Error(`Open documents are not ready after ${timeoutMs}ms`));
      }, timeoutMs);

      const onReady = () => {
        if (!this.#ready()) return;

        clearTimeout(timeout);
        this.#readyWaiters.delete(onReady);
        resolve(this.snapshot());
      };

      this.#readyWaiters.add(onReady);
      onReady();
    });
  }
}

export const setupOpenDocuments = () => {
  const registry = new OpenDocumentRegistry();
  setContext(key, registry);
  return registry;
};

export const getOpenDocuments = (): OpenDocumentRegistry => {
  const registry = getContext<OpenDocumentRegistry>(key);
  if (!registry) {
    throw new Error('OpenDocumentRegistry not found');
  }
  return registry;
};
