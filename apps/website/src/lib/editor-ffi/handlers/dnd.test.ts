import { beforeEach, describe, expect, it, vi } from 'vitest';
import { handleDragEnd, handleDragEnter, handleDragLeave, handleDragOver, handleDragStart, handleDrop } from './dnd';
import type { Message } from '@typie/editor-ffi/browser';
import type { AttachmentImportItem } from '../attachment-importer';

const edgeAutoScroll = vi.hoisted(() => ({
  onScroll: null as ((clientX: number, clientY: number) => void) | null,
}));

vi.mock('../edge-auto-scroll', () => ({
  EditorEdgeAutoScroll: class {
    update(...args: [unknown, unknown, (clientX: number, clientY: number) => void]) {
      edgeAutoScroll.onScroll = args[2];
    }

    stop() {
      edgeAutoScroll.onScroll = null;
    }
  },
}));

const createFile = (name: string, type: string) => new File(['content'], name, { type });

const createDataTransfer = ({
  files = [],
  items,
  types = [],
  data = {},
}: {
  files?: File[];
  items?: { kind: string; type: string }[];
  types?: string[];
  data?: Record<string, string>;
} = {}) => {
  const resolvedItems = (items ?? files.map((f) => ({ kind: 'file', type: f.type }))).map((item) => ({
    kind: item.kind,
    type: item.type,
  }));
  const store = new Map(Object.entries(data));
  return {
    files,
    items: resolvedItems,
    types,
    dropEffect: 'none',
    effectAllowed: 'uninitialized',
    getData: vi.fn((type: string) => store.get(type) ?? ''),
    setData: vi.fn((type: string, value: string) => store.set(type, value)),
    setDragImage: vi.fn(),
  } as unknown as DataTransfer;
};

