import { getContext, setContext } from 'svelte';
import type { Editor } from './editor.svelte';

const EDITOR_KEY = Symbol('editor');

export function setEditor(editor: Editor): void {
  setContext(EDITOR_KEY, editor);
}

export function getEditor(): Editor {
  const editor = getContext<Editor>(EDITOR_KEY);
  if (!editor) {
    throw new Error('Editor not found. Make sure to call setEditor in a parent component.');
  }
  return editor;
}
