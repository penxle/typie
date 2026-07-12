import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { Editor as NativeEditor } from '$lib/editor/editor.svelte';
import type { Editor as FfiEditor } from '$lib/editor-ffi/editor.svelte';

export type RegisteredEditor = NativeEditor | FfiEditor;

const key: unique symbol = Symbol('EditorRegistry');

class EditorRegistry {
  #entries = new SvelteMap<string, RegisteredEditor>();

  register(paneId: string, slug: string, editor: RegisteredEditor | undefined) {
    if (editor) {
      const key = `${paneId}-${slug}`;
      this.#entries.set(key, editor);
    }
  }

  unregister(paneId: string, slug: string) {
    const key = `${paneId}-${slug}`;
    this.#entries.delete(key);
  }

  get(paneId: string, slug: string): RegisteredEditor | undefined {
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