const createDragEvent = (dataTransfer: DataTransfer | null = createDataTransfer(), currentTarget?: HTMLElement) => {
  return {
    clientX: 110,
    clientY: 220,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    dataTransfer,
    currentTarget,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as unknown as DragEvent;
};

const createCtx = ({ readOnly = false, protectContent = false } = {}) => {
  const messages: Message[] = [];
  const enqueue = vi.fn((message: Message) => {
    messages.push(message);
  });
  const flush = vi.fn();
  const extensionAreaEl = document.createElement('div');
  const canReusePlaceholder = vi.fn<(nodeId: string, kind: 'image' | 'file') => boolean>(() => false);
  const importAtDrop = vi.fn<
    (
      items: readonly AttachmentImportItem[],
      options: {
        page: number;
        x: number;
        y: number;
        modifiers: { alt: boolean; ctrl: boolean; meta: boolean; shift: boolean };
        reuseNodeId?: string;
        onFailure: (item: AttachmentImportItem) => void;
      },
    ) => boolean
  >(() => true);
  const editor = {
    readOnly,
    protectContent,
    isSelectionCollapsed: false,
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    selectionHitTest: vi.fn(() => true),
    copySelection: vi.fn(() => ({ text: 'Hello', html: '<p>Hello</p>' })),
    endNativeDragAdmission: vi.fn(),
    enqueue,
    flush,
    focus: vi.fn(),
    extensionAreaEl,
    gesture: {
      isDoubleTapSelectionDragActive: false,
      gestureActive: false,
      isReadOnlyTouchDragCandidate: vi.fn(() => false),
      isReadOnlyTouchDragArmed: vi.fn(() => false),
      handleNativeDragStart: vi.fn(),
      handleNativeDragEnd: vi.fn(),
    },
  };
  const attachmentState = {
    editor,
    attachmentDropTargetNodeId: null as string | null,
    attachmentImporter: {
      canReusePlaceholder,
      importAtDrop,
      awaitingDropReservation: false,
    },
  };

  return {
    ctx: attachmentState as never,
    attachmentState,
    editor,
    messages,
    enqueue,
    flush,
    extensionAreaEl,
    canReusePlaceholder,
    importAtDrop,
  };
};

const targetAtPoint = (root: HTMLElement, nodeId: string): HTMLElement => {
  const wrapper = document.createElement('div');
  wrapper.dataset.externalElement = '';
  wrapper.dataset.nodeId = nodeId;
  const child = document.createElement('span');
  wrapper.append(child);
  root.append(wrapper);
  Object.defineProperty(document, 'elementFromPoint', {
    configurable: true,
    value: vi.fn(() => child),
  });
  return wrapper;
};

beforeEach(() => {
  edgeAutoScroll.onScroll = null;
  Object.defineProperty(document, 'elementFromPoint', {
    configurable: true,
    value: vi.fn(() => null),
  });
});

describe('handleDragStart', () => {
  it('starts internal selection drag and exposes html/plain data for external drops', () => {
    const { ctx, messages, editor } = createCtx();
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragStart(ctx, event);

    expect(editor.selectionHitTest).toHaveBeenCalledWith(0, 10, 20);
    expect(dataTransfer.effectAllowed).toBe('copyMove');
    expect(dataTransfer.setData).toHaveBeenCalledWith('application/x-typie-internal-selection', '1');
    expect(dataTransfer.setData).toHaveBeenCalledWith('text/plain', 'Hello');
    expect(dataTransfer.setData).toHaveBeenCalledWith('text/html', '<p>Hello</p>');
    expect(dataTransfer.setDragImage).toHaveBeenCalledWith(expect.any(HTMLImageElement), 0, 0);
    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'start_internal_selection' },
      },
    ]);
    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('allows read-only selection drag as external copy data and clears pending pointer press', () => {
    const { ctx, messages, flush } = createCtx({ readOnly: true });
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragStart(ctx, event);

    expect(dataTransfer.effectAllowed).toBe('copy');
    expect(dataTransfer.setData).not.toHaveBeenCalledWith('application/x-typie-internal-selection', expect.any(String));
    expect(dataTransfer.setData).toHaveBeenCalledWith('text/plain', 'Hello');
    expect(dataTransfer.setData).toHaveBeenCalledWith('text/html', '<p>Hello</p>');
    expect(dataTransfer.setDragImage).toHaveBeenCalledWith(expect.any(HTMLImageElement), 0, 0);
    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'start_internal_selection' },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(1);
    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('보호 문서의 read-only 드래그는 dataTransfer 에 쓰지 않고 차단한다', () => {
    const { ctx } = createCtx({ readOnly: true, protectContent: true });
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragStart(ctx, event);

    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(dataTransfer.setData).not.toHaveBeenCalled();
  });

  it('blocks a read-only touch drag while the gesture is active but not armed', () => {
    const { ctx, editor } = createCtx({ readOnly: true });
    editor.gesture.gestureActive = true;
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragStart(ctx, event);

    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(dataTransfer.setData).not.toHaveBeenCalled();
  });

  it('starts an armed read-only touch drag and notifies the gesture controller', () => {
    const { ctx, editor } = createCtx({ readOnly: true });
    editor.gesture.gestureActive = true;
    editor.gesture.isReadOnlyTouchDragCandidate = vi.fn(() => true);
    editor.gesture.isReadOnlyTouchDragArmed = vi.fn(() => true);
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragStart(ctx, event);

    expect(editor.gesture.handleNativeDragStart).toHaveBeenCalledTimes(1);
    expect(dataTransfer.effectAllowed).toBe('copy');
    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('blocks a native drag while a double-tap selection drag is active', () => {
    const { ctx, editor } = createCtx();
    editor.gesture.isDoubleTapSelectionDragActive = true;
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragStart(ctx, event);

    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(dataTransfer.setData).not.toHaveBeenCalled();
  });
});

describe('handleDragEnter', () => {
  it('starts an external DnD session for external files', () => {
    const { ctx, messages, flush } = createCtx();
    const event = createDragEvent(createDataTransfer({ files: [createFile('image.png', 'image/png')] }));

    handleDragEnter(ctx, event);

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'enter_external', payload: 'image_files' },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(1);
  });

  it('does not start an external session for internal selection drags', () => {
    const { ctx, messages, flush } = createCtx();
    const event = createDragEvent(createDataTransfer({ types: ['application/x-typie-internal-selection'] }));

    handleDragEnter(ctx, event);

    expect(messages).toEqual([]);
    expect(flush).not.toHaveBeenCalled();
  });
});

describe('handleDragOver', () => {
  it('prevents default when a transferable payload has editor-local coordinates', () => {
    const { ctx, messages, flush } = createCtx();
    const dataTransfer = createDataTransfer({ files: [createFile('image.png', 'image/png')] });
    const event = createDragEvent(dataTransfer);

    handleDragOver(ctx, event);

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: undefined,
        },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(1);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(dataTransfer.dropEffect).toBe('copy');
  });

  it('uses move feedback for internal selection drags without entering an external session', () => {
    const { ctx, messages, flush } = createCtx();
    const dataTransfer = createDataTransfer({ types: ['application/x-typie-internal-selection'] });
    const event = createDragEvent(dataTransfer);

    handleDragOver(ctx, event);

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: undefined,
        },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(1);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(dataTransfer.dropEffect).toBe('move');
  });

  it('keeps routing internal drags even when dragover does not expose custom transfer types', () => {
    const { ctx, messages, flush } = createCtx();
    handleDragStart(ctx, createDragEvent(createDataTransfer()));
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragOver(ctx, event);

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'start_internal_selection' },
      },
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: undefined,
        },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(2);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(dataTransfer.dropEffect).toBe('move');
  });

  it('does not accept browser drop when there is no transferable payload', () => {
    const { ctx, messages, flush } = createCtx();
    const dataTransfer = createDataTransfer();
    const event = createDragEvent(dataTransfer);

    handleDragOver(ctx, event);

    expect(messages).toEqual([]);
    expect(flush).not.toHaveBeenCalled();
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(dataTransfer.dropEffect).toBe('none');
  });

  it('re-hit-tests the attachment candidate at the coordinates reported by edge auto-scroll', () => {
    const { ctx, attachmentState, editor, extensionAreaEl, canReusePlaceholder, messages } = createCtx();
    const first = document.createElement('div');
    first.dataset.externalElement = '';
    first.dataset.nodeId = 'first-node';
    const firstChild = document.createElement('span');
    first.append(firstChild);
    const second = document.createElement('div');
    second.dataset.externalElement = '';
    second.dataset.nodeId = 'second-node';
    const secondChild = document.createElement('span');
    second.append(secondChild);
    extensionAreaEl.append(first, second);
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: vi.fn((clientX: number) => (clientX === 150 ? secondChild : firstChild)),
    });
    canReusePlaceholder.mockImplementation((...args) => args[1] === 'image');
    const event = createDragEvent(createDataTransfer({ files: [createFile('image.png', 'image/png')] }), extensionAreaEl);

    handleDragOver(ctx, event);
    expect(attachmentState.attachmentDropTargetNodeId).toBe('first-node');

    edgeAutoScroll.onScroll?.(150, 230);

    expect(editor.clientToLocal).toHaveBeenLastCalledWith(150, 230);
    expect(document.elementFromPoint).toHaveBeenLastCalledWith(150, 230);
    expect(attachmentState.attachmentDropTargetNodeId).toBe('second-node');
    expect(messages.filter((message) => message.type === 'dnd' && message.op.type === 'over')).toEqual([
      expect.objectContaining({ op: expect.objectContaining({ reuse_node_id: 'first-node' }) }),
      expect.objectContaining({ op: expect.objectContaining({ reuse_node_id: 'second-node' }) }),
    ]);
  });
});

