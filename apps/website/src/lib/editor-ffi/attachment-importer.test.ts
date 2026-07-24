import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EditorAttachmentImporter } from './attachment-importer';
import type { AttachmentPlaceholderKind, EditorEvent, InputModifiers, Message } from '@typie/editor-ffi/browser';
import type { FileAsset, ImageAsset } from './types';

const upload = vi.hoisted(() => ({
  getImageDimensions: vi.fn<(src: string) => Promise<{ width: number; height: number }>>(),
  uploadFileAsFile: vi.fn<(file: File) => Promise<FileAsset>>(),
  uploadImageFile: vi.fn<(file: File) => Promise<ImageAsset>>(),
}));

vi.mock('./handlers/upload', () => upload);

type InflightImage = { uploadId: string; url?: string; width: number; height: number };
type InflightFile = { uploadId: string; name: string; size: number };
type Receipt = Extract<EditorEvent, { type: 'attachment_placeholders_inserted' }>;
type ReceiptListener = (editor: FakeEditor, event: Receipt) => void;

const imageAsset = (id: string): ImageAsset => ({
  id,
  url: `https://example.com/${id}.webp`,
  originalUrl: `https://example.com/${id}.png`,
  width: 640,
  height: 480,
  placeholder: 'preview',
});

const fileAsset = (id: string, name = `${id}.pdf`): FileAsset => ({
  id,
  name,
  size: '1024',
  url: `https://example.com/${name}`,
});

const file = (name: string, type = 'application/octet-stream'): File => new File([name], name, { type });

const external = (node: string, kind: AttachmentPlaceholderKind, id?: string) => ({
  page_idx: 0,
  node,
  bounds: { x: 0, y: 0, width: 100, height: 100 },
  is_selected: false,
  data: kind === 'image' ? { type: 'image' as const, id, proportion: 100 } : { type: 'file' as const, id },
});

class FakeEditor {
  #receiptListeners = new Set<ReceiptListener>();
  destroyed = false;
  readOnly = false;
  externalElements: ReturnType<typeof external>[] = [];
  inflightImages = new Map<string, InflightImage>();
  inflightFiles = new Map<string, InflightFile>();
  imageAssets = new Map<string, ImageAsset>();
  messages: Message[] = [];
  listenerCountsAtEnqueue: number[] = [];
  onRegistrations = 0;
  focus = vi.fn();
  flushImpl: () => void = vi.fn();

  on(event: EditorEvent['type'], listener: ReceiptListener): () => void {
    if (event !== 'attachment_placeholders_inserted') throw new Error(`Unexpected event: ${event}`);
    this.onRegistrations++;
    this.#receiptListeners.add(listener);
    return () => this.#receiptListeners.delete(listener);
  }

