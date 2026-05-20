import { describe, expect, it, vi } from 'vitest';
import {
  clampImageWidth,
  computeImagePresentation,
  createDeleteImageMessage,
  createDropImageSelectionMessages,
  createSetImageAttrsMessage,
  getExternalElementPlaceholderLabel,
  IMAGE_PROPORTION_MAX,
  processImageUpload,
  proportionToScale,
  resolveResizedImageProportion,
} from './image';

describe('getExternalElementPlaceholderLabel', () => {
  it('returns null for image and labels for non-image placeholders', () => {
    expect(getExternalElementPlaceholderLabel({ type: 'image', id: undefined, proportion: 100 })).toBeNull();
    expect(getExternalElementPlaceholderLabel({ type: 'file', id: undefined })).toBe('파일');
    expect(getExternalElementPlaceholderLabel({ type: 'embed', id: undefined })).toBe('임베드');
    expect(getExternalElementPlaceholderLabel({ type: 'archived', id: undefined })).toBe('보관된 블록');
  });
});

describe('proportionToScale', () => {
  it('converts FFI percentage proportion (1-100) to a 0-1 scale', () => {
    expect(proportionToScale(100)).toBe(1);
    expect(proportionToScale(50)).toBe(0.5);
  });

  it('treats invalid proportion as full width', () => {
    expect(proportionToScale(0)).toBe(1);
    expect(proportionToScale(-10)).toBe(1);
    expect(proportionToScale(Number.NaN)).toBe(1);
  });

  it('clamps proportion above max to 1', () => {
    expect(proportionToScale(IMAGE_PROPORTION_MAX + 50)).toBe(1);
  });
});

describe('createDropImageSelectionMessages', () => {
  it('creates pointer down/up messages so the drop position becomes the insertion point', () => {
    expect(createDropImageSelectionMessages({ page: 2, x: 10, y: 20 })).toEqual([
      { type: 'pointer', event: { type: 'down', page: 2, x: 10, y: 20, count: 1 } },
      { type: 'pointer', event: { type: 'up' } },
    ]);
  });

  it('returns no messages when drop coordinates are unavailable', () => {
    expect(createDropImageSelectionMessages(null)).toEqual([]);
  });
});

describe('createSetImageAttrsMessage', () => {
  it('emits a node set_attrs message carrying the new image id while preserving proportion', () => {
    expect(
      createSetImageAttrsMessage({
        nodeId: 'node-1',
        currentId: undefined,
        currentProportion: 100,
        nextId: 'image-1',
      }),
    ).toEqual({
      type: 'node',
      op: {
        type: 'set_attrs',
        id: 'node-1',
        attrs: { type: 'image', id: 'image-1', proportion: 100 },
      },
    });
  });

  it('rounds fractional proportion to the FFI u32 percent unit', () => {
    expect(
      createSetImageAttrsMessage({
        nodeId: 'node-1',
        currentId: 'image-1',
        currentProportion: 100,
        nextProportion: 62.4,
      }),
    ).toMatchObject({
      type: 'node',
      op: {
        attrs: { type: 'image', id: 'image-1', proportion: 62 },
      },
    });
  });

  it('clamps proportion to the 1..100 range', () => {
    expect(
      createSetImageAttrsMessage({
        nodeId: 'node-1',
        currentId: 'image-1',
        currentProportion: 100,
        nextProportion: 250,
      }),
    ).toMatchObject({
      type: 'node',
      op: { attrs: { proportion: 100 } },
    });

    expect(
      createSetImageAttrsMessage({
        nodeId: 'node-1',
        currentId: 'image-1',
        currentProportion: 100,
        nextProportion: 0,
      }),
    ).toMatchObject({
      type: 'node',
      op: { attrs: { proportion: 1 } },
    });
  });
});

describe('createDeleteImageMessage', () => {
  it('creates a delete node message for empty or failed image nodes', () => {
    expect(createDeleteImageMessage('node-1')).toEqual({
      type: 'node',
      op: { type: 'delete', id: 'node-1' },
    });
  });
});

describe('clampImageWidth', () => {
  it('enforces both the minimum (max(10%, 100px)) and maximum (originalWidth or bounds)', () => {
    expect(clampImageWidth(30, 1200, 800)).toBe(100);
    expect(clampImageWidth(2000, 1200, 800)).toBe(800);
    expect(clampImageWidth(2000, 500, 800)).toBe(500);
  });
});

describe('computeImagePresentation', () => {
  it('reports inflight preview when only an inflight image is present', () => {
    const result = computeImagePresentation({
      proportion: 100,
      boundsWidth: 800,
      imageId: undefined,
      asset: undefined,
      inflight: { url: 'blob:preview', width: 1600, height: 1200 },
    });
    expect(result).toMatchObject({
      hasImage: true,
      isUploading: true,
      isResolvingAsset: false,
      url: 'blob:preview',
      width: 800,
      height: 600,
    });
  });

  it('reports the uploaded asset once it becomes available, even if inflight still lingers', () => {
    const result = computeImagePresentation({
      proportion: 100,
      boundsWidth: 800,
      imageId: 'image-1',
      asset: { id: 'image-1', url: 'https://cdn/asset.webp', originalUrl: 'https://cdn/raw.png', width: 1600, height: 1200 },
      inflight: { url: 'blob:preview', width: 1600, height: 1200 },
    });
    expect(result).toMatchObject({
      hasImage: true,
      isUploading: false,
      isResolvingAsset: false,
      url: 'https://cdn/asset.webp',
    });
  });

  it('reports resolving state when an image id exists but neither asset nor inflight is loaded yet', () => {
    const result = computeImagePresentation({
      proportion: 100,
      boundsWidth: 800,
      imageId: 'image-1',
      asset: undefined,
      inflight: undefined,
    });
    expect(result).toMatchObject({
      hasImage: false,
      isUploading: false,
      isResolvingAsset: true,
    });
  });

  it('uses the proportion scale to compute width and preserves aspect ratio', () => {
    const result = computeImagePresentation({
      proportion: 50,
      boundsWidth: 800,
      imageId: 'image-1',
      asset: { id: 'image-1', url: 'https://cdn/asset.webp', originalUrl: 'https://cdn/raw.png', width: 1000, height: 500 },
      inflight: undefined,
    });
    expect(result.width).toBe(400);
    expect(result.height).toBe(200);
  });

  it('reports empty state when no image id and no inflight exist', () => {
    const result = computeImagePresentation({
      proportion: 100,
      boundsWidth: 800,
      imageId: undefined,
      asset: undefined,
      inflight: undefined,
    });
    expect(result).toMatchObject({ hasImage: false, isUploading: false, isResolvingAsset: false });
  });
});