describe('handleDragLeave', () => {
  it('routes dragleave during an internal drag so the engine can clear only the active target', () => {
    const { ctx, messages, flush } = createCtx();
    handleDragStart(ctx, createDragEvent(createDataTransfer()));
    const event = {
      ...createDragEvent(createDataTransfer()),
      currentTarget: document.createElement('div'),
      relatedTarget: null,
    } as unknown as DragEvent;

    handleDragLeave(ctx, event);

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'start_internal_selection' },
      },
      {
        type: 'dnd',
        op: { type: 'leave' },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(2);
  });
});

describe('external attachment target', () => {
  it('highlights and reuses an empty image placeholder only for image-only input', () => {
    const { ctx, attachmentState, extensionAreaEl, canReusePlaceholder, importAtDrop, messages } = createCtx();
    targetAtPoint(extensionAreaEl, 'image-node');
    canReusePlaceholder.mockImplementation((nodeId, kind) => nodeId === 'image-node' && kind === 'image');
    const png = createFile('image.png', 'image/png');
    const dataTransfer = createDataTransfer({ files: [png] });

    handleDragOver(ctx, createDragEvent(dataTransfer, extensionAreaEl));

    expect(attachmentState.attachmentDropTargetNodeId).toBe('image-node');
    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: 'image-node',
        },
      },
    ]);

    handleDrop(ctx, createDragEvent(dataTransfer, extensionAreaEl), vi.fn());

    expect(importAtDrop).toHaveBeenCalledWith([{ file: png, kind: 'image' }], expect.objectContaining({ reuseNodeId: 'image-node' }));
    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();
  });

  it('treats every item as File when reusing an empty file placeholder', () => {
    const { ctx, extensionAreaEl, canReusePlaceholder, importAtDrop } = createCtx();
    targetAtPoint(extensionAreaEl, 'file-node');
    canReusePlaceholder.mockImplementation((nodeId, kind) => nodeId === 'file-node' && kind === 'file');
    const png = createFile('image.png', 'image/png');
    const pdf = createFile('document.pdf', 'application/pdf');
    const dataTransfer = createDataTransfer({ files: [png, pdf] });

    handleDragOver(ctx, createDragEvent(dataTransfer, extensionAreaEl));
    handleDrop(ctx, createDragEvent(dataTransfer, extensionAreaEl), vi.fn());

    expect(importAtDrop).toHaveBeenCalledWith(
      [
        { file: png, kind: 'file' },
        { file: pdf, kind: 'file' },
      ],
      expect.objectContaining({ reuseNodeId: 'file-node' }),
    );
  });

  it('falls back to generic ordered kinds for mixed input over an image placeholder', () => {
    const { ctx, attachmentState, extensionAreaEl, canReusePlaceholder, importAtDrop, messages } = createCtx();
    targetAtPoint(extensionAreaEl, 'image-node');
    canReusePlaceholder.mockImplementation((nodeId, kind) => nodeId === 'image-node' && kind === 'image');
    const png = createFile('image.png', 'image/png');
    const pdf = createFile('document.pdf', 'application/pdf');
    const dataTransfer = createDataTransfer({ files: [png, pdf] });

    handleDragOver(ctx, createDragEvent(dataTransfer, extensionAreaEl));

    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();
    expect(messages.at(-1)).toEqual({
      type: 'dnd',
      op: {
        type: 'over',
        page: 0,
        x: 10,
        y: 20,
        modifiers: { alt: false, ctrl: false, meta: false, shift: false },
        reuse_node_id: undefined,
      },
    });

    handleDrop(ctx, createDragEvent(dataTransfer, extensionAreaEl), vi.fn());

    expect(importAtDrop).toHaveBeenCalledWith(
      [
        { file: png, kind: 'image' },
        { file: pdf, kind: 'file' },
      ],
      expect.objectContaining({ reuseNodeId: undefined }),
    );
  });

  it('does not propose a candidate outside the current editor root', () => {
    const { ctx, attachmentState, extensionAreaEl, canReusePlaceholder } = createCtx();
    const otherRoot = document.createElement('div');
    targetAtPoint(otherRoot, 'foreign-node');
    canReusePlaceholder.mockReturnValue(true);

    handleDragOver(ctx, createDragEvent(createDataTransfer({ files: [createFile('image.png', 'image/png')] }), extensionAreaEl));

    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();
    expect(canReusePlaceholder).not.toHaveBeenCalled();
  });

  it('clears the affordance on unsupported over, outer leave, drop, and drag end', () => {
    const { ctx, attachmentState, extensionAreaEl } = createCtx();

    attachmentState.attachmentDropTargetNodeId = 'node';
    handleDragOver(ctx, createDragEvent(createDataTransfer(), extensionAreaEl));
    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();

    attachmentState.attachmentDropTargetNodeId = 'node';
    handleDragLeave(ctx, {
      ...createDragEvent(createDataTransfer(), extensionAreaEl),
      relatedTarget: null,
    } as unknown as DragEvent);
    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();

    attachmentState.attachmentDropTargetNodeId = 'node';
    handleDrop(ctx, createDragEvent(createDataTransfer({ files: [createFile('image.png', 'image/png')] }), extensionAreaEl), vi.fn());
    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();

    attachmentState.attachmentDropTargetNodeId = 'node';
    handleDragEnd(ctx);
    expect(attachmentState.attachmentDropTargetNodeId).toBeNull();
  });
});