  enqueue(message: Message): void {
    this.listenerCountsAtEnqueue.push(this.#receiptListeners.size);
    this.messages.push(message);
  }

  flush(): void {
    this.flushImpl();
  }

  emitReceipt(requestId: string, nodeIds: string[]): void {
    const event: Receipt = { type: 'attachment_placeholders_inserted', request_id: requestId, node_ids: nodeIds };
    for (const listener of this.#receiptListeners) listener(this, event);
  }

  get receiptListenerCount(): number {
    return this.#receiptListeners.size;
  }
}

type FakeContext = { editor?: FakeEditor; fileAssets: Map<string, FileAsset> };

const createImporter = () => {
  const editor = new FakeEditor();
  const ctx: FakeContext = { editor, fileAssets: new Map() };
  const importer = new EditorAttachmentImporter(ctx as never);
  return { ctx, editor, importer };
};

const requestFrom = (message: Message): { requestId: string; kinds: AttachmentPlaceholderKind[] } => {
  if (message.type === 'insertion' && message.op.type === 'attachment_placeholders') {
    return { requestId: message.op.request_id, kinds: message.op.kinds };
  }
  if (message.type === 'dnd' && message.op.type === 'drop' && message.op.payload.type === 'files') {
    return { requestId: message.op.payload.request_id, kinds: message.op.payload.kinds };
  }
  throw new Error('Expected an attachment placeholder request');
};

const latestRequestFrom = (editor: FakeEditor): { requestId: string; kinds: AttachmentPlaceholderKind[] } => {
  const message = editor.messages.at(-1);
  if (!message) throw new Error('Expected an enqueued message');
  return requestFrom(message);
};

const installReceipt = (editor: FakeEditor, nodeIds: string[], options: { unrelated?: boolean } = {}): void => {
  let emitted = false;
  editor.flushImpl = () => {
    if (emitted) return;
    emitted = true;
    const { requestId, kinds } = latestRequestFrom(editor);
    if (options.unrelated) editor.emitReceipt('unrelated-request', ['unrelated-node']);
    for (const [index, nodeId] of nodeIds.entries()) {
      const kind = kinds[index];
      if (!kind) throw new Error(`Missing kind for ${nodeId}`);
      editor.externalElements.push(external(nodeId, kind));
    }
    editor.emitReceipt(requestId, nodeIds);
  };
};

const waitForIdle = async (editor: FakeEditor): Promise<void> => {
  await vi.waitFor(() => {
    expect(editor.inflightImages.size + editor.inflightFiles.size).toBe(0);
  });
};

const nodeMessages = (editor: FakeEditor): Message[] => editor.messages.filter((message) => message.type === 'node');

let createObjectURL: ReturnType<typeof vi.fn>;
let revokeObjectURL: ReturnType<typeof vi.fn>;

beforeEach(() => {
  upload.getImageDimensions.mockReset().mockResolvedValue({ width: 320, height: 240 });
  upload.uploadImageFile.mockReset().mockImplementation(async (input) => imageAsset(`image-${input.name}`));
  upload.uploadFileAsFile.mockReset().mockImplementation(async (input) => fileAsset(`file-${input.name}`, input.name));
  createObjectURL = vi.fn((input: File) => `blob:${input.name}`);
  revokeObjectURL = vi.fn();
  Object.defineProperties(URL, {
    createObjectURL: { configurable: true, writable: true, value: createObjectURL },
    revokeObjectURL: { configurable: true, writable: true, value: revokeObjectURL },
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('attachment receipt mapping', () => {
  it('listens before enqueue, ignores unrelated receipts, and preserves ordered item-to-node mapping', async () => {
    const { ctx, editor, importer } = createImporter();
    const image = file('cover.png', 'image/png');
    const document = file('notes.pdf', 'application/pdf');
    installReceipt(editor, ['image-node', 'file-node'], { unrelated: true });

    expect(
      importer.importAtSelection(
        [
          { file: image, kind: 'image' },
          { file: document, kind: 'file' },
        ],
        { onFailure: vi.fn() },
      ),
    ).toBe(true);

    expect(editor.listenerCountsAtEnqueue[0]).toBe(1);
    expect(editor.receiptListenerCount).toBe(0);
    expect(editor.inflightImages.has('image-node')).toBe(true);
    expect(editor.inflightFiles.has('file-node')).toBe(true);
    await waitForIdle(editor);
    expect(upload.uploadImageFile).toHaveBeenCalledWith(image);
    expect(upload.uploadFileAsFile).toHaveBeenCalledWith(document);
    expect(editor.imageAssets.has('image-cover.png')).toBe(true);
    expect(ctx.fileAssets.has('file-notes.pdf')).toBe(true);
    expect(nodeMessages(editor)).toHaveLength(2);
    expect(nodeMessages(editor)).toContainEqual({
      type: 'node',
      op: { type: 'set_attr', id: 'image-node', attr: { type: 'image', attr: { type: 'id', value: 'image-cover.png' } } },
    });
    expect(nodeMessages(editor)).toContainEqual({
      type: 'node',
      op: { type: 'set_attrs', id: 'file-node', attrs: { type: 'file', id: 'file-notes.pdf' } },
    });
  });

  it.each([
    [
      'no matching receipt',
      (editor: FakeEditor) => {
        editor.flushImpl = () => editor.emitReceipt('unrelated-request', ['unrelated-node']);
      },
      'no matching receipt',
    ],
    [
      'duplicate matching receipts',
      (editor: FakeEditor) => {
        editor.flushImpl = () => {
          const { requestId } = latestRequestFrom(editor);
          editor.externalElements.push(external('node-a', 'image'));
          editor.emitReceipt(requestId, ['node-a']);
          editor.emitReceipt(requestId, ['node-a']);
        };
      },
      'duplicate matching receipts',
    ],
    ['count mismatch', (editor: FakeEditor) => installReceipt(editor, ['node-a']), 'count mismatch'],
    [
      'duplicate node IDs',
      (editor: FakeEditor) => {
        editor.flushImpl = () => {
          const { requestId } = latestRequestFrom(editor);
          editor.externalElements.push(external('node-a', 'image'));
          editor.emitReceipt(requestId, ['node-a', 'node-a']);
        };
      },
      'duplicate node IDs',
    ],
  ])('rejects %s without reserving or reporting an upload failure', (_name, configure, diagnostic) => {
    const { editor, importer } = createImporter();
    const onFailure = vi.fn();
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());
    configure(editor);

    const accepted = importer.importAtSelection(
      [
        { file: file('a.png', 'image/png'), kind: 'image' },
        { file: file('b.png', 'image/png'), kind: 'image' },
      ],
      { onFailure },
    );

    expect(accepted).toBe(false);
    expect(editor.receiptListenerCount).toBe(0);
    expect(editor.inflightImages.size).toBe(0);
    expect(upload.uploadImageFile).not.toHaveBeenCalled();
    expect(onFailure).not.toHaveBeenCalled();
    expect(consoleError).toHaveBeenCalledOnce();
    expect(consoleError.mock.calls[0]?.[0]).toEqual(expect.stringContaining(diagnostic));
  });

  it.each([
    [
      'enqueue',
      (editor: FakeEditor, error: Error) => {
        vi.spyOn(editor, 'enqueue').mockImplementation(() => {
          throw error;
        });
      },
    ],
    [
      'flush',
      (editor: FakeEditor, error: Error) => {
        editor.flushImpl = () => {
          throw error;
        };
      },
    ],
  ])('disposes the receipt listener and diagnoses a thrown %s', (_name, configure) => {
    const { editor, importer } = createImporter();
    const error = new Error('request failed');
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());
    configure(editor, error);

    expect(importer.importAtSelection([{ file: file('a.png', 'image/png'), kind: 'image' }], { onFailure: vi.fn() })).toBe(false);
    expect(editor.receiptListenerCount).toBe(0);
    expect(editor.inflightImages.size).toBe(0);
    expect(consoleError).toHaveBeenCalledExactlyOnceWith(expect.stringContaining('enqueue or flush'), error);
  });

  it.each([
    ['missing', vi.fn<(editor: FakeEditor) => void>()],
    [
      'wrong-kind',
      (editor: FakeEditor) => {
        editor.externalElements.push(external('destination', 'file'));
      },
    ],
    [
      'filled',
      (editor: FakeEditor) => {
        editor.externalElements.push(external('destination', 'image', 'asset'));
      },
    ],
    [
      'pending',
      (editor: FakeEditor) => {
        editor.externalElements.push(external('destination', 'image'));
        editor.inflightImages.set('destination', { uploadId: 'pending', width: 0, height: 0 });
      },
    ],
  ])('rejects an explicitly supplied %s destination before requesting a replacement batch', async (_name, configure) => {
    const { editor, importer } = createImporter();
    const onFailure = vi.fn();
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());
    configure(editor);
    installReceipt(editor, ['created']);
    const inflightBefore = [...editor.inflightImages.entries()];

    const accepted = importer.importAtSelection([{ file: file('a.png', 'image/png'), kind: 'image' }], {
      existingNodeId: 'destination',
      onFailure,
    });
    await Promise.resolve();

    expect(accepted).toBe(false);
    expect(editor.onRegistrations).toBe(0);
    expect(editor.messages).toEqual([]);
    expect(editor.inflightImages.entries().toArray()).toEqual(inflightBefore);
    expect(editor.externalElements.some(({ node }) => node === 'created')).toBe(false);
    expect(upload.uploadImageFile).not.toHaveBeenCalled();
    expect(onFailure).not.toHaveBeenCalled();
    expect(consoleError).not.toHaveBeenCalled();
  });

  it('reuses one valid existing placeholder without registering a listener or inserting', async () => {
    const { editor, importer } = createImporter();
    editor.externalElements.push(external('existing', 'image'));

    expect(
      importer.importAtSelection([{ file: file('a.png', 'image/png'), kind: 'image' }], {
        existingNodeId: 'existing',
        onFailure: vi.fn(),
      }),
    ).toBe(true);

    expect(editor.onRegistrations).toBe(0);
    expect(editor.messages).toEqual([]);
    expect(editor.inflightImages.has('existing')).toBe(true);
    await waitForIdle(editor);
  });

  it('prepends the existing target and inserts one ordered batch only for the remaining items', async () => {
    const { editor, importer } = createImporter();
    const first = file('first.png', 'image/png');
    const second = file('second.pdf', 'application/pdf');
    editor.externalElements.push(external('existing', 'image'));
    installReceipt(editor, ['tail-node']);

    expect(
      importer.importAtSelection(
        [
          { file: first, kind: 'image' },
          { file: second, kind: 'file' },
        ],
        { existingNodeId: 'existing', onFailure: vi.fn() },
      ),
    ).toBe(true);

    expect(editor.messages[0]).toMatchObject({
      type: 'insertion',
      op: { type: 'attachment_placeholders', kinds: ['file'] },
    });
    expect(editor.inflightImages.has('existing')).toBe(true);
    expect(editor.inflightFiles.has('tail-node')).toBe(true);
    await waitForIdle(editor);
    expect(
      nodeMessages(editor)
        .map((message) => (message.type === 'node' ? message.op.id : ''))
        .toSorted((left, right) => left.localeCompare(right)),
    ).toEqual(['existing', 'tail-node']);
  });

  it('rejects a drop receipt that places the reuse candidate anywhere except first', () => {
    const { editor, importer } = createImporter();
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());
    editor.externalElements.push(external('candidate', 'image'));
    editor.flushImpl = () => {
      const { requestId } = latestRequestFrom(editor);
      editor.externalElements.push(external('created', 'file'));
      editor.emitReceipt(requestId, ['created', 'candidate']);
    };

    const accepted = importer.importAtDrop(
      [
        { file: file('a.png', 'image/png'), kind: 'image' },
        { file: file('b.pdf'), kind: 'file' },
      ],
      { page: 2, x: 10, y: 20, modifiers: { shift: true }, reuseNodeId: 'candidate', onFailure: vi.fn() },
    );

    expect(accepted).toBe(false);
    expect(editor.inflightImages.size + editor.inflightFiles.size).toBe(0);
    expect(editor.receiptListenerCount).toBe(0);
    expect(consoleError).toHaveBeenCalledOnce();
    expect(consoleError.mock.calls[0]?.[0]).toEqual(expect.stringContaining('reuse candidate'));
  });

  it.each([
    ['candidate reuse', ['candidate', 'created'], ['created']],
    ['engine fallback', ['fallback-a', 'fallback-b'], ['fallback-a', 'fallback-b']],
  ])('tracks auto-created provenance for drop %s', async (_name, nodeIds, deletedIds) => {
    const { editor, importer } = createImporter();
    const onFailure = vi.fn();
    editor.externalElements.push(external('candidate', 'file'));
    installReceipt(editor, nodeIds);
    upload.uploadFileAsFile.mockRejectedValue(new Error('upload failed'));

    expect(
      importer.importAtDrop(
        [
          { file: file('a.pdf'), kind: 'file' },
          { file: file('b.pdf'), kind: 'file' },
        ],
        { page: 0, x: 1, y: 2, modifiers: {}, reuseNodeId: 'candidate', onFailure },
      ),
    ).toBe(true);

    await waitForIdle(editor);
    expect(onFailure).toHaveBeenCalledTimes(2);
    expect(
      nodeMessages(editor)
        .filter((message) => message.type === 'node' && message.op.type === 'delete')
        .map((message) => (message.type === 'node' ? message.op.id : '')),
    ).toEqual(deletedIds);
  });

  it('forwards the exact drop coordinates, modifiers, ordered kinds, and valid candidate', () => {
    const { editor, importer } = createImporter();
    const modifiers: InputModifiers = { ctrl: true, alt: true };
    editor.externalElements.push(external('candidate', 'image'));
    installReceipt(editor, ['candidate']);

    expect(
      importer.importAtDrop([{ file: file('a.png', 'image/png'), kind: 'image' }], {
        page: 3,
        x: 12.5,
        y: 42,
        modifiers,
        reuseNodeId: 'candidate',
        onFailure: vi.fn(),
      }),
    ).toBe(true);
    expect(editor.messages[0]).toMatchObject({
      type: 'dnd',
      op: {
        type: 'drop',
        page: 3,
        x: 12.5,
        y: 42,
        modifiers,
        payload: { type: 'files', kinds: ['image'], reuse_node_id: 'candidate' },
      },
    });
  });
});

describe('attachment target lifecycle', () => {
  it('reserves every target synchronously but starts at most five complete image pipelines', async () => {
    const { editor, importer } = createImporter();
    const files = Array.from({ length: 7 }, (_, index) => file(`${index}.png`, 'image/png'));
    const nodeIds = files.map((_, index) => `node-${index}`);
    const uploads = files.map(() => Promise.withResolvers<ImageAsset>());
    let active = 0;
    let maxActive = 0;
    upload.uploadImageFile.mockImplementation((input) => {
      const index = files.indexOf(input);
      const pending = uploads[index];
      if (!pending) throw new Error(`Missing upload for ${input.name}`);
      active++;
      maxActive = Math.max(maxActive, active);
      return pending.promise.finally(() => active--);
    });
    installReceipt(editor, nodeIds);

    expect(
      importer.importAtSelection(
        files.map((input) => ({ file: input, kind: 'image' as const })),
        { onFailure: vi.fn() },
      ),
    ).toBe(true);
    expect(editor.inflightImages.size).toBe(7);
    expect(editor.inflightImages.values().every((pending) => pending.url === undefined)).toBe(true);

    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledTimes(5));
    expect(createObjectURL).toHaveBeenCalledTimes(5);
    expect([...editor.inflightImages.values()].filter((pending) => pending.url !== undefined)).toHaveLength(5);
    const firstUpload = uploads[0];
    const secondUpload = uploads[1];
    if (!firstUpload || !secondUpload) throw new Error('Expected the first two uploads');
    firstUpload.resolve(imageAsset('image-0'));
    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledTimes(6));
    secondUpload.resolve(imageAsset('image-1'));
    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledTimes(7));
    for (const [index, pending] of uploads.slice(2).entries()) pending.resolve(imageAsset(`image-${index + 2}`));

    await waitForIdle(editor);
    expect(maxActive).toBe(5);
    expect(nodeMessages(editor)).toHaveLength(7);
    expect(revokeObjectURL).toHaveBeenCalledTimes(7);
  });

