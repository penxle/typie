import { describe, expect, it, vi } from 'vitest';
import { handlePaste } from './clipboard';
import type { Editor } from '../editor.svelte';

const imageFile = (name = 'image.png') => new File(['x'], name, { type: 'image/png' });

const makeEditor = (overrides: Partial<Editor> = {}) =>
  ({
    insertImagesFromFiles: vi.fn().mockReturnValue(false),
    enqueue: vi.fn(),
    ...overrides,
  }) as unknown as Editor;

type FakePasteEvent = ClipboardEvent & { currentTarget: HTMLInputElement; preventDefault: ReturnType<typeof vi.fn> };

const makeEvent = (clipboardData: Partial<DataTransfer>): FakePasteEvent => {
  const event = {
    clipboardData: {
      files: [] as File[],
      items: [] as DataTransferItem[],
      getData: () => '',
      ...clipboardData,
    },
    currentTarget: null,
    preventDefault: vi.fn(),
  };
  return event as unknown as FakePasteEvent;
};

describe('handlePaste', () => {
  it('routes clipboard image files to insertImagesFromFiles and skips text paste', () => {
    const file = imageFile();
    const insertImagesFromFiles = vi.fn().mockReturnValue(true);
    const enqueue = vi.fn();
    const editor = makeEditor({ insertImagesFromFiles, enqueue });

    const event = makeEvent({
      files: [file] as unknown as FileList,
      getData: ((kind: string) => (kind === 'text/plain' ? 'fallback text' : '')) as DataTransfer['getData'],
    });

    handlePaste(editor, event);

    expect(insertImagesFromFiles).toHaveBeenCalledWith([file]);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(enqueue).not.toHaveBeenCalled();
  });

  it('falls back to FFI clipboard paste when no image is present', () => {
    const insertImagesFromFiles = vi.fn().mockReturnValue(false);
    const enqueue = vi.fn();
    const editor = makeEditor({ insertImagesFromFiles, enqueue });

    const event = makeEvent({
      getData: ((kind: string) => (kind === 'text/plain' ? 'hello' : '<p>hello</p>')) as DataTransfer['getData'],
    });

    handlePaste(editor, event);

    expect(insertImagesFromFiles).toHaveBeenCalledWith([]);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(enqueue).toHaveBeenCalledWith({
      type: 'clipboard',
      op: { type: 'paste', text: 'hello', html: '<p>hello</p>' },
    });
  });

  it('lets the default paste behavior run when both image and plain text are missing', () => {
    const insertImagesFromFiles = vi.fn().mockReturnValue(false);
    const enqueue = vi.fn();
    const editor = makeEditor({ insertImagesFromFiles, enqueue });

    const event = makeEvent({});

    handlePaste(editor, event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(enqueue).not.toHaveBeenCalled();
  });
});
