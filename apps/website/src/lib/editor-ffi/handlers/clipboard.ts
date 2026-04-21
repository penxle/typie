import type { EditorEventHandler } from '../types';

export const handlePaste: EditorEventHandler<HTMLInputElement, ClipboardEvent> = (editor, e) => {
  const text = e.clipboardData?.getData('text/plain') || undefined;
  const html = e.clipboardData?.getData('text/html') || undefined;

  if (!text) {
    return;
  }

  e.preventDefault();
  editor.enqueue({ type: 'clipboard', op: { type: 'paste', text, html } });
};
