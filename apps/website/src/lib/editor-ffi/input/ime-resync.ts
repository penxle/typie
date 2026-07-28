import type { Editor } from '../editor.svelte';
import type { ImeInputAdapter } from './ime-input-adapter';

export const wireImeResyncListener = (
  editor: Editor,
  adapter: ImeInputAdapter,
  getInput: () => HTMLTextAreaElement | null,
  onResync?: () => void,
): (() => void) => {
  return editor.on('ime_resync_required', () => {
    onResync?.();
    queueMicrotask(() => {
      adapter.resetForResync(getInput());
    });
  });
};
