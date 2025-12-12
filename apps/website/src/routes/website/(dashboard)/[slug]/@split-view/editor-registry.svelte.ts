import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { Editor as TipTapEditor } from '@tiptap/core';
import type { Ref } from '@typie/ui/utils';
import type { Editor as PenxleEditor } from '$lib/editor/editor.svelte';

const key: unique symbol = Symbol('EditorRegistry');

export type EditorEntry = { type: 'tiptap'; editor: Ref<TipTapEditor> } | { type: 'penxle'; editor: PenxleEditor };

class EditorRegistry {
  #entries = new SvelteMap<string, EditorEntry>();

  registerTipTap(viewId: string, slug: string, editor: Ref<TipTapEditor> | undefined) {
    if (editor) {
      const key = `${viewId}-${slug}`;
      this.#entries.set(key, { type: 'tiptap', editor });
    }
  }

  registerPenxle(viewId: string, slug: string, editor: PenxleEditor | undefined) {
    if (editor) {
      const key = `${viewId}-${slug}`;
      this.#entries.set(key, { type: 'penxle', editor });
    }
  }

  unregister(viewId: string, slug: string) {
    const key = `${viewId}-${slug}`;
    this.#entries.delete(key);
  }

  get(viewId: string, slug: string): EditorEntry | undefined {
    const key = `${viewId}-${slug}`;
    return this.#entries.get(key);
  }

  getTipTap(viewId: string, slug: string): Ref<TipTapEditor> | undefined {
    const entry = this.get(viewId, slug);
    if (entry?.type === 'tiptap') {
      return entry.editor;
    }
    return undefined;
  }

  getPenxle(viewId: string, slug: string): PenxleEditor | undefined {
    const entry = this.get(viewId, slug);
    if (entry?.type === 'penxle') {
      return entry.editor;
    }
    return undefined;
  }
}

export const setupEditorRegistry = () => {
  const registry = new EditorRegistry();
  setContext(key, registry);
  return registry;
};

export const getEditorRegistry = (): EditorRegistry => {
  const registry = getContext<EditorRegistry>(key);
  if (!registry) {
    throw new Error('EditorRegistry not found');
  }
  return registry;
};
