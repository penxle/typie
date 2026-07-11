import { describe, expect, it, vi } from 'vitest';
import {
  calculateImageContainerSize,
  calculateImageHeight,
  calculateImageWidth,
  deleteNodeMessage,
  getFirstImageFile,
  openImagePicker,
  processImageUpload,
  queuePendingImages,
  resolveImageSrc,
  setImageAttrsMessage,
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
  let commitSawInflight = false;

  return {
    deps: {
      setInflightImage: (nodeId: string, image: { url: string; width: number; height: number }) => inflight.set(nodeId, image),
      deleteInflightImage: (nodeId: string) => inflight.delete(nodeId),
      setImageAsset: (asset: ImageAsset) => assets.set(asset.id, asset),
      isCurrent: () => true,
      commit: (message: Message) => {
        commitSawInflight = inflight.has('node-1');
        messages.push(message);
      },
      focus,
    },
    messages,
    inflight,
    assets,
    focus,
    commitSawInflight: () => commitSawInflight,
  };
};

describe('이미지 표시 여부 결정', () => {
  const inflight = { url: 'blob:preview', width: 320, height: 240 };
  const asset = createAsset();

  it('업로드 중에는 inflight 미리보기 이미지를 표시한다', () => {
    expect(resolveImageSrc(undefined, inflight)).toBe('blob:preview');
  });

  it('업로드가 완료되면 inflight 대신 asset 이미지를 표시한다', () => {
    expect(resolveImageSrc(asset, inflight)).toBe(asset.url);
  });

  it('이미지가 없으면 표시하지 않는다', () => {
    expect(resolveImageSrc()).toBeUndefined();
  });
});

