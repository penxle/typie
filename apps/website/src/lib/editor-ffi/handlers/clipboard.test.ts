import { beforeEach, describe, expect, it, vi } from 'vitest';
import { handlePaste, requestPaste } from './clipboard';
import type { Message } from '@typie/editor-ffi/browser';
import type { AttachmentImportFailureHandler, AttachmentImportItem } from '../attachment-importer';

type FakeEditor = {
  enqueue: ReturnType<typeof vi.fn<(message: Message) => void>>;
  scrollIntoView: ReturnType<typeof vi.fn>;
};

const createContext = (importAccepted = true) => {
  const editor: FakeEditor = {
    enqueue: vi.fn(),
    scrollIntoView: vi.fn(),
  };
  const importAtSelection = vi.fn<
    (items: readonly AttachmentImportItem[], options: { onFailure: AttachmentImportFailureHandler }) => boolean
  >(() => importAccepted);
  return {
    ctx: {
      editor,
      attachmentImporter: { importAtSelection },
    },
    editor,
    importAtSelection,
  };
};

const transferItem = (file: File | null) => ({
  kind: 'file',
  type: file?.type ?? '',
  getAsFile: () => file,
});

const clipboardData = ({
  html = '',
  text = '',
  items = [],
  files = [],
}: {
  html?: string;
  text?: string;
  items?: ReturnType<typeof transferItem>[];
  files?: File[];
}) =>
  ({
    items,
    files,
    getData: (type: string) => (type === 'text/html' ? html : type === 'text/plain' ? text : ''),
  }) as unknown as DataTransfer;

const pasteEvent = (data: DataTransfer | null) =>
  ({
    clipboardData: data,
    preventDefault: vi.fn(),
  }) as unknown as ClipboardEvent & { currentTarget: HTMLTextAreaElement };

const clipboardItem = (types: Record<string, Blob>) => ({
  types: Object.keys(types),
  getType: vi.fn(async (type: string) => {
    const blob = types[type];
    if (!blob) throw new Error(`Missing type: ${type}`);
    return blob;
  }),
});

const installClipboard = ({
  read,
  readText = vi.fn(async () => ''),
}: {
  read: () => Promise<ReturnType<typeof clipboardItem>[]>;
  readText?: () => Promise<string>;
}) => {
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { read: vi.fn(read), readText: vi.fn(readText) },
  });
  return navigator.clipboard as Clipboard & {
    read: ReturnType<typeof vi.fn>;
    readText: ReturnType<typeof vi.fn>;
  };
};

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('native paste arbitration', () => {
  it('prefers trim-non-empty HTML over ordered binary siblings and plain text', () => {
    const image = new File(['image'], 'image.png', { type: 'image/png' });
    const { ctx, editor, importAtSelection } = createContext();
    const event = pasteEvent(
      clipboardData({
        html: '  <p>rich</p>  ',
        text: 'plain',
        items: [transferItem(image)],
        files: [image],
      }),
    );

    handlePaste(ctx as never, event, vi.fn());

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'clipboard',
      op: { type: 'paste', html: '  <p>rich</p>  ', text: 'plain' },
    });
    expect(importAtSelection).not.toHaveBeenCalled();
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(editor.scrollIntoView).toHaveBeenCalledOnce();
  });

  it('preserves DataTransferItem order and classifies SVG as image', () => {
    const svg = new File(['<svg/>'], 'diagram.svg', { type: 'image/svg+xml' });
    const pdf = new File(['pdf'], 'document.pdf', { type: 'application/pdf' });
    const { ctx, importAtSelection } = createContext();
    const onFailure = vi.fn();
    const event = pasteEvent(
      clipboardData({
        text: 'ignored',
        items: [transferItem(svg), transferItem(pdf)],
        files: [pdf, svg],
      }),
    );

    handlePaste(ctx as never, event, onFailure);

    expect(importAtSelection).toHaveBeenCalledWith(
      [
        { file: svg, kind: 'image' },
        { file: pdf, kind: 'file' },
      ],
      { onFailure },
    );
    expect(event.preventDefault).toHaveBeenCalledOnce();
  });

  it('falls back to the complete clipboardData.files list when any file item is unavailable', () => {
    const image = new File(['image'], 'image.png', { type: 'image/png' });
    const pdf = new File(['pdf'], 'document.pdf', { type: 'application/pdf' });
    const { ctx, importAtSelection } = createContext();
    const event = pasteEvent(clipboardData({ items: [transferItem(image), transferItem(null)], files: [image, pdf] }));

    handlePaste(ctx as never, event, vi.fn());

    expect(importAtSelection.mock.calls[0]?.[0]).toEqual([
      { file: image, kind: 'image' },
      { file: pdf, kind: 'file' },
    ]);
  });

  it('uses plain text only when neither HTML nor files are available', () => {
    const { ctx, editor, importAtSelection } = createContext();
    const event = pasteEvent(clipboardData({ text: 'plain' }));

    handlePaste(ctx as never, event, vi.fn());

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'clipboard',
      op: { type: 'paste', html: undefined, text: 'plain' },
    });
    expect(importAtSelection).not.toHaveBeenCalled();
    expect(event.preventDefault).toHaveBeenCalledOnce();
  });

  it('does not prevent an empty native paste', () => {
    const { ctx, editor, importAtSelection } = createContext();
    const event = pasteEvent(clipboardData({ html: ' \n ' }));

    handlePaste(ctx as never, event, vi.fn());

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(importAtSelection).not.toHaveBeenCalled();
  });

  it('does not fall through to plain text when attachment mapping is rejected', () => {
    const image = new File(['image'], 'image.png', { type: 'image/png' });
    const { ctx, editor, importAtSelection } = createContext(false);
    const event = pasteEvent(clipboardData({ text: 'must not paste', items: [transferItem(image)] }));

    handlePaste(ctx as never, event, vi.fn());

    expect(importAtSelection).toHaveBeenCalledOnce();
    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(editor.scrollIntoView).not.toHaveBeenCalled();
  });
});

