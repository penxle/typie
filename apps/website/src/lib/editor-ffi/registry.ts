import type { ResourceUpdate } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

const editors = new Set<Editor>();

export function register(editor: Editor): void {
  editors.add(editor);
}

export function unregister(editor: Editor): void {
  editors.delete(editor);
}

export function snapshot(): Editor[] {
  return [...editors];
}

export function fanOutResourceUpdate(update: ResourceUpdate | undefined): void {
  if (!update) return;
  try {
    for (const editor of snapshot()) {
      try {
        editor.receiveResourceUpdate(update);
      } catch (err) {
        console.error('Failed to apply an editor resource update.', err);
      }
    }
  } finally {
    update.free();
  }
}
