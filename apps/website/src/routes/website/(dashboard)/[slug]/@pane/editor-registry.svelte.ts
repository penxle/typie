import { createStableContext } from '@typie/ui/context/stable';
import { SvelteMap } from 'svelte/reactivity';
import type { Editor as FfiEditor } from '$lib/editor-ffi/editor.svelte';

export type RegisteredEditor = FfiEditor;

const [getEditorRegistry, setEditorRegistry] = createStableContext<EditorRegistry>('pane.EditorRegistry');

export { getEditorRegistry };

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
  setEditorRegistry(registry);
  return registry;
};
