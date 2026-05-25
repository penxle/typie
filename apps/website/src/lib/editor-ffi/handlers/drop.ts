import type { EditorContext } from '../editor.svelte';

export const handleDragOver = (ctx: EditorContext, event: DragEvent) => {
  if (!ctx.editor || ctx.editor.readOnly) return;

  const items = [...(event.dataTransfer?.items ?? [])];
  if (items.length === 0 || !items.some((item) => item.kind === 'file')) return;

  event.preventDefault();

  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }
};

export const handleDrop = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  if (!editor || editor.readOnly) return;

  const files = [...(event.dataTransfer?.files ?? [])];
  if (files.length === 0) return;

  event.preventDefault();

  const imageFiles = files.filter((f) => f.type.startsWith('image/'));
  const otherFiles = files.filter((f) => !f.type.startsWith('image/'));

  for (const file of imageFiles) {
    ctx.pendingImageDrops.push(file);
    editor.enqueue({
      type: 'insertion',
      op: { type: 'fragment', fragment: { node: { type: 'image', id: undefined } } },
    });
  }

  for (const file of otherFiles) {
    ctx.pendingFileDrops.push(file);
    editor.enqueue({
      type: 'insertion',
      op: { type: 'fragment', fragment: { node: { type: 'file', id: undefined } } },
    });
  }

  editor.focus();
};
