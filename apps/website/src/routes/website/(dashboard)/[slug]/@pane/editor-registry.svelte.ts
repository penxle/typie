import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { Editor as NativeEditor } from '$lib/editor/editor.svelte';

const key: unique symbol = Symbol('EditorRegistry');

class EditorRegistry {
  #entries = new SvelteMap<string, NativeEditor>();

  register(paneId: string, slug: string, editor: NativeEditor | undefined) {
    if (editor) {
      const key = `${paneId}-${slug}`;
      this.#entries.set(key, editor);
    }
  }

  unregister(paneId: string, slug: string) {
    const key = `${paneId}-${slug}`;
    this.#entries.delete(key);
  }

  get(paneId: string, slug: string): NativeEditor | undefined {
    const key = `${paneId}-${slug}`;
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