describe('processImageUpload', () => {
  const makeEditor = () => ({
    inflightImages: new Map<string, { url: string; width: number; height: number }>(),
    imageAssets: new Map<
      string,
      { id: string; url: string; originalUrl: string; width: number; height: number; placeholder?: string | null }
    >(),
    enqueue: vi.fn(),
    focus: vi.fn(),
  });

  const file = new File(['x'], 'image.png', { type: 'image/png' });

  it('stores the uploaded asset and dispatches set_attrs with the new id', async () => {
    const editor = makeEditor();

    const result = await processImageUpload({
      file,
      nodeId: 'node-1',
      currentProportion: 100,
      editor,
      getImageDimensions: async () => ({ width: 640, height: 480 }),
      uploadImage: async () => ({
        id: 'image-1',
        url: 'https://cdn/asset.webp',
        originalUrl: 'https://cdn/raw.png',
        width: 640,
        height: 480,
        placeholder: 'placeholder',
      }),
      createObjectUrl: () => 'blob:preview',
      revokeObjectUrl: vi.fn(),
    });

    expect(result.ok).toBe(true);
    expect(editor.imageAssets.get('image-1')).toMatchObject({ id: 'image-1', url: 'https://cdn/asset.webp' });
    expect(editor.enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'node',
        op: expect.objectContaining({
          type: 'set_attrs',
          id: 'node-1',
          attrs: expect.objectContaining({ type: 'image', id: 'image-1', proportion: 100 }),
        }),
      }),
    );
    expect(editor.inflightImages.size).toBe(0);
  });

  it('sets an inflight preview before the upload resolves and clears it after success', async () => {
    const editor = makeEditor();
    const revokeObjectUrl = vi.fn();

    let resolveUpload!: (value: {
      id: string;
      url: string;
      originalUrl: string;
      width: number;
      height: number;
      placeholder?: string | null;
    }) => void;
    const uploadPromise = new Promise<{
      id: string;
      url: string;
      originalUrl: string;
      width: number;
      height: number;
      placeholder?: string | null;
    }>((resolve) => {
      resolveUpload = resolve;
    });

    const runner = processImageUpload({
      file,
      nodeId: 'node-1',
      currentProportion: 100,
      editor,
      getImageDimensions: async () => ({ width: 200, height: 100 }),
      uploadImage: () => uploadPromise,
      createObjectUrl: () => 'blob:preview',
      revokeObjectUrl,
    });

    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(editor.inflightImages.get('node-1')).toEqual({ url: 'blob:preview', width: 200, height: 100 });

    resolveUpload({
      id: 'image-1',
      url: 'https://cdn/a.webp',
      originalUrl: 'https://cdn/raw.png',
      width: 200,
      height: 100,
      placeholder: null,
    });
    await runner;

    expect(editor.inflightImages.size).toBe(0);
    expect(revokeObjectUrl).toHaveBeenCalledWith('blob:preview');
  });

  it('deletes the placeholder node and reports failure on upload error', async () => {
    const editor = makeEditor();
    const onFailure = vi.fn();

    const result = await processImageUpload({
      file,
      nodeId: 'node-1',
      currentProportion: 100,
      editor,
      getImageDimensions: async () => ({ width: 640, height: 480 }),
      uploadImage: async () => {
        throw new Error('upload failed');
      },
      createObjectUrl: () => 'blob:preview',
      revokeObjectUrl: vi.fn(),
      onFailure,
    });

    expect(result.ok).toBe(false);
    expect(editor.enqueue).toHaveBeenCalledWith({ type: 'node', op: { type: 'delete', id: 'node-1' } });
    expect(onFailure).toHaveBeenCalledTimes(1);
    expect(editor.inflightImages.size).toBe(0);
  });
});

describe('resolveResizedImageProportion', () => {
  it('returns the new width as an integer percent of bounds', () => {
    expect(
      resolveResizedImageProportion({
        boundsWidth: 800,
        originalWidth: 1200,
        initialWidth: 400,
        initialClientX: 100,
        nextClientX: 150,
        reverse: false,
      }),
    ).toEqual({ width: 500, proportion: 62.5 });
  });

  it('handles reverse handle by inverting the delta direction', () => {
    expect(
      resolveResizedImageProportion({
        boundsWidth: 800,
        originalWidth: 1200,
        initialWidth: 400,
        initialClientX: 100,
        nextClientX: 50,
        reverse: true,
      }),
    ).toEqual({ width: 500, proportion: 62.5 });
  });

  it('returns proportion 0 when bounds collapse', () => {
    expect(
      resolveResizedImageProportion({
        boundsWidth: 0,
        originalWidth: 1200,
        initialWidth: 400,
        initialClientX: 100,
        nextClientX: 200,
        reverse: false,
      }),
    ).toEqual({ width: 400, proportion: 0 });
  });
});
