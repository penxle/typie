import type { ImeTextInput } from '../input/ime-context';
import type { EditorEventHandler } from '../types';

export const handleCopy: EditorEventHandler<ImeTextInput, ClipboardEvent> = (editor, e) => {
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

export const handleCut: EditorEventHandler<ImeTextInput, ClipboardEvent> = (editor, e) => {
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

export const handlePaste: EditorEventHandler<ImeTextInput, ClipboardEvent> = (editor, e) => {
  const text = e.clipboardData?.getData('text/plain') ?? '';
  const html = e.clipboardData?.getData('text/html') || undefined;

  if (!text && !html) {
    return;
  }

  e.preventDefault();
  editor.enqueue({ type: 'clipboard', op: { type: 'paste', text, html } });
};

export const writeClipboardPayload = async (html: string, text: string): Promise<void> => {
  try {
    const item = new ClipboardItem({
      'text/html': new Blob([html], { type: 'text/html' }),
      'text/plain': new Blob([text], { type: 'text/plain' }),
    });
    await navigator.clipboard.write([item]);
  } catch {
    // ClipboardItem can fail (Firefox doesn't support it, or permission denied); fall back to plain-text write.
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // user denied permission; silent failure
    }
  }
};

export const readClipboardRich = async (): Promise<{ html: string | undefined; text: string } | undefined> => {
  try {
    const items = await navigator.clipboard.read();
    let html: string | undefined;
    let text = '';
    for (const item of items) {
      if (item.types.includes('text/html')) {
        const blob = await item.getType('text/html');
        html = await blob.text();
      }
      if (item.types.includes('text/plain')) {
        const blob = await item.getType('text/plain');
        text = await blob.text();
      }
    }
    return { html, text };
  } catch {
    try {
      const text = await navigator.clipboard.readText();
      return { html: undefined, text };
    } catch {
      return undefined;
    }
  }
};
