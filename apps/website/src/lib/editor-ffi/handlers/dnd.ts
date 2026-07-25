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
  if (!local || dndSuspended(ctx)) return false;

  const reuseNodeId = updateAttachmentDropTarget(ctx, editor, root, clientX, clientY, dataTransfer);
  editor.enqueue({
    type: 'dnd',
    op: { type: 'over', page: local.page, x: local.x, y: local.y, reuse_node_id: reuseNodeId ?? undefined, modifiers },
  });
  editor.flush();
  return true;
};

/**
 * 앞선 드롭이 예약을 기다리는 동안에는 엔진의 DnD 상태를 **한 번도** 건드리지 않는다.
 *
 * `DndState`는 슬롯 하나뿐이고 세션·세대 태그가 없다(`crates/editor-core/src/dnd.rs`). `DndOp::Drop`은
 * 물리적 드롭 시점이 아니라 **op이 처리되는 시점의** `drop_target`에서 삽입 위치를 뽑는다
 * (`handle/dnd.rs`). 예약 왕복을 기다리는 사이 새 드래그가 `enter_external`/`over`로 목표를 옮기면
 * 앞 드롭의 파일이 **새 드래그가 가리키던 위치**에 조용히 떨어지고, 이어서 상태가 `Idle`이 되므로
 * 새 드래그의 정상적인 드롭이 `accepts_payload`에서 거부된다. 엔진은 동결이고 "예약이 문서 변경보다
 * 먼저"는 플랜의 구속 조건이라, 막을 수 있는 곳은 여기(드래그 단계)뿐이다.
 */
const dndSuspended = (ctx: EditorContext): boolean => ctx.attachmentImporter.awaitingDropReservation;

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
  // 내부 드래그 시작도 슬롯을 `InternalDnd`로 갈아 끼워 앞 드롭을 죽인다.
  if (!editor || !dataTransfer || editor.isSelectionCollapsed || dndSuspended(ctx)) {
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

// 참고: 이 핸들러는 (이 태스크 이전에도) `preventDefault`를 부르지 않는다 — 이 앱에서 드롭 대상이
// 성립하는 근거는 전적으로 `handleDragOver`의 취소다. 그래서 유예 중 `enter_external`을 건너뛰어도
// 브라우저의 `drop` 발생 여부에는 영향이 없다.
export const handleDragEnter = (ctx: EditorContext, event: DragEvent) => {
  const editor = ctx.editor;
  const dataTransfer = event.dataTransfer;
  if (!editor || editor.readOnly || !dataTransfer || dndSuspended(ctx) || hasInternalSelectionDrag(editor, dataTransfer)) {
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

  // 유예 중: 엔진 `over`는 보내지 않지만 **preventDefault는 반드시 한다**.
  //
  // HTML DnD 모델에서 `dragover`의 기본 동작을 취소하지 않으면 현재 드래그 작업이 `"none"`으로 되돌아가고,
  // 놓는 순간 브라우저가 `drop`을 **아예 발생시키지 않은 채** 파일을 자기 방식대로 처리한다(탭이 그 파일로
  // 이동 = 열려 있던 문서에서 이탈). 이 앱에는 window/document 레벨 안전망이 없어 아무도 그것을 잡지 못한다.
  // 취소는 `drop_target`을 옮기지 않으므로 게이트의 목적(앞 드롭의 목표 보존)과 충돌하지 않는다.
  // "놓을 수 없음"은 `dropEffect = 'none'`으로만 표현한다.
  if (dndSuspended(ctx)) {
    event.preventDefault();
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
  // 유예 중에는 `leave`도 보내지 않는다 — 앞 드롭의 상태를 Idle로 만들어 그 드롭을 죽인다.
  if (!dndSuspended(ctx)) {
    editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
    editor.flush();
  }
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

  // 앞 드롭이 예약을 기다리는 중이면 이 드롭은 엔진에 **아무것도** 보내지 않는다. 첨부라면 아래
  // `importAtDrop`이 거부하며 항목별로 실패를 알리고, 그 밖의 페이로드는 이번 왕복 동안 버려진다.
  const suspended = dndSuspended(ctx);
  if (!suspended) {
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
  }
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
  if (!payload || suspended) {
    setAttachmentDropTarget(ctx, null);
    setDropEffect(dataTransfer, 'none');
    if (suspended) {
      // dragover를 취소해 둔 드롭이므로 기본 동작도 막는다. 엔진에는 아무것도 보내지 않는다 —
      // 이 왕복 동안 텍스트/내부 선택 드롭은 그냥 버려진다(파일은 위 첨부 분기에서 항목별로 보고된다).
      event.preventDefault();
    } else {
      editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
      editor.flush();
    }
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
  // `end`는 무조건 Idle로 만든다 — 유예 중이면 앞 드롭이 소비될 상태를 지워 버린다.
  if (dndSuspended(ctx)) return;
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
