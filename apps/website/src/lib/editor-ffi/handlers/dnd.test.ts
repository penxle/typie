import { describe, expect, it, vi } from 'vitest';
import {
  handleDragEnter,
  handleDragLeave,
  handleDragOver,
  handleDragStart,
  handleDrop,
  isAcceptedFilePlaceholderDrag,
  isAcceptedImagePlaceholderDrag,
} from './dnd';
import type { Message } from '@typie/editor-ffi/browser';

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
  } as unknown as DataTransfer;
};

const createDragEvent = (dataTransfer: DataTransfer | null = createDataTransfer()) => {
  return {
    clientX: 110,
    clientY: 220,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    dataTransfer,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as unknown as DragEvent;
};

const createCtx = ({ readOnly = false } = {}) => {
  const messages: Message[] = [];
  const enqueue = vi.fn((message: Message) => {
    messages.push(message);
  });
  const flush = vi.fn();
  const pendingImageDrops: File[] = [];
  const pendingFileDrops: File[] = [];
  const editor = {
    readOnly,
    isSelectionCollapsed: false,
    clientToLocal: vi.fn(() => ({ page: 0, x: 10, y: 20 })),
    selectionHitTest: vi.fn(() => true),
    copySelection: vi.fn(() => ({ text: 'Hello', html: '<p>Hello</p>' })),
    endNativeDragAdmission: vi.fn(),
    enqueue,
    flush,
    focus: vi.fn(),
  };

  return {
    ctx: {
      editor,
      pendingImageDrops,
      pendingFileDrops,
    } as never,
    editor,
    messages,
    enqueue,
    flush,
    pendingImageDrops,
    pendingFileDrops,
  };
};

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
    expect(messages).toEqual([
      {
        type: 'dnd',
        op: { type: 'start_internal_selection' },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(1);
    expect(event.preventDefault).not.toHaveBeenCalled();
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

describe('placeholder direct drop admission', () => {
  it('accepts only external image files for image placeholders', () => {
    expect(isAcceptedImagePlaceholderDrag(createDataTransfer({ files: [createFile('image.png', 'image/png')] }))).toBe(true);
    expect(
      isAcceptedImagePlaceholderDrag(createDataTransfer({ files: [createFile('a.png', 'image/png'), createFile('b.jpg', 'image/jpeg')] })),
    ).toBe(true);
    expect(isAcceptedImagePlaceholderDrag(createDataTransfer({ files: [createFile('doc.pdf', 'application/pdf')] }))).toBe(false);
    expect(
      isAcceptedImagePlaceholderDrag(
        createDataTransfer({ files: [createFile('image.png', 'image/png'), createFile('doc.pdf', 'application/pdf')] }),
      ),
    ).toBe(false);
  });

  it('accepts external files for file placeholders, including multiple images', () => {
    expect(isAcceptedFilePlaceholderDrag(createDataTransfer({ files: [createFile('doc.pdf', 'application/pdf')] }))).toBe(true);
    expect(isAcceptedFilePlaceholderDrag(createDataTransfer({ files: [createFile('image.png', 'image/png')] }))).toBe(true);
    expect(
      isAcceptedFilePlaceholderDrag(
        createDataTransfer({ files: [createFile('image.png', 'image/png'), createFile('doc.pdf', 'application/pdf')] }),
      ),
    ).toBe(true);
  });
});

describe('handleDrop', () => {
  it('routes dropped files through DndOp::Drop and queues bytes without direct fragment insertion', () => {
    const { ctx, messages, flush, pendingImageDrops, pendingFileDrops } = createCtx();
    const png = createFile('image.png', 'image/png');
    const pdf = createFile('doc.pdf', 'application/pdf');
    const event = createDragEvent(createDataTransfer({ files: [png, pdf] }));

    handleDrop(ctx, event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(pendingImageDrops).toEqual([png]);
    expect(pendingFileDrops).toEqual([pdf]);
    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
        },
      },
      {
        type: 'dnd',
        op: {
          type: 'drop',
          page: 0,
          x: 10,
          y: 20,
          payload: { type: 'files', image_count: 1, file_count: 1 },
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
        },
      },
    ]);
    expect(flush).toHaveBeenCalledTimes(2);
  });

  it('routes text/html payload through DndOp::Drop', () => {
    const { ctx, messages } = createCtx();
    const event = createDragEvent(
      createDataTransfer({
        types: ['text/html', 'text/plain'],
        data: { 'text/html': '<p>Hello</p>', 'text/plain': 'Hello' },
      }),
    );

    handleDrop(ctx, event);

    expect(messages).toEqual([
      {
        type: 'dnd',
        op: {
          type: 'over',
          page: 0,
          x: 10,
          y: 20,
          modifiers: { alt: false, ctrl: false, meta: false, shift: false },
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

    handleDrop(ctx, createDragEvent(createDataTransfer()));

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