describe('v2 image flow messages', () => {
  it('creates a v2 node attrs message for uploaded image id and rounded proportion', () => {
    expect(setImageAttrsMessage('node-1', 'image-1', 74.6)).toEqual({
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
    expect(deleteNodeMessage('node-1')).toEqual({
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

const createCtx = () => {
  const messages: Message[] = [];
  const pendingImageDrops: File[] = [];
  const editor = {
    enqueue: (message: Message) => {
      messages.push(message);
    },
  };

  return {
    ctx: { editor, pendingImageDrops } as never,
    messages,
    pendingImageDrops,
  };
};

describe('v2 pending image queueing', () => {
  it('각 파일을 pending 큐에 넣고 빈 이미지 노드 삽입 메시지를 보낸다', () => {
    const { ctx, messages, pendingImageDrops } = createCtx();
    const first = createFile('first.png', 'image/png');
    const second = createFile('second.png', 'image/png');

    queuePendingImages(ctx, [first, second]);

    expect(pendingImageDrops).toEqual([first, second]);
    expect(messages).toEqual([
      {
        type: 'insertion',
        op: { type: 'fragment', fragment: { node: { type: 'image', id: undefined } } },
      },
      {
        type: 'insertion',
        op: { type: 'fragment', fragment: { node: { type: 'image', id: undefined } } },
      },
    ]);
  });

  it('파일이 없으면 아무것도 하지 않는다', () => {
    const { ctx, messages, pendingImageDrops } = createCtx();

    queuePendingImages(ctx, []);

    expect(pendingImageDrops).toEqual([]);
    expect(messages).toEqual([]);
  });
});

describe('v2 image picker flow', () => {
  const setPickerFiles = (picker: HTMLInputElement, files: File[]) => {
    Object.defineProperty(picker, 'files', { value: files });
  };

  it('파일 선택을 취소하면 노드를 유지한다', () => {
    const { ctx, messages } = createCtx();
    const processFile = vi.fn();

    const picker = openImagePicker(ctx, processFile);
    picker.dispatchEvent(new Event('cancel'));

    expect(processFile).not.toHaveBeenCalled();
    expect(messages).toEqual([]);
  });

  it('파일 없이 선택을 마치면 노드를 유지한다', () => {
    const { ctx, messages } = createCtx();
    const processFile = vi.fn();

    const picker = openImagePicker(ctx, processFile);
    setPickerFiles(picker, []);
    picker.dispatchEvent(new Event('change'));

    expect(processFile).not.toHaveBeenCalled();
    expect(messages).toEqual([]);
  });

  it('첫 파일은 현재 노드에 업로드하고 나머지는 새 이미지 노드로 삽입한다', () => {
    const { ctx, messages, pendingImageDrops } = createCtx();
    const processFile = vi.fn();
    const first = createFile('first.png', 'image/png');
    const second = createFile('second.png', 'image/png');

    const picker = openImagePicker(ctx, processFile);
    setPickerFiles(picker, [first, second]);
    picker.dispatchEvent(new Event('change'));

    expect(processFile).toHaveBeenCalledExactlyOnceWith(first);
    expect(pendingImageDrops).toEqual([second]);
    expect(messages).toEqual([
      {
        type: 'insertion',
        op: { type: 'fragment', fragment: { node: { type: 'image', id: undefined } } },
      },
    ]);
  });
});

describe('v2 image upload processing', () => {
  it('keeps the preview pending through local commit and clears it afterwards', async () => {
    const { deps, messages, inflight, assets, focus, commitSawInflight } = createDeps();
    const revokeObjectUrl = vi.fn();

    const result = await processImageUpload({
      file: createFile('image.png', 'image/png'),
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
    expect(messages).toEqual([setImageAttrsMessage('node-1', 'image-1', 100)]);
    expect(commitSawInflight()).toBe(true);
    expect(inflight.has('node-1')).toBe(false);
    expect(revokeObjectUrl).toHaveBeenCalledWith('blob:image');
    expect(focus).toHaveBeenCalled();
  });

  it('does not cache or commit an upload that is no longer current', async () => {
    const { deps, messages, inflight, assets } = createDeps();
    const revokeObjectUrl = vi.fn();
    const { promise, resolve } = Promise.withResolvers<ImageAsset>();
    let current = true;

    const upload = processImageUpload({
      file: createFile('image.png', 'image/png'),
      nodeId: 'node-1',
      getProportion: () => 100,
      ...deps,
      isCurrent: () => current,
      createObjectUrl: () => 'blob:image',
      revokeObjectUrl,
      readImageDimensions: async () => ({ width: 320, height: 240 }),
      uploadImageFile: () => promise,
    });

    current = false;
    resolve(createAsset('image-1'));

    await expect(upload).resolves.toBe('cancelled');
    expect(assets.size).toBe(0);
    expect(messages).toEqual([]);
    expect(inflight.has('node-1')).toBe(false);
    expect(revokeObjectUrl).toHaveBeenCalledWith('blob:image');
  });

  it('업로드가 실패하면 미리보기를 정리하고 빈 노드를 삭제한다', async () => {
    const { deps, messages, inflight, focus } = createDeps();
    const revokeObjectUrl = vi.fn();

    const result = await processImageUpload({
      file: createFile('image.png', 'image/png'),
      nodeId: 'node-1',
      getProportion: () => 100,
      ...deps,
      isCurrent: () => inflight.has('node-1'),
      createObjectUrl: () => 'blob:image',
      revokeObjectUrl,
      readImageDimensions: async () => ({ width: 320, height: 240 }),
      uploadImageFile: async () => {
        throw new Error('upload failed');
      },
    });

    expect(result).toBe('failed');
    expect(messages).toEqual([deleteNodeMessage('node-1')]);
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

  it('이미지 치수를 알게 된 뒤에는 업로드 중과 완료 후에 같은 폭 제한 정책을 사용한다', () => {
    const uploading = calculateImageContainerSize({
      boundsWidth: 800,
      proportion: 100,
      originalWidth: 320,
      originalHeight: 240,
    });
    const ready = calculateImageContainerSize({
      boundsWidth: 800,
      proportion: 100,
      originalWidth: 320,
      originalHeight: 240,
    });

    expect(uploading).toEqual({ width: '320px', height: '240px' });
    expect(ready).toEqual(uploading);
  });

  it('이미지 치수를 아직 모를 때만 전체 폭으로 폴백한다', () => {
    expect(
      calculateImageContainerSize({
        boundsWidth: 800,
        proportion: 100,
        originalWidth: 0,
        originalHeight: 0,
      }),
    ).toEqual({ width: '100%', height: undefined });
  });
});
