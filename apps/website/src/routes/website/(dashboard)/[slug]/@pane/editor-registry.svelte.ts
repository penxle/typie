import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { Editor as TipTapEditor } from '@tiptap/core';
import type { Ref } from '@typie/ui/utils';
import type { Editor as NativeEditor } from '$lib/editor/editor.svelte';

const key: unique symbol = Symbol('EditorRegistry');

export type EditorEntry = { type: 'tiptap'; editor: Ref<TipTapEditor> } | { type: 'native'; editor: NativeEditor };

class EditorRegistry {
  #entries = new SvelteMap<string, EditorEntry>();

  registerTipTap(paneId: string, slug: string, editor: Ref<TipTapEditor> | undefined) {
    if (editor) {
      const key = `${paneId}-${slug}`;
      this.#entries.set(key, { type: 'tiptap', editor });
    }
  }

  registerNative(paneId: string, slug: string, editor: NativeEditor | undefined) {
    if (editor) {
      const key = `${paneId}-${slug}`;
      this.#entries.set(key, { type: 'native', editor });
    }
  }

  unregister(paneId: string, slug: string) {
    const key = `${paneId}-${slug}`;
    this.#entries.delete(key);
  }

  get(paneId: string, slug: string): EditorEntry | undefined {
    const key = `${paneId}-${slug}`;
    return this.#entries.get(key);
  }

  getTipTap(paneId: string, slug: string): Ref<TipTapEditor> | undefined {
    const entry = this.get(paneId, slug);
    if (entry?.type === 'tiptap') {
      return entry.editor;
    }
    return undefined;
  }

  getNative(paneId: string, slug: string): NativeEditor | undefined {
    const entry = this.get(paneId, slug);
    if (entry?.type === 'native') {
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
