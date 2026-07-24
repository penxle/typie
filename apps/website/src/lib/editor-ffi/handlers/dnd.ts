import { EditorEdgeAutoScroll } from '../edge-auto-scroll';
import { markNativeSelectionDragStarted } from './pointer';
import type { DndDropPayload, ExternalDndPayloadKind, InputModifiers } from '@typie/editor-ffi/browser';
import type { AttachmentImportFailureHandler, AttachmentImportItem } from '../attachment-importer';
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

const attachmentKind = (type: string): AttachmentImportItem['kind'] => (type.startsWith('image/') ? 'image' : 'file');

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

const setAttachmentDropTarget = (ctx: EditorContext, nodeId: string | null): void => {
  if (ctx.attachmentDropTargetNodeId !== nodeId) {
    ctx.attachmentDropTargetNodeId = nodeId;
  }
};

const externalNodeAtPoint = (editor: EditorInstance, root: EventTarget | null, clientX: number, clientY: number): string | undefined => {
  if (!(root instanceof HTMLElement) || root !== editor.extensionAreaEl) return undefined;
  const element = document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[data-external-element][data-node-id]');
  if (!element || !root.contains(element)) return undefined;
  return element.dataset.nodeId;
};

const hoverAttachmentKinds = (dataTransfer: DataTransfer): AttachmentImportItem['kind'][] | undefined => {
  const fileItems = [...dataTransfer.items].filter((item) => item.kind === 'file');
  if (fileItems.length > 0) {
    if (fileItems.some((item) => item.type === '')) return undefined;
    return fileItems.map((item) => attachmentKind(item.type));
  }

  const files = filesFromTransfer(dataTransfer);
  if (files.some((file) => file.type === '')) return undefined;
  return files.length > 0 ? files.map((file) => attachmentKind(file.type)) : undefined;
};

const reusableAttachmentNode = (
  ctx: EditorContext,
  editor: EditorInstance,
  root: EventTarget | null,
  clientX: number,
  clientY: number,
  kinds: readonly AttachmentImportItem['kind'][],
): { nodeId: string; kind: AttachmentImportItem['kind'] } | undefined => {
  const nodeId = externalNodeAtPoint(editor, root, clientX, clientY);
  if (!nodeId || kinds.length === 0) return undefined;
  if (kinds.every((kind) => kind === 'image') && ctx.attachmentImporter.canReusePlaceholder(nodeId, 'image')) {
    return { nodeId, kind: 'image' };
  }
  return ctx.attachmentImporter.canReusePlaceholder(nodeId, 'file') ? { nodeId, kind: 'file' } : undefined;
};

const updateAttachmentDropTarget = (
  ctx: EditorContext,
  editor: EditorInstance,
  root: EventTarget | null,
  clientX: number,
  clientY: number,
  dataTransfer: DataTransfer,
): string | null => {
  if (hasInternalSelectionDrag(editor, dataTransfer)) {
    setAttachmentDropTarget(ctx, null);
    return null;
  }
  const kinds = hoverAttachmentKinds(dataTransfer);
  const nodeId = kinds ? (reusableAttachmentNode(ctx, editor, root, clientX, clientY, kinds)?.nodeId ?? null) : null;
  setAttachmentDropTarget(ctx, nodeId);
  return nodeId;
};

const attachmentDropIntent = (
  ctx: EditorContext,
  editor: EditorInstance,
  root: EventTarget | null,
  clientX: number,
  clientY: number,
  files: readonly File[],
): { items: AttachmentImportItem[]; reuseNodeId?: string } => {
  const kinds = files.map((file) => attachmentKind(file.type));
  const reuse = reusableAttachmentNode(ctx, editor, root, clientX, clientY, kinds);
  return {
    items: files.map((file) => ({ file, kind: reuse?.kind === 'file' ? 'file' : attachmentKind(file.type) })),
    reuseNodeId: reuse?.nodeId,
  };
};

