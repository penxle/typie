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
  charCount: number;
};

const key: unique symbol = Symbol('OpenDocuments');

export class OpenDocumentRegistry {
  #entries = new SvelteMap<string, OpenDocument>();

  upsert(paneKey: string, doc: OpenDocument) {
    this.#entries.set(paneKey, doc);
  }

  remove(paneKey: string) {
    this.#entries.delete(paneKey);
  }

  snapshot(): { documents: OpenDocument[] } {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- transient accumulator local to this call; never a render signal
    const byId = new Map<string, OpenDocument>();
    for (const doc of this.#entries.values()) {
      const prev = byId.get(doc.documentId);
      byId.set(
        doc.documentId,
        prev ? { ...prev, active: prev.active || doc.active, charCount: Math.max(prev.charCount, doc.charCount) } : { ...doc },
      );
    }
    return { documents: [...byId.values()].toSorted((a, b) => a.documentId.localeCompare(b.documentId)) };
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