  it('commits image/file IDs, caches assets, cleans only owned tokens, and never focuses', async () => {
    const { ctx, editor, importer } = createImporter();
    const picture = file('picture.png', 'image/png');
    const document = file('document.pdf', 'application/pdf');
    installReceipt(editor, ['image-node', 'file-node']);
    upload.uploadImageFile.mockResolvedValue(imageAsset('image-asset'));
    upload.uploadFileAsFile.mockResolvedValue(fileAsset('file-asset'));

    expect(
      importer.importAtSelection(
        [
          { file: picture, kind: 'image' },
          { file: document, kind: 'file' },
        ],
        { onFailure: vi.fn() },
      ),
    ).toBe(true);

    await waitForIdle(editor);
    expect(editor.imageAssets.get('image-asset')).toEqual(imageAsset('image-asset'));
    expect(ctx.fileAssets.get('file-asset')).toEqual(fileAsset('file-asset'));
    expect(nodeMessages(editor)).toHaveLength(2);
    expect(nodeMessages(editor)).toContainEqual({
      type: 'node',
      op: { type: 'set_attr', id: 'image-node', attr: { type: 'image', attr: { type: 'id', value: 'image-asset' } } },
    });
    expect(nodeMessages(editor)).toContainEqual({
      type: 'node',
      op: { type: 'set_attrs', id: 'file-node', attrs: { type: 'file', id: 'file-asset' } },
    });
    expect(editor.focus).not.toHaveBeenCalled();
  });

