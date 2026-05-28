import type { Action } from 'svelte/action';
import type { Editor } from '$lib/editor-ffi/editor.svelte';

// eslint-disable-next-line @typescript-eslint/no-empty-function
export const editorZoom: Action<HTMLElement, Editor> = () => {};

export function zoomDiffers(a: number, b: number): boolean {
  return a !== b;
}