describe('handleDrop', () => {
  it('preserves ordered files and dispatches only after the final Over flush', () => {
    const { ctx, messages, flush, importAtDrop } = createCtx();
    const png = createFile('image.png', 'image/png');
    const jpeg = createFile('photo.jpg', 'image/jpeg');
    const pdf = createFile('doc.pdf', 'application/pdf');
    const onFailure = vi.fn();
    const event = createDragEvent(createDataTransfer({ files: [png, pdf, jpeg] }));

    handleDrop(ctx, event, onFailure);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: undefined,
        },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(1);
    expect(importAtDrop).toHaveBeenCalledWith(
      [
        { file: png, kind: 'image' },
        { file: pdf, kind: 'file' },
        { file: jpeg, kind: 'image' },
      ],
      {
        page: 0,
        x: 10,
        y: 20,
        modifiers: { alt: false, ctrl: false, meta: false, shift: false },
        reuseNodeId: undefined,
        onFailure,
      },
    );
    expect(flush.mock.invocationCallOrder[0]).toBeLessThan(importAtDrop.mock.invocationCallOrder[0] ?? 0);
  });

  it('routes text/html payload through DndOp::Drop', () => {
    const { ctx, messages } = createCtx();
    const event = createDragEvent(
      createDataTransfer({
        types: ['text/html', 'text/plain'],
        data: { 'text/html': '<p>Hello</p>', 'text/plain': 'Hello' },
      }),
    );

    handleDrop(ctx, event, vi.fn());

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: undefined,
        },
      },
      {
        type: 'dnd',
        op: {
          type: 'drop',
          page: 0,
          x: 10,
          y: 20,
          payload: { type: 'text', text: 'Hello', html: '<p>Hello</p>' },
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
        },
      },
    ]);
  });

  it('routes internal drops from the active internal drag session even when drop does not expose custom transfer types', () => {
    const { ctx, messages } = createCtx();
    handleDragStart(ctx, createDragEvent(createDataTransfer()));

    handleDrop(ctx, createDragEvent(createDataTransfer()), vi.fn());

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'start_internal_selection' },
      },
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
          reuse_node_id: undefined,
        },
      },
      {
        type: 'dnd',
        op: {
          type: 'drop',
          page: 0,
          x: 10,
          y: 20,
          payload: { type: 'internal_selection' },
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
        },
      },
    ]);
  });
});