  it('leaves a reused target empty, deletes an auto-created target, and lets a successful sibling finish', async () => {
    const { ctx, editor, importer } = createImporter();
    const first = file('failed.pdf');
    const second = file('ok.pdf');
    const onFailure = vi.fn();
    editor.externalElements.push(external('existing', 'file'));
    installReceipt(editor, ['created']);
    upload.uploadFileAsFile.mockImplementation(async (input) => {
      if (input === first) throw new Error('failed');
      return fileAsset('successful');
    });

    expect(
      importer.importAtSelection(
        [
          { file: first, kind: 'file' },
          { file: second, kind: 'file' },
        ],
        { existingNodeId: 'existing', onFailure },
      ),
    ).toBe(true);

    await waitForIdle(editor);
    expect(onFailure).toHaveBeenCalledExactlyOnceWith({ file: first, kind: 'file' });
    expect(ctx.fileAssets.has('successful')).toBe(true);
    expect(nodeMessages(editor)).toEqual([
      { type: 'node', op: { type: 'set_attrs', id: 'created', attrs: { type: 'file', id: 'successful' } } },
    ]);
  });

  it.each([
    ['editor replacement', (ctx: FakeContext) => (ctx.editor = new FakeEditor())],
    ['destroy', (_ctx: FakeContext, editor: FakeEditor) => (editor.destroyed = true)],
    ['read-only transition', (_ctx: FakeContext, editor: FakeEditor) => (editor.readOnly = true)],
    ['node deletion', (_ctx: FakeContext, editor: FakeEditor) => (editor.externalElements = [])],
    ['undo removal', (_ctx: FakeContext, editor: FakeEditor) => (editor.externalElements = [])],
    ['kind replacement', (_ctx: FakeContext, editor: FakeEditor) => (editor.externalElements = [external('existing', 'file')])],
    ['ID replacement', (_ctx: FakeContext, editor: FakeEditor) => (editor.externalElements = [external('existing', 'image', 'other-id')])],
    [
      'pending-token replacement',
      (_ctx: FakeContext, editor: FakeEditor) =>
        editor.inflightImages.set('existing', { uploadId: 'replacement', url: 'blob:replacement', width: 10, height: 10 }),
    ],
  ])('treats %s during upload as stale without cache, commit, delete, or callback', async (_name, makeStale) => {
    const { ctx, editor, importer } = createImporter();
    const pending = Promise.withResolvers<ImageAsset>();
    const onFailure = vi.fn();
    editor.externalElements.push(external('existing', 'image'));
    upload.uploadImageFile.mockReturnValue(pending.promise);

    expect(
      importer.importAtSelection([{ file: file('picture.png', 'image/png'), kind: 'image' }], {
        existingNodeId: 'existing',
        onFailure,
      }),
    ).toBe(true);
    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledOnce());
    makeStale(ctx, editor);
    pending.resolve(imageAsset('uploaded'));
    await vi.waitFor(() => expect(revokeObjectURL).toHaveBeenCalledWith('blob:picture.png'));

    expect(editor.imageAssets.size).toBe(0);
    expect(nodeMessages(editor)).toEqual([]);
    expect(onFailure).not.toHaveBeenCalled();
    if (_name === 'pending-token replacement') {
      expect(editor.inflightImages.get('existing')?.uploadId).toBe('replacement');
    } else {
      expect(editor.inflightImages.has('existing')).toBe(false);
    }
  });

  it('rechecks current state after image metadata before uploading', async () => {
    const { editor, importer } = createImporter();
    const dimensions = Promise.withResolvers<{ width: number; height: number }>();
    const onFailure = vi.fn();
    editor.externalElements.push(external('existing', 'image'));
    upload.getImageDimensions.mockReturnValue(dimensions.promise);

    expect(
      importer.importAtSelection([{ file: file('picture.png', 'image/png'), kind: 'image' }], {
        existingNodeId: 'existing',
        onFailure,
      }),
    ).toBe(true);
    await vi.waitFor(() => expect(upload.getImageDimensions).toHaveBeenCalledOnce());
    editor.readOnly = true;
    dimensions.resolve({ width: 100, height: 100 });
    await vi.waitFor(() => expect(editor.inflightImages.size).toBe(0));

    expect(upload.uploadImageFile).not.toHaveBeenCalled();
    expect(onFailure).not.toHaveBeenCalled();
    expect(nodeMessages(editor)).toEqual([]);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:picture.png');
  });

  it('rechecks state immediately before deleting a failed auto-created target', async () => {
    const { editor, importer } = createImporter();
    const item = { file: file('failed.pdf'), kind: 'file' as const };
    installReceipt(editor, ['created']);
    upload.uploadFileAsFile.mockRejectedValue(new Error('failed'));
    const onFailure = vi.fn(() => {
      editor.readOnly = true;
    });

    expect(importer.importAtSelection([item], { onFailure })).toBe(true);
    await waitForIdle(editor);
    expect(onFailure).toHaveBeenCalledWith(item);
    expect(nodeMessages(editor)).toEqual([]);
  });

  it('swallows a throwing failure callback and still cleans the owned target without cancelling siblings', async () => {
    const { ctx, editor, importer } = createImporter();
    const failed = file('failed.pdf');
    const successful = file('successful.pdf');
    installReceipt(editor, ['failed-node', 'successful-node']);
    upload.uploadFileAsFile.mockImplementation(async (input) => {
      if (input === failed) throw new Error('failed');
      return fileAsset('successful');
    });
    const onFailure = vi.fn(() => {
      throw new Error('toast failed');
    });

    expect(
      importer.importAtSelection(
        [
          { file: failed, kind: 'file' },
          { file: successful, kind: 'file' },
        ],
        { onFailure },
      ),
    ).toBe(true);
    await waitForIdle(editor);

    expect(onFailure).toHaveBeenCalledOnce();
    expect(ctx.fileAssets.has('successful')).toBe(true);
    expect(nodeMessages(editor)).toEqual([
      { type: 'node', op: { type: 'delete', id: 'failed-node' } },
      { type: 'node', op: { type: 'set_attrs', id: 'successful-node', attrs: { type: 'file', id: 'successful' } } },
    ]);
  });

  it('diagnoses an unexpected rejected worker promise', async () => {
    const { editor, importer } = createImporter();
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());
    installReceipt(editor, ['created']);
    upload.uploadImageFile.mockRejectedValue(new Error('upload failed'));
    revokeObjectURL.mockImplementation(() => {
      throw new Error('cleanup failed');
    });

    expect(importer.importAtSelection([{ file: file('failed.png', 'image/png'), kind: 'image' }], { onFailure: vi.fn() })).toBe(true);
    await vi.waitFor(() => expect(consoleError).toHaveBeenCalledOnce());

    expect(consoleError.mock.calls[0]?.[0]).toEqual(expect.stringContaining('worker failure'));
    expect(consoleError.mock.calls[0]?.[1]).toEqual(expect.objectContaining({ message: 'cleanup failed' }));
  });

  it('cancels running and queued image targets, revoking only URLs that were created', async () => {
    const { editor, importer } = createImporter();
    const files = Array.from({ length: 6 }, (_, index) => file(`${index}.png`, 'image/png'));
    const uploads = files.map(() => Promise.withResolvers<ImageAsset>());
    installReceipt(
      editor,
      files.map((_, index) => `node-${index}`),
    );
    upload.uploadImageFile.mockImplementation((input) => {
      const pending = uploads[files.indexOf(input)];
      if (!pending) throw new Error(`Missing upload for ${input.name}`);
      return pending.promise;
    });

    expect(
      importer.importAtSelection(
        files.map((input) => ({ file: input, kind: 'image' as const })),
        { onFailure: vi.fn() },
      ),
    ).toBe(true);
    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledTimes(5));
    importer.cancelNode(editor as never, 'node-0');
    importer.cancelNode(editor as never, 'node-5');
    expect(editor.inflightImages.has('node-0')).toBe(false);
    expect(editor.inflightImages.has('node-5')).toBe(false);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:0.png');

    for (const [index, pending] of uploads.slice(0, 5).entries()) pending.resolve(imageAsset(`image-${index}`));
    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledTimes(5));
    expect(createObjectURL.mock.calls.some(([input]) => input === files[5])).toBe(false);
    expect(revokeObjectURL).not.toHaveBeenCalledWith('blob:5.png');
  });

  it('cancelEditor clears and revokes only pending state owned by the supplied editor', async () => {
    const { ctx, editor, importer } = createImporter();
    const pending = Promise.withResolvers<ImageAsset>();
    editor.externalElements.push(external('existing', 'image'));
    upload.uploadImageFile.mockReturnValue(pending.promise);
    expect(
      importer.importAtSelection([{ file: file('picture.png', 'image/png'), kind: 'image' }], {
        existingNodeId: 'existing',
        onFailure: vi.fn(),
      }),
    ).toBe(true);
    await vi.waitFor(() => expect(upload.uploadImageFile).toHaveBeenCalledOnce());
    const replacement = new FakeEditor();
    replacement.inflightImages.set('other', { uploadId: 'other', url: 'blob:other', width: 1, height: 1 });
    ctx.editor = replacement;

    importer.cancelEditor(editor as never);
    expect(editor.inflightImages.size).toBe(0);
    expect(replacement.inflightImages.has('other')).toBe(true);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:picture.png');
    expect(revokeObjectURL).not.toHaveBeenCalledWith('blob:other');
    pending.resolve(imageAsset('uploaded'));
  });

  it('routes an image MIME explicitly classified as file through file persistence', async () => {
    const { editor, importer } = createImporter();
    const input = file('keep-as-file.png', 'image/png');
    installReceipt(editor, ['file-node']);

    expect(importer.importAtSelection([{ file: input, kind: 'file' }], { onFailure: vi.fn() })).toBe(true);
    await waitForIdle(editor);
    expect(upload.uploadFileAsFile).toHaveBeenCalledWith(input);
    expect(upload.uploadImageFile).not.toHaveBeenCalled();
    expect(createObjectURL).not.toHaveBeenCalled();
  });

  it('keeps an SVG File unchanged through preview creation, dimension decoding, and upload', async () => {
    const { editor, importer } = createImporter();
    const svg = file('drawing.svg', 'image/svg+xml');
    installReceipt(editor, ['image-node']);

    expect(importer.importAtSelection([{ file: svg, kind: 'image' }], { onFailure: vi.fn() })).toBe(true);
    await waitForIdle(editor);
    expect(createObjectURL).toHaveBeenCalledExactlyOnceWith(svg);
    expect(upload.getImageDimensions).toHaveBeenCalledExactlyOnceWith('blob:drawing.svg');
    expect(upload.uploadImageFile).toHaveBeenCalledExactlyOnceWith(svg);
  });
});

describe('placeholder reuse checks', () => {
  it('requires the current live editable editor, exact empty kind, and no pending token', () => {
    const { ctx, editor, importer } = createImporter();
    editor.externalElements.push(external('image-node', 'image'), external('file-node', 'file'), external('filled', 'image', 'asset'));

    expect(importer.canReusePlaceholder('image-node', 'image')).toBe(true);
    expect(importer.canReusePlaceholder('image-node', 'file')).toBe(false);
    expect(importer.canReusePlaceholder('filled', 'image')).toBe(false);
    editor.inflightImages.set('image-node', { uploadId: 'pending', width: 0, height: 0 });
    expect(importer.canReusePlaceholder('image-node', 'image')).toBe(false);
    editor.inflightImages.clear();
    editor.readOnly = true;
    expect(importer.canReusePlaceholder('image-node', 'image')).toBe(false);
    editor.readOnly = false;
    ctx.editor = undefined;
    expect(importer.canReusePlaceholder('image-node', 'image')).toBe(false);
  });
});