describe('programmatic paste arbitration', () => {
  it('does not paste into a replacement editor after the clipboard read completes', async () => {
    const read = Promise.withResolvers<ReturnType<typeof clipboardItem>[]>();
    installClipboard({ read: () => read.promise });
    const { ctx, editor, importAtSelection } = createContext();
    const replacement = createContext().editor;

    const paste = requestPaste(ctx as never, vi.fn());
    ctx.editor = replacement;
    read.resolve([clipboardItem({ 'text/plain': new Blob(['plain'], { type: 'text/plain' }) })]);
    await paste;

    expect(editor.enqueue).not.toHaveBeenCalled();
    expect(replacement.enqueue).not.toHaveBeenCalled();
    expect(importAtSelection).not.toHaveBeenCalled();
  });

  it('prefers non-empty HTML even when another ClipboardItem contains binary data', async () => {
    const binary = clipboardItem({ 'image/png': new Blob(['image'], { type: 'image/png' }) });
    installClipboard({
      read: async () => [
        clipboardItem({ 'text/html': new Blob([' \n '], { type: 'text/html' }) }),
        binary,
        clipboardItem({
          'text/html': new Blob(['<p>rich</p>'], { type: 'text/html' }),
          'text/plain': new Blob(['plain'], { type: 'text/plain' }),
        }),
      ],
    });
    const { ctx, editor, importAtSelection } = createContext();

    await requestPaste(ctx as never, vi.fn());

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'clipboard',
      op: { type: 'paste', html: '<p>rich</p>', text: 'plain' },
    });
    expect(importAtSelection).not.toHaveBeenCalled();
    expect(binary.getType).not.toHaveBeenCalled();
  });

  it('materializes the first non-text representation from each item in item order', async () => {
    const pdf = new Blob(['pdf'], { type: 'application/pdf' });
    const svg = new Blob(['<svg/>'], { type: 'image/svg+xml' });
    installClipboard({
      read: async () => [
        clipboardItem({ 'application/pdf': pdf, 'image/png': new Blob(['ignored'], { type: 'image/png' }) }),
        clipboardItem({ 'image/svg+xml': svg }),
      ],
    });
    const { ctx, importAtSelection } = createContext();
    const onFailure = vi.fn();

    await requestPaste(ctx as never, onFailure);

    const [items, options] = importAtSelection.mock.calls[0] ?? [];
    expect(items).toEqual([
      { file: expect.any(File), kind: 'file' },
      { file: expect.any(File), kind: 'image' },
    ]);
    expect(items?.map((item) => [item.file.name, item.file.type])).toEqual([
      ['clipboard-file', 'application/pdf'],
      ['clipboard-image', 'image/svg+xml'],
    ]);
    expect(options).toEqual({ onFailure });
  });

  it('uses plain text when rich read has no HTML or binary representation', async () => {
    installClipboard({
      read: async () => [clipboardItem({ 'text/plain': new Blob(['plain'], { type: 'text/plain' }) })],
    });
    const { ctx, editor } = createContext();

    await requestPaste(ctx as never, vi.fn());

    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'clipboard',
      op: { type: 'paste', html: undefined, text: 'plain' },
    });
  });

  it('falls back to readText when navigator.clipboard.read rejects', async () => {
    const clipboard = installClipboard({
      read: async () => {
        throw new Error('denied');
      },
      readText: async () => 'fallback',
    });
    const { ctx, editor } = createContext();

    await requestPaste(ctx as never, vi.fn());

    expect(clipboard.readText).toHaveBeenCalledOnce();
    expect(editor.enqueue).toHaveBeenCalledWith({
      type: 'clipboard',
      op: { type: 'paste', html: undefined, text: 'fallback' },
    });
  });
});
