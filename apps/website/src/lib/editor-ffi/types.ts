import type { EditorEvent } from '@typie/editor-ffi/browser';
import type { Editor } from './editor.svelte';

export type EditorEventListener<K extends EditorEvent['type']> = (editor: Editor, event: Extract<EditorEvent, { type: K }>) => void;

export type EditorEventHandler<E extends Element, T extends Event> = (editor: Editor, event: T & { currentTarget: E }) => void;
