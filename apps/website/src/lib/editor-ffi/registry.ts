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