describe('drop awaiting its reservation', () => {
  // 엔진의 `DndState`는 슬롯 하나뿐이고 `Drop`은 **처리 시점의** drop_target을 읽는다. 예약 왕복 동안
  // 새 드래그가 목표를 옮기면 앞 드롭의 파일이 그 위치에 조용히 떨어지므로, 드래그 단계 전체를 막는다.
  const dropFiles = (harness: ReturnType<typeof createCtx>) => {
    const dataTransfer = createDataTransfer({ files: [createFile('a.png', 'image/png')], types: ['Files'] });
    handleDrop(harness.ctx, createDragEvent(dataTransfer, harness.extensionAreaEl), vi.fn());
    harness.attachmentState.attachmentImporter.awaitingDropReservation = harness.importAtDrop.mock.calls.length > 0;
  };

  it('sends no dnd op for a new drag while an earlier drop awaits its reservation', () => {
    const harness = createCtx();
    dropFiles(harness);
    expect(harness.importAtDrop).toHaveBeenCalledOnce();
    const afterFirstDrop = [...harness.messages];
    expect(afterFirstDrop.at(-1)).toMatchObject({ type: 'dnd', op: { type: 'over', x: 10, y: 20 } });

    const second = createDataTransfer({ files: [createFile('b.png', 'image/png')], types: ['Files'] });
    handleDragEnter(harness.ctx, createDragEvent(second, harness.extensionAreaEl));
    const overEvent = createDragEvent(second, harness.extensionAreaEl);
    handleDragOver(harness.ctx, overEvent);
    handleDragLeave(harness.ctx, createDragEvent(second, harness.extensionAreaEl));
    handleDragEnd(harness.ctx);
    handleDragStart(harness.ctx, createDragEvent(second, harness.extensionAreaEl));

    // enter/over/leave/end/내부 드래그 시작 중 어느 것도 엔진에 닿지 않는다 →
    // 앞 드롭이 소비할 drop_target은 여전히 **자기 것**이다.
    expect(harness.messages).toEqual(afterFirstDrop);
  });

  it('still cancels dragover while suspended so the browser fires drop instead of opening the file', () => {
    const harness = createCtx();
    dropFiles(harness);
    const afterFirstDrop = [...harness.messages];

    const second = createDataTransfer({ files: [createFile('b.png', 'image/png')], types: ['Files'] });
    const overEvent = createDragEvent(second, harness.extensionAreaEl);
    handleDragOver(harness.ctx, overEvent);

    // dragover의 기본 동작을 막지 않으면 브라우저는 `drop`을 발생시키지 않고 그 파일로 탭을 이동시킨다
    // (window/document 레벨 안전망이 이 앱에는 없다) → 열려 있던 문서를 잃는다.
    expect(overEvent.preventDefault).toHaveBeenCalled();
    // 대신 "놓을 수 없음"은 dropEffect로만 표현하고, 엔진 상태는 그대로 둔다.
    expect(second.dropEffect).toBe('none');
    expect(harness.messages).toEqual(afterFirstDrop);
  });

  it('cancels dragover and advertises a real drop effect once the reservation settles', () => {
    const harness = createCtx();
    dropFiles(harness);
    harness.attachmentState.attachmentImporter.awaitingDropReservation = false;

    const second = createDataTransfer({ files: [createFile('b.png', 'image/png')], types: ['Files'] });
    const overEvent = createDragEvent(second, harness.extensionAreaEl);
    handleDragOver(harness.ctx, overEvent);

    expect(overEvent.preventDefault).toHaveBeenCalled();
    expect(second.dropEffect).toBe('copy');
  });

  it('rejects a second file drop without touching the engine and lets the importer report it', () => {
    const harness = createCtx();
    dropFiles(harness);
    const afterFirstDrop = [...harness.messages];
    harness.importAtDrop.mockReturnValue(false);

    const onFailure = vi.fn();
    const second = createDataTransfer({ files: [createFile('b.png', 'image/png')], types: ['Files'] });
    const dropEvent = createDragEvent(second, harness.extensionAreaEl);
    handleDrop(harness.ctx, dropEvent, onFailure);

    // 파일 드롭은 실제로 도착해야 하고(브라우저가 삼키면 안 된다), 항목별 실패 보고는 importer가 한다.
    expect(harness.messages).toEqual(afterFirstDrop);
    expect(dropEvent.preventDefault).toHaveBeenCalled();
    expect(harness.importAtDrop).toHaveBeenCalledTimes(2);
    expect(harness.importAtDrop.mock.calls[1]?.[0].map((item) => item.file.name)).toEqual(['b.png']);
  });

  it('discards a text payload for the round trip instead of consuming the pending drop target', () => {
    const harness = createCtx();
    dropFiles(harness);
    const afterFirstDrop = [...harness.messages];

    const text = createDataTransfer({ types: ['text/plain'], data: { 'text/plain': 'hello' } });
    const dropEvent = createDragEvent(text, harness.extensionAreaEl);
    handleDrop(harness.ctx, dropEvent, vi.fn());

    expect(harness.messages).toEqual(afterFirstDrop);
    expect(dropEvent.preventDefault).toHaveBeenCalled();
  });

  it('resumes normal drag handling once the reservation settles', () => {
    const harness = createCtx();
    dropFiles(harness);
    harness.attachmentState.attachmentImporter.awaitingDropReservation = false;

    const second = createDataTransfer({ files: [createFile('b.png', 'image/png')], types: ['Files'] });
    handleDragEnter(harness.ctx, createDragEvent(second, harness.extensionAreaEl));
    const overEvent = createDragEvent(second, harness.extensionAreaEl);
    handleDragOver(harness.ctx, overEvent);

    expect(harness.messages.at(-2)).toMatchObject({ type: 'dnd', op: { type: 'enter_external' } });
    expect(harness.messages.at(-1)).toMatchObject({ type: 'dnd', op: { type: 'over' } });
    expect(overEvent.preventDefault).toHaveBeenCalled();
  });
});
