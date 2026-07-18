import type { Editor } from '../editor.svelte';
import type { ImeInputAdapter } from './ime-input-adapter';

export const wireImeResyncListener = (
  editor: Editor,
  adapter: ImeInputAdapter,
  getInput: () => HTMLTextAreaElement | null,
): (() => void) => {
  return editor.on('ime_resync_required', () => {
    queueMicrotask(() => {
      adapter.resetForResync(getInput());
    });
  });
};
