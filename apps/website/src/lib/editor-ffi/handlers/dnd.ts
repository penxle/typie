import { EditorEdgeAutoScroll } from '../edge-auto-scroll';
import { markNativeSelectionDragStarted } from './pointer';
import type { DndDropPayload, ExternalDndPayloadKind, InputModifiers } from '@typie/editor-ffi/browser';
import type { EditorContext } from '../editor.svelte';

const INTERNAL_SELECTION_MIME = 'application/x-typie-internal-selection';

type BrowserDropEffect = 'copy' | 'move' | 'none';
type EditorInstance = NonNullable<EditorContext['editor']>;

const internalDndEditors = new WeakSet<EditorInstance>();
const dndEdgeAutoScrolls = new WeakMap<EditorInstance, EditorEdgeAutoScroll>();

let EMPTY_DRAG_IMAGE: HTMLImageElement | null = null;
const setEmptyDragImage = (dataTransfer: DataTransfer): void => {
  if (!EMPTY_DRAG_IMAGE && typeof Image !== 'undefined') {
    EMPTY_DRAG_IMAGE = new Image();
    EMPTY_DRAG_IMAGE.src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
  }

  if (EMPTY_DRAG_IMAGE) {
    dataTransfer.setDragImage(EMPTY_DRAG_IMAGE, 0, 0);
  }
};

const edgeAutoScrollFor = (editor: EditorInstance): EditorEdgeAutoScroll => {
  let edgeAutoScroll = dndEdgeAutoScrolls.get(editor);
  if (!edgeAutoScroll) {
    edgeAutoScroll = new EditorEdgeAutoScroll();
    dndEdgeAutoScrolls.set(editor, edgeAutoScroll);
  }
  return edgeAutoScroll;
};

const stopDndEdgeAutoScroll = (editor: EditorInstance): void => {
  dndEdgeAutoScrolls.get(editor)?.stop();
};

const modifiersFromEvent = (event: DragEvent): InputModifiers => ({
  alt: event.altKey,
  ctrl: event.ctrlKey,
  meta: event.metaKey,
  shift: event.shiftKey,
});

const filesFromTransfer = (dataTransfer: DataTransfer): File[] => [...dataTransfer.files];

const hasInternalSelection = (dataTransfer: DataTransfer): boolean => {
  return [...dataTransfer.types].includes(INTERNAL_SELECTION_MIME);
};

const hasInternalSelectionDrag = (editor: EditorInstance, dataTransfer: DataTransfer): boolean => {
  return internalDndEditors.has(editor) || hasInternalSelection(dataTransfer);
};

const hasText = (dataTransfer: DataTransfer): boolean => {
  const types = new Set(dataTransfer.types);
  return types.has('text/html') || types.has('text/plain');
};

const externalPayloadKindFromTransfer = (dataTransfer: DataTransfer): ExternalDndPayloadKind | null => {
  if (hasInternalSelection(dataTransfer)) {
    return null;
  }

  const items = [...dataTransfer.items];
  const fileItems = items.filter((item) => item.kind === 'file');
  if (fileItems.length > 0) {
    const imageCount = fileItems.filter((item) => item.type.startsWith('image/')).length;
    if (imageCount === fileItems.length) return 'image_files';
    if (imageCount === 0) return 'files';
    return 'mixed_files';
  }

  const files = filesFromTransfer(dataTransfer);
  if (files.length > 0) {
    const imageCount = files.filter((file) => file.type.startsWith('image/')).length;
    if (imageCount === files.length) return 'image_files';
    if (imageCount === 0) return 'files';
    return 'mixed_files';
  }

  if ([...dataTransfer.types].includes('text/html')) {
    return 'html';
  }
  if ([...dataTransfer.types].includes('text/plain')) {
    return 'text';
  }

  return null;
};

const setDropEffect = (dataTransfer: DataTransfer | null, effect: BrowserDropEffect) => {
  if (dataTransfer) {
    dataTransfer.dropEffect = effect;
  }
};

const dispatchDndOverAtClient = (editor: EditorInstance, clientX: number, clientY: number, modifiers: InputModifiers): boolean => {
  const local = editor.clientToLocal(clientX, clientY);
  if (!local) return false;

  editor.enqueue({ type: 'dnd', op: { type: 'over', page: local.page, x: local.x, y: local.y, modifiers } });
  editor.flush();
  return true;
};

const hasTransferablePayload = (editor: EditorInstance, dataTransfer: DataTransfer): boolean => {
  return hasInternalSelectionDrag(editor, dataTransfer) || externalPayloadKindFromTransfer(dataTransfer) !== null;
};

const dropEffectFromTransfer = (editor: EditorInstance, dataTransfer: DataTransfer, modifiers: InputModifiers): BrowserDropEffect => {
  return hasInternalSelectionDrag(editor, dataTransfer) && !modifiers.alt ? 'move' : 'copy';
};

export const handleDragStart = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor || !dataTransfer || editor.isSelectionCollapsed) {
    event.preventDefault();
    return;
  }

  if (editor.readOnly && editor.protectContent) {
    event.preventDefault();
    return;
  }

  const local = editor.clientToLocal(event.clientX, event.clientY);
  if (!local || !editor.selectionHitTest(local.page, local.x, local.y)) {
    event.preventDefault();
    return;
  }

  const payload = editor.copySelection();
  if (!payload) {
    event.preventDefault();
    return;
  }

  setEmptyDragImage(dataTransfer);

  if (editor.readOnly) {
    markNativeSelectionDragStarted(editor);
    dataTransfer.effectAllowed = 'copy';
    dataTransfer.setData('text/plain', payload.text);
    dataTransfer.setData('text/html', payload.html);
    editor.enqueue({ type: 'dnd', op: { type: 'start_internal_selection' } });
    editor.flush();
    return;
  }

  internalDndEditors.add(editor);
  markNativeSelectionDragStarted(editor);
  dataTransfer.effectAllowed = 'copyMove';
  dataTransfer.setData(INTERNAL_SELECTION_MIME, '1');
  dataTransfer.setData('text/plain', payload.text);
  dataTransfer.setData('text/html', payload.html);
  editor.enqueue({ type: 'dnd', op: { type: 'start_internal_selection' } });
  editor.flush();
};

