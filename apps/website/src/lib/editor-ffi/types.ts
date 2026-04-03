import type { EditorEvent } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

export type EditorEventListener<K extends EditorEvent['type']> =
  Extract<EditorEvent, { type: K }> extends { value: infer V } ? (editor: Editor, value: V) => void : (editor: Editor) => void;

export type EditorEventHandler<E extends Element, T extends Event> = (editor: Editor, event: T & { currentTarget: E }) => void;
