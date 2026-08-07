import type { AttachmentImportFailureHandler, AttachmentImportItem } from '../attachment-importer';
import type { Editor, EditorContext } from '../editor.svelte';
import type { ImeTextInput } from '../input/ime-context';
import type { EditorEventHandler } from '../types';

export const handleCopy: EditorEventHandler<ImeTextInput, ClipboardEvent> = (editor, e) => {
  if (editor.readOnly && editor.protectContent) {
    e.preventDefault();
    return;
  }

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
  editor.updateNow(() => {
    editor.enqueue({ type: 'clipboard', op: { type: 'cut' } });
    editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
  });
};

export const deferPasteShortcutDuringComposition = (
  e: KeyboardEvent,
  requestPaste: () => void,
  requestPasteTextOnly: () => void,
): (() => void) | undefined => {
  if (!e.isComposing || e.altKey) return;

  const isMac = navigator.platform.toUpperCase().includes('MAC');
  const hasMod = isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
  if (!hasMod || (e.code !== 'KeyV' && e.key.toLowerCase() !== 'v' && e.key !== 'ㅍ')) return;

  // Some IMEs consume the first paste shortcut to commit preedit without emitting
  // a ClipboardEvent. Preserve the native paste-event path outside composition,
  // and recover only this missing event after the composition commit is applied.
  e.preventDefault();
  e.stopPropagation();
  return e.shiftKey ? requestPasteTextOnly : requestPaste;
};

const toImportItem = (file: File): AttachmentImportItem => ({
  file,
  kind: file.type.startsWith('image/') ? 'image' : 'file',
});

const filesFromTransfer = (data: DataTransfer): File[] => {
  const fileItems = [...data.items].filter((item) => item.kind === 'file');
  const files = fileItems.map((item) => item.getAsFile());
  return fileItems.length > 0 && files.every((file): file is File => file !== null) ? files : [...data.files];
};

const readClipboardText = async (items: readonly ClipboardItem[]): Promise<{ html: string | undefined; text: string }> => {
  let html: string | undefined;
  let text = '';
  for (const item of items) {
    if (!html?.trim() && item.types.includes('text/html')) {
      const blob = await item.getType('text/html');
      html = await blob.text();
    }
    if (text === '' && item.types.includes('text/plain')) {
      const blob = await item.getType('text/plain');
      text = await blob.text();
    }
  }
  return { html, text };
};

const scrollAfterPaste = (editor: Editor): void => {
  editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'typewriter' });
};

const paste = (
  ctx: EditorContext,
  {
    html,
    text,
    files,
  }: {
    html: string | undefined;
    text: string;
    files: readonly File[];
  },
  onFailure: AttachmentImportFailureHandler,
): boolean => {
  const editor = ctx.editor;
  if (!editor || editor.readOnly) return false;

  if (html?.trim()) {
    editor.updateNow(() => {
      editor.enqueue({ type: 'clipboard', op: { type: 'paste', html, text } });
      scrollAfterPaste(editor);
    });
    return true;
  }
  if (files.length > 0) {
    const imported = ctx.attachmentImporter.importAtSelection(files.map(toImportItem), { onFailure });
    if (imported) scrollAfterPaste(editor);
    return imported;
  }
  if (text === '') return false;

  editor.updateNow(() => {
    editor.enqueue({ type: 'clipboard', op: { type: 'paste', html: undefined, text } });
    scrollAfterPaste(editor);
  });
  return true;
};

export const handlePaste = (
  ctx: EditorContext,
  e: ClipboardEvent & { currentTarget: ImeTextInput },
  onFailure: AttachmentImportFailureHandler,
): void => {
  const data = e.clipboardData;
  if (!data) return;

  const html = data.getData('text/html') || undefined;
  const text = data.getData('text/plain');
  const files = html?.trim() ? [] : filesFromTransfer(data);
  if (text === '' && !html?.trim() && files.length === 0) return;

  e.preventDefault();
  paste(ctx, { html, text, files }, onFailure);
};

export const requestPaste = async (ctx: EditorContext, onFailure: AttachmentImportFailureHandler): Promise<void> => {
  const currentEditor = ctx.editor;
  if (!currentEditor) return;
  if (currentEditor.readOnly) {
    currentEditor.editBlockedHandler?.();
    return;
  }

  let html: string | undefined;
  let text: string;
  const files: File[] = [];

  try {
    const items = await navigator.clipboard.read();
    ({ html, text } = await readClipboardText(items));

    if (!html?.trim()) {
      for (const item of items) {
        const type = item.types.find((candidate) => !candidate.startsWith('text/'));
        if (type) {
          const blob = await item.getType(type);
          const mimeType = blob.type || type;
          const name = mimeType.startsWith('image/') ? 'clipboard-image' : 'clipboard-file';
          files.push(new File([blob], name, { type: mimeType }));
        }
      }
    }
  } catch {
    html = undefined;
    files.length = 0;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      return;
    }
  }

  if (ctx.editor !== currentEditor || currentEditor.destroyed || currentEditor.readOnly) return;
  paste(ctx, { html, text, files }, onFailure);
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
    return await readClipboardText(items);
  } catch {
    try {
      const text = await navigator.clipboard.readText();
      return { html: undefined, text };
    } catch {
      return undefined;
    }
  }
};
