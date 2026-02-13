import { getContext, setContext } from 'svelte';
import type { Editor } from './editor.svelte';

export class EditorContext {
  editor: Editor = $state(null as unknown as Editor);

  documentId = $state<string | null>(null);
  serverSnapshot = $state<Uint8Array | undefined>();
  serverVersion = $state<string | null>(null);
  serverGeneration = $state<number>(0);

  resetKey = $state(0);
}

const EDITOR_KEY = Symbol('editor');

export function setupEditorContext(): EditorContext {
  const ctx = new EditorContext();
  setContext(EDITOR_KEY, ctx);
  return ctx;
}

export function getEditorContext(): EditorContext {
  return getContext<EditorContext>(EDITOR_KEY);
}
