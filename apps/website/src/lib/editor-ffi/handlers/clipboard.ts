import type { EditorEventHandler } from '../types';

export const handlePaste: EditorEventHandler<HTMLInputElement, ClipboardEvent> = (editor, e) => {
  const imageFiles = [...(e.clipboardData?.files ?? [])].filter((file) => file.type.startsWith('image/'));
  if (imageFiles.length > 0) {
    e.preventDefault();
    editor.insertImagesFromFiles(imageFiles);
    return;
  }

  const text = e.clipboardData?.getData('text/plain') || undefined;
  const html = e.clipboardData?.getData('text/html') || undefined;

  if (!text) {
    return;
  }

  e.preventDefault();
  editor.enqueue({ type: 'clipboard', op: { type: 'paste', text, html } });
};
