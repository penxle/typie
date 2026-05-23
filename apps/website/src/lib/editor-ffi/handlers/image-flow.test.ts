import { describe, expect, it, vi } from 'vitest';
import {
  calculateImageHeight,
  calculateImageWidth,
  createDeleteNodeMessage,
  createSetImageAttrsMessage,
  getFirstImageFile,
  processImageUpload,
} from './image-flow';
import type { Message } from '@typie/editor-ffi/browser';
import type { ImageAsset } from '../types';

const createFile = (name: string, type: string) => new File(['content'], name, { type });

const createAsset = (id = 'image-1'): ImageAsset => ({
  id,
  url: 'https://example.com/image.webp',
  originalUrl: 'https://example.com/original.png',
  width: 640,
  height: 480,
  placeholder: 'placeholder',
});

const createDeps = () => {
  const messages: Message[] = [];
  const inflight = new Map<string, { url: string; width: number; height: number }>();
  const assets = new Map<string, ImageAsset>();
  const focus = vi.fn();

  return {
    deps: {
      setInflightImage: (nodeId: string, image: { url: string; width: number; height: number }) => inflight.set(nodeId, image),
      deleteInflightImage: (nodeId: string) => inflight.delete(nodeId),
      setImageAsset: (asset: ImageAsset) => assets.set(asset.id, asset),
      enqueue: (message: Message) => messages.push(message),
      focus,
    },
    messages,
    inflight,
    assets,
    focus,
  };
};

describe('v2 image flow messages', () => {
  it('creates a v2 node attrs message for uploaded image id and rounded proportion', () => {
    expect(createSetImageAttrsMessage('node-1', 'image-1', 74.6)).toEqual({
      type: 'node',
      op: {
        type: 'set_attrs',
        id: 'node-1',
        attrs: {
          type: 'image',
          id: 'image-1',
          proportion: 75,
        },
      },
    });
  });

  it('creates a v2 node delete message for placeholder cleanup and deletion', () => {
    expect(createDeleteNodeMessage('node-1')).toEqual({
      type: 'node',
      op: {
        type: 'delete',
        id: 'node-1',
      },
    });
  });
});

describe('v2 image drop filtering', () => {
  it('selects the first image file from dropped files', () => {
    const text = createFile('memo.txt', 'text/plain');
    const png = createFile('image.png', 'image/png');
    const webp = createFile('image.webp', 'image/webp');

    expect(getFirstImageFile([text, png, webp])).toBe(png);
  });

  it('ignores drops without image files', () => {
    expect(getFirstImageFile([createFile('memo.txt', 'text/plain')])).toBeUndefined();
  });
});

describe('v2 image upload processing', () => {
  it('stores inflight preview, persists uploaded image id, and cleans up object url', async () => {
    const { deps, messages, inflight, assets, focus } = createDeps();
    const file = createFile('image.png', 'image/png');
    const revokeObjectUrl = vi.fn();

    const result = await processImageUpload({
      file,
      nodeId: 'node-1',
      getProportion: () => 100,
      ...deps,
      createObjectUrl: () => 'blob:image',
      revokeObjectUrl,
      readImageDimensions: async () => ({ width: 320, height: 240 }),
      uploadImageFile: async () => createAsset('image-1'),
    });

    expect(result).toBe('uploaded');
    expect(assets.get('image-1')).toEqual(createAsset('image-1'));
    expect(messages).toEqual([createSetImageAttrsMessage('node-1', 'image-1', 100)]);
    expect(inflight.get('node-1')).toEqual({ url: 'blob:image', width: 320, height: 240 });
    expect(revokeObjectUrl).not.toHaveBeenCalled();
    expect(focus).toHaveBeenCalled();
  });

  it('cleans up preview and leaves node as empty placeholder when upload fails', async () => {
    const { deps, messages, inflight, focus } = createDeps();
    const file = createFile('image.png', 'image/png');
    const revokeObjectUrl = vi.fn();

    const result = await processImageUpload({
      file,
      nodeId: 'node-1',
      getProportion: () => 100,
      ...deps,
      createObjectUrl: () => 'blob:image',
      revokeObjectUrl,
      readImageDimensions: async () => ({ width: 320, height: 240 }),
      uploadImageFile: async () => {
        throw new Error('upload failed');
      },
    });

    expect(result).toBe('failed');
    expect(messages).toEqual([]);
    expect(inflight.has('node-1')).toBe(false);
    expect(revokeObjectUrl).toHaveBeenCalledWith('blob:image');
    expect(focus).toHaveBeenCalled();
  });
});

describe('v2 image resize calculations', () => {
  it('calculates rendered image size from proportion and original ratio', () => {
    const width = calculateImageWidth(800, 50, 1000);

    expect(width).toBe(400);
    expect(calculateImageHeight(width, 1000, 500)).toBe(200);
  });

  it('does not render wider than the original image', () => {
    expect(calculateImageWidth(800, 100, 320)).toBe(320);
  });
});
