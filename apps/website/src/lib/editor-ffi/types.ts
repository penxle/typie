import type { EditorEvent } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

export type EditorEventValue<T extends EditorEvent> = T extends { value: infer V } ? V : never;
export type EditorEventListener<T extends EditorEvent> = T extends { value: infer V }
  ? (editor: Editor, value: V) => void
  : (editor: Editor) => void;
export type EditorEventHandler<K extends EditorEvent['type']> =
  Extract<EditorEvent, { type: K }> extends { value: infer V } ? (editor: Editor, value: V) => void : (editor: Editor) => void;
