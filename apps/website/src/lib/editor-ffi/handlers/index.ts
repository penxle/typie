import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

export const handle =
  <E extends Element, T extends Event>(editor: Editor | undefined, handler: EditorEventHandler<E, T>) =>
  (event: T) => {
    if (editor) {
      handler(editor, event as never);
    }
  };
