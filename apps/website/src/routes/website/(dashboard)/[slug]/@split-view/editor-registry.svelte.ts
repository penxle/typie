import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { Editor } from '@tiptap/core';
import type { Ref } from '@typie/ui/utils';

const key: unique symbol = Symbol('EditorRegistry');

class EditorRegistry {
  #entries = new SvelteMap<string, Ref<Editor>>();

  register(viewId: string, slug: string, editor: Ref<Editor> | undefined) {
    if (editor) {
      const key = `${viewId}-${slug}`;
      this.#entries.set(key, editor);
    }
  }

  unregister(viewId: string, slug: string) {
    const key = `${viewId}-${slug}`;
    this.#entries.delete(key);
  }

  get(viewId: string, slug: string): Ref<Editor> | undefined {
    const key = `${viewId}-${slug}`;
    return this.#entries.get(key);
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
