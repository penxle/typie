import { describe, expect, it, vi } from 'vitest';
import { handleDragOver, handleDrop } from './drop';
import type { Message } from '@typie/editor-ffi/browser';

const createFile = (name: string, type: string) => new File(['content'], name, { type });

const createDataTransfer = (files: File[], itemKinds?: { kind: string; type: string }[]) => {
  const items = (itemKinds ?? files.map((f) => ({ kind: 'file', type: f.type }))).map((item) => ({
    kind: item.kind,
    type: item.type,
  }));

  return {
    files,
    items,
    dropEffect: '',
  } as unknown as DataTransfer;
};

const createDragEvent = (dataTransfer: DataTransfer | null = null) => {
  return {
    dataTransfer,
    preventDefault: vi.fn(),
  } as unknown as DragEvent;
};

const createCtx = (readOnly = false, hasEditor = true) => {
  const messages: Message[] = [];
  const pendingImageDrops: File[] = [];
  const pendingFileDrops: File[] = [];
  const focus = vi.fn();

  const editor = hasEditor
    ? ({
        readOnly,
        enqueue: (message: Message) => messages.push(message),
        focus,
      } as never)
    : undefined;

  return {
    ctx: { editor, pendingImageDrops, pendingFileDrops } as never,
    messages,
    pendingImageDrops,
    pendingFileDrops,
    focus,
  };
};

describe('handleDragOver', () => {
  it('does nothing when editor is not initialized', () => {
    const { ctx } = createCtx(false, false);
    const event = createDragEvent(createDataTransfer([createFile('image.png', 'image/png')]));

    handleDragOver(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('calls preventDefault and sets dropEffect when files are dragged over', () => {
    const { ctx } = createCtx();
    const dataTransfer = createDataTransfer([createFile('image.png', 'image/png')]);
    const event = createDragEvent(dataTransfer);

    handleDragOver(ctx, event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(dataTransfer.dropEffect).toBe('copy');
  });

  it('does nothing when readOnly', () => {
    const { ctx } = createCtx(true);
    const event = createDragEvent(createDataTransfer([createFile('image.png', 'image/png')]));

    handleDragOver(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('does nothing when no file items are dragged', () => {
    const { ctx } = createCtx();
    const event = createDragEvent(createDataTransfer([], [{ kind: 'string', type: 'text/plain' }]));

    handleDragOver(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('does nothing when dataTransfer is null', () => {
    const { ctx } = createCtx();
    const event = createDragEvent(null);

    handleDragOver(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
  });
});

describe('handleDrop', () => {
  it('does nothing when editor is not initialized', () => {
    const { ctx, messages, pendingImageDrops } = createCtx(false, false);
    const event = createDragEvent(createDataTransfer([createFile('image.png', 'image/png')]));

    handleDrop(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(messages).toHaveLength(0);
    expect(pendingImageDrops).toHaveLength(0);
  });

  it('does nothing when readOnly', () => {
    const { ctx, messages, pendingImageDrops } = createCtx(true);
    const event = createDragEvent(createDataTransfer([createFile('image.png', 'image/png')]));

    handleDrop(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(messages).toHaveLength(0);
    expect(pendingImageDrops).toHaveLength(0);
  });

  it('does nothing when no files are dropped', () => {
    const { ctx, messages } = createCtx();
    const event = createDragEvent(createDataTransfer([]));

    handleDrop(ctx, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(messages).toHaveLength(0);
  });

  it('enqueues image node insertion and pushes to pendingImageDrops for each image file', () => {
    const { ctx, messages, pendingImageDrops, focus } = createCtx();
    const png = createFile('a.png', 'image/png');
    const webp = createFile('b.webp', 'image/webp');
    const event = createDragEvent(createDataTransfer([png, webp]));

    handleDrop(ctx, event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(pendingImageDrops).toEqual([png, webp]);
    expect(messages).toHaveLength(2);
    expect(messages.every((m) => m.type === 'insertion' && m.op.type === 'fragment' && m.op.fragment.node.type === 'image')).toBe(true);
    expect(focus).toHaveBeenCalled();
  });

  it('enqueues file node insertion and pushes to pendingFileDrops for non-image files', () => {
    const { ctx, messages, pendingFileDrops, focus } = createCtx();
    const pdf = createFile('doc.pdf', 'application/pdf');
    const xlsx = createFile('sheet.xlsx', 'application/octet-stream');
    const event = createDragEvent(createDataTransfer([pdf, xlsx]));

    handleDrop(ctx, event);

    expect(pendingFileDrops).toEqual([pdf, xlsx]);
    expect(messages).toHaveLength(2);
    expect(messages.every((m) => m.type === 'insertion' && m.op.type === 'fragment' && m.op.fragment.node.type === 'file')).toBe(true);
    expect(focus).toHaveBeenCalled();
  });

  it('routes mixed image and non-image files to their respective queues', () => {
    const { ctx, messages, pendingImageDrops, pendingFileDrops } = createCtx();
    const png = createFile('photo.png', 'image/png');
    const pdf = createFile('doc.pdf', 'application/pdf');
    const event = createDragEvent(createDataTransfer([png, pdf]));

    handleDrop(ctx, event);

    expect(pendingImageDrops).toEqual([png]);
    expect(pendingFileDrops).toEqual([pdf]);
    expect(messages).toHaveLength(2);
    expect(messages[0]).toMatchObject({ op: { fragment: { node: { type: 'image' } } } });
    expect(messages[1]).toMatchObject({ op: { fragment: { node: { type: 'file' } } } });
  });

  it('preserves drop order within each queue when multiple files of the same type are dropped', () => {
    const { ctx, pendingImageDrops } = createCtx();
    const first = createFile('first.png', 'image/png');
    const second = createFile('second.jpg', 'image/jpeg');
    const third = createFile('third.gif', 'image/gif');
    const event = createDragEvent(createDataTransfer([first, second, third]));

    handleDrop(ctx, event);

    expect(pendingImageDrops[0]).toBe(first);
    expect(pendingImageDrops[1]).toBe(second);
    expect(pendingImageDrops[2]).toBe(third);
  });
});
