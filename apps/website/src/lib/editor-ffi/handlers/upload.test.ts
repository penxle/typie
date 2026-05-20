import { describe, expect, it } from 'vitest';
import { assignPendingImageFiles, getClipboardImageFiles, getDataTransferImageFiles } from './upload';

const imageFile = (name: string, type = 'image/png') => new File(['x'], name, { type });

describe('getClipboardImageFiles', () => {
  it('prefers files attached directly to the clipboard payload', () => {
    const image = imageFile('image.png');
    const text = new File(['hello'], 'note.txt', { type: 'text/plain' });

    const clipboardData = {
      files: [image, text],
      items: [],
    } as unknown as DataTransfer;

    expect(getClipboardImageFiles(clipboardData)).toEqual([image]);
  });

  it('falls back to dataTransfer items when files are absent', () => {
    const image = imageFile('clipboard.png');
    const clipboardData = {
      files: [],
      items: [
        { type: 'image/png', getAsFile: () => image },
        { type: 'text/plain', getAsFile: () => null },
      ],
    } as unknown as DataTransfer;

    expect(getClipboardImageFiles(clipboardData)).toEqual([image]);
  });

  it('returns an empty list when the clipboard payload is missing', () => {
    expect(getClipboardImageFiles(null)).toEqual([]);
  });
});

describe('getDataTransferImageFiles', () => {
  it('returns only image files from drag-and-drop payloads', () => {
    const image = imageFile('image.png');
    const text = new File(['hello'], 'note.txt', { type: 'text/plain' });

    const dataTransfer = { files: [image, text] } as unknown as DataTransfer;

    expect(getDataTransferImageFiles(dataTransfer)).toEqual([image]);
  });

  it('returns an empty list when the drop payload is missing', () => {
    expect(getDataTransferImageFiles(null)).toEqual([]);
  });
});

describe('assignPendingImageFiles', () => {
  it('assigns pending files to empty image nodes in document order', () => {
    const first = imageFile('first.png');
    const second = imageFile('second.png');

    const result = assignPendingImageFiles(
      [
        { nodeId: 'a', assigned: false, inflight: false },
        { nodeId: 'b', assigned: false, inflight: false },
      ],
      [first, second],
    );

    expect(result.assignments).toEqual([
      { nodeId: 'a', file: first },
      { nodeId: 'b', file: second },
    ]);
    expect(result.remainingFiles).toEqual([]);
  });

  it('skips nodes that already have an image, an assignment, or an inflight upload', () => {
    const first = imageFile('first.png');
    const second = imageFile('second.png');

    const result = assignPendingImageFiles(
      [
        { nodeId: 'done', imageId: 'image-1', assigned: false, inflight: false },
        { nodeId: 'assigned', assigned: true, inflight: false },
        { nodeId: 'uploading', assigned: false, inflight: true },
        { nodeId: 'target', assigned: false, inflight: false },
      ],
      [first, second],
    );

    expect(result.assignments).toEqual([{ nodeId: 'target', file: first }]);
    expect(result.remainingFiles).toEqual([second]);
  });

  it('leaves remaining files untouched when no candidates are available', () => {
    const file = imageFile('orphan.png');

    const result = assignPendingImageFiles([{ nodeId: 'done', imageId: 'image-1', assigned: false, inflight: false }], [file]);

    expect(result.assignments).toEqual([]);
    expect(result.remainingFiles).toEqual([file]);
  });
});
