import type { EditorEventHandler } from '../types';

export const handleCopy: EditorEventHandler<HTMLInputElement, ClipboardEvent> = (editor, e) => {
  const payload = editor.copySelection();
  if (!payload) {
    return;
  }

  if (!e.clipboardData) {
    return;
  }

  try {
    e.clipboardData.setData('text/html', payload.html);
    e.clipboardData.setData('text/plain', payload.text);
  } catch {
    return;
  }
  e.preventDefault();
};

export const handleCut: EditorEventHandler<HTMLInputElement, ClipboardEvent> = (editor, e) => {
  const payload = editor.copySelection();
  if (!payload) {
    return;
  }

  if (!e.clipboardData) {
    return;
  }

  try {
    e.clipboardData.setData('text/html', payload.html);
    e.clipboardData.setData('text/plain', payload.text);
  } catch {
    return;
  }
  e.preventDefault();
  editor.enqueue({ type: 'clipboard', op: { type: 'cut' } });
};

export const handlePaste: EditorEventHandler<HTMLInputElement, ClipboardEvent> = (editor, e) => {
  const text = e.clipboardData?.getData('text/plain') || undefined;
  const html = e.clipboardData?.getData('text/html') || undefined;

  if (!text) {
    return;
  }

  e.preventDefault();
  editor.enqueue({ type: 'clipboard', op: { type: 'paste', text, html } });
};