export const handleDragEnter = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor || editor.readOnly || !dataTransfer || hasInternalSelectionDrag(editor, dataTransfer)) return;

  const payload = externalPayloadKindFromTransfer(dataTransfer);
  if (!payload) return;

  editor.enqueue({ type: 'dnd', op: { type: 'enter_external', payload } });
  editor.flush();
};

export const handleDragOver = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor || editor.readOnly || !dataTransfer) return;

  const local = editor.clientToLocal(event.clientX, event.clientY);
  if (!hasTransferablePayload(editor, dataTransfer) || !local) {
    setDropEffect(dataTransfer, 'none');
    stopDndEdgeAutoScroll(editor);
    return;
  }

  const modifiers = modifiersFromEvent(event);
  editor.enqueue({ type: 'dnd', op: { type: 'over', page: local.page, x: local.x, y: local.y, modifiers } });
  editor.flush();
  event.preventDefault();
  setDropEffect(dataTransfer, dropEffectFromTransfer(editor, dataTransfer, modifiers));
  edgeAutoScrollFor(editor).update(editor, { clientX: event.clientX, clientY: event.clientY }, (clientX, clientY) => {
    if (editor.destroyed) {
      stopDndEdgeAutoScroll(editor);
      return;
    }

    dispatchDndOverAtClient(editor, clientX, clientY, modifiers);
  });
};

export const handleDragLeave = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  if (!editor || editor.readOnly) return;

  const current = event.currentTarget;
  const related = event.relatedTarget;
  if (current instanceof Node && related instanceof Node && current.contains(related)) {
    return;
  }

  editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
  editor.flush();
  stopDndEdgeAutoScroll(editor);
};

export const handleDrop = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor) return;
  stopDndEdgeAutoScroll(editor);
  if (editor.readOnly || !dataTransfer) return;

  const local = editor.clientToLocal(event.clientX, event.clientY);
  if (!local || !hasTransferablePayload(editor, dataTransfer)) {
    return;
  }

  const modifiers = modifiersFromEvent(event);
  editor.enqueue({ type: 'dnd', op: { type: 'over', page: local.page, x: local.x, y: local.y, modifiers } });
  editor.flush();
  const payload = dropPayloadFromTransfer(ctx, editor, dataTransfer);
  if (!payload) {
    setDropEffect(dataTransfer, 'none');
    editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
    editor.flush();
    internalDndEditors.delete(editor);
    return;
  }

  event.preventDefault();
  setDropEffect(dataTransfer, dropEffectFromTransfer(editor, dataTransfer, modifiers));
  editor.enqueue({
    type: 'dnd',
    op: {
      type: 'drop',
      page: local.page,
      x: local.x,
      y: local.y,
      payload,
      modifiers,
    },
  });
  editor.flush();
  editor.endNativeDragAdmission({ restoreFocus: true });
  editor.focus();
  internalDndEditors.delete(editor);
};

export const handleDragEnd = (ctx: EditorContext) => {
  const editor = ctx.editor;
  if (!editor) return;
  internalDndEditors.delete(editor);
  stopDndEdgeAutoScroll(editor);
  editor.endNativeDragAdmission({ restoreFocus: false });
  editor.enqueue({ type: 'dnd', op: { type: 'end' } });
  editor.flush();
};

const dropPayloadFromTransfer = (ctx: EditorContext, editor: EditorInstance, dataTransfer: DataTransfer): DndDropPayload | null => {
  if (hasInternalSelectionDrag(editor, dataTransfer)) {
    return { type: 'internal_selection' };
  }

  const files = filesFromTransfer(dataTransfer);
  if (files.length > 0) {
    const imageFiles = files.filter((file) => file.type.startsWith('image/'));
    const otherFiles = files.filter((file) => !file.type.startsWith('image/'));
    ctx.pendingImageDrops.push(...imageFiles);
    ctx.pendingFileDrops.push(...otherFiles);
    return {
      type: 'files',
      image_count: imageFiles.length,
      file_count: otherFiles.length,
    };
  }

  if (hasText(dataTransfer)) {
    const html = dataTransfer.getData('text/html') || undefined;
    const text = dataTransfer.getData('text/plain');
    return { type: 'text', text, html };
  }

  return null;
};

export const isAcceptedImagePlaceholderDrag = (dataTransfer: DataTransfer | null): boolean => {
  if (!dataTransfer) return false;
  const fileItems = [...dataTransfer.items].filter((item) => item.kind === 'file');
  if (fileItems.length > 0) {
    return fileItems.every((item) => item.type.startsWith('image/'));
  }

  const files = filesFromTransfer(dataTransfer);
  return files.length > 0 && files.every((file) => file.type.startsWith('image/'));
};

export const isAcceptedFilePlaceholderDrag = (dataTransfer: DataTransfer | null): boolean => {
  if (!dataTransfer) return false;
  const fileItems = [...dataTransfer.items].filter((item) => item.kind === 'file');
  if (fileItems.length > 0) {
    return true;
  }

  const files = filesFromTransfer(dataTransfer);
  return files.length > 0;
};