const dispatchDndOverAtClient = (
  ctx: EditorContext,
  editor: EditorInstance,
  root: EventTarget | null,
  dataTransfer: DataTransfer,
  clientX: number,
  clientY: number,
  modifiers: InputModifiers,
): boolean => {
  const local = editor.clientToLocal(clientX, clientY);
  if (!local) return false;

  const reuseNodeId = updateAttachmentDropTarget(ctx, editor, root, clientX, clientY, dataTransfer);
  editor.enqueue({
    type: 'dnd',
    op: { type: 'over', page: local.page, x: local.x, y: local.y, reuse_node_id: reuseNodeId ?? undefined, modifiers },
  });
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
  setAttachmentDropTarget(ctx, null);
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor || !dataTransfer || editor.isSelectionCollapsed) {
    event.preventDefault();
    return;
  }

  if (editor.gesture.isDoubleTapSelectionDragActive) {
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

  const isReadOnlyTouchDrag = editor.readOnly && editor.gesture.gestureActive;
  const canStartReadOnlyTouchDrag =
    editor.readOnly && editor.gesture.isReadOnlyTouchDragCandidate() && editor.gesture.isReadOnlyTouchDragArmed();

  if (isReadOnlyTouchDrag && !canStartReadOnlyTouchDrag) {
    event.preventDefault();
    return;
  }

  if (canStartReadOnlyTouchDrag) {
    editor.gesture.handleNativeDragStart();
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
  if (!editor || editor.readOnly || !dataTransfer || hasInternalSelectionDrag(editor, dataTransfer)) {
    setAttachmentDropTarget(ctx, null);
    return;
  }

  const payload = externalPayloadKindFromTransfer(dataTransfer);
  if (!payload) {
    setAttachmentDropTarget(ctx, null);
    return;
  }

  editor.enqueue({ type: 'dnd', op: { type: 'enter_external', payload } });
  editor.flush();
};

export const handleDragOver = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  const root = event.currentTarget;
  if (!editor || editor.readOnly || !dataTransfer) {
    setAttachmentDropTarget(ctx, null);
    if (editor) stopDndEdgeAutoScroll(editor);
    return;
  }

  const local = editor.clientToLocal(event.clientX, event.clientY);
  if (!hasTransferablePayload(editor, dataTransfer) || !local) {
    setDropEffect(dataTransfer, 'none');
    setAttachmentDropTarget(ctx, null);
    stopDndEdgeAutoScroll(editor);
    return;
  }

  const modifiers = modifiersFromEvent(event);
  const reuseNodeId = updateAttachmentDropTarget(ctx, editor, root, event.clientX, event.clientY, dataTransfer);
  editor.enqueue({
    type: 'dnd',
    op: { type: 'over', page: local.page, x: local.x, y: local.y, reuse_node_id: reuseNodeId ?? undefined, modifiers },
  });
  editor.flush();
  if (ctx.editor !== editor || editor.destroyed || editor.readOnly) {
    setAttachmentDropTarget(ctx, null);
    stopDndEdgeAutoScroll(editor);
    return;
  }
  event.preventDefault();
  setDropEffect(dataTransfer, dropEffectFromTransfer(editor, dataTransfer, modifiers));
  edgeAutoScrollFor(editor).update(editor, { clientX: event.clientX, clientY: event.clientY }, (clientX, clientY) => {
    if (ctx.editor !== editor || editor.destroyed || editor.readOnly) {
      setAttachmentDropTarget(ctx, null);
      stopDndEdgeAutoScroll(editor);
      return;
    }

    if (!dispatchDndOverAtClient(ctx, editor, root, dataTransfer, clientX, clientY, modifiers)) {
      setAttachmentDropTarget(ctx, null);
    }
  });
};

export const handleDragLeave = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  if (!editor || editor.readOnly) {
    setAttachmentDropTarget(ctx, null);
    return;
  }

  const current = event.currentTarget;
  const related = event.relatedTarget;
  if (current instanceof Node && related instanceof Node && current.contains(related)) {
    return;
  }

  setAttachmentDropTarget(ctx, null);
  editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
  editor.flush();
  stopDndEdgeAutoScroll(editor);
};

export const handleDrop = (ctx: EditorContext, event: DragEvent, onFailure: AttachmentImportFailureHandler) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor) {
    setAttachmentDropTarget(ctx, null);
    return;
  }
  stopDndEdgeAutoScroll(editor);
  if (editor.readOnly || !dataTransfer) {
    setAttachmentDropTarget(ctx, null);
    return;
  }

  const local = editor.clientToLocal(event.clientX, event.clientY);
  if (!local || !hasTransferablePayload(editor, dataTransfer)) {
    setAttachmentDropTarget(ctx, null);
    return;
  }

  const modifiers = modifiersFromEvent(event);
  const files = hasInternalSelectionDrag(editor, dataTransfer) ? [] : filesFromTransfer(dataTransfer);
  const attachmentIntent =
    files.length > 0 ? attachmentDropIntent(ctx, editor, event.currentTarget, event.clientX, event.clientY, files) : undefined;
  editor.enqueue({
    type: 'dnd',
    op: {
      type: 'over',
      page: local.page,
      x: local.x,
      y: local.y,
      reuse_node_id: attachmentIntent?.reuseNodeId,
      modifiers,
    },
  });
  editor.flush();
  if (ctx.editor !== editor || editor.destroyed || editor.readOnly) {
    setAttachmentDropTarget(ctx, null);
    return;
  }
  if (attachmentIntent) {
    const { items, reuseNodeId } = attachmentIntent;
    setAttachmentDropTarget(ctx, null);
    event.preventDefault();
    setDropEffect(dataTransfer, dropEffectFromTransfer(editor, dataTransfer, modifiers));
    ctx.attachmentImporter.importAtDrop(items, {
      page: local.page,
      x: local.x,
      y: local.y,
      modifiers,
      reuseNodeId,
      onFailure,
    });
    editor.endNativeDragAdmission({ restoreFocus: true });
    editor.focus();
    internalDndEditors.delete(editor);
    return;
  }

  const payload = dropPayloadFromTransfer(editor, dataTransfer);
  if (!payload) {
    setAttachmentDropTarget(ctx, null);
    setDropEffect(dataTransfer, 'none');
    editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
    editor.flush();
    internalDndEditors.delete(editor);
    return;
  }

  setAttachmentDropTarget(ctx, null);
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
  setAttachmentDropTarget(ctx, null);
  const editor = ctx.editor;
  if (!editor) return;
  internalDndEditors.delete(editor);
  stopDndEdgeAutoScroll(editor);
  editor.gesture.handleNativeDragEnd();
  editor.endNativeDragAdmission({ restoreFocus: false });
  editor.enqueue({ type: 'dnd', op: { type: 'end' } });
  editor.flush();
};

const dropPayloadFromTransfer = (editor: EditorInstance, dataTransfer: DataTransfer): DndDropPayload | null => {
  if (hasInternalSelectionDrag(editor, dataTransfer)) {
    return { type: 'internal_selection' };
  }

  if (hasText(dataTransfer)) {
    const html = dataTransfer.getData('text/html') || undefined;
    const text = dataTransfer.getData('text/plain');
    return { type: 'text', text, html };
  }

  return null;
};
