import { describe, expect, it } from 'vitest';
import { deriveImageStage } from './handlers/image-flow';
import type { ImageAsset } from './types';

const asset: ImageAsset = {
  id: 'image-1',
  url: 'https://example.com/image.webp',
  originalUrl: 'https://example.com/original.png',
  width: 640,
  height: 480,
  placeholder: 'placeholder',
};

const inflight = { url: 'blob:preview', width: 640, height: 480 };

describe('deriveImageStage', () => {
  it('returns empty when there is no imageId and no inflight', () => {
    expect(deriveImageStage({ imageId: undefined, inflight: undefined, asset: undefined })).toBe('empty');
  });

  it('returns empty when imageId is null (WASM may send null)', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect(deriveImageStage({ imageId: null as any, inflight: undefined, asset: undefined })).toBe('empty');
  });

  it('returns empty when imageId is empty string', () => {
    expect(deriveImageStage({ imageId: '', inflight: undefined, asset: undefined })).toBe('empty');
  });

  it('returns uploading when inflight exists and asset is not yet settled', () => {
    expect(deriveImageStage({ imageId: undefined, inflight, asset: undefined })).toBe('uploading');
  });

  it('returns resolving when imageId is present but asset is not in cache', () => {
    expect(deriveImageStage({ imageId: 'image-1', inflight: undefined, asset: undefined })).toBe('resolving');
  });

  it('returns ready when asset is in cache', () => {
    expect(deriveImageStage({ imageId: 'image-1', inflight: undefined, asset })).toBe('ready');
  });

  it('returns ready when asset arrives after inflight', () => {
    expect(deriveImageStage({ imageId: 'image-1', inflight, asset })).toBe('ready');
  });

  it('same imageId in multiple nodes shares resolving/ready — derived from imageId-keyed cache', () => {
    const stageA = deriveImageStage({ imageId: 'shared-id', inflight: undefined, asset: undefined });
    const stageB = deriveImageStage({ imageId: 'shared-id', inflight: undefined, asset: undefined });
    expect(stageA).toBe('resolving');
    expect(stageB).toBe('resolving');

    const stageC = deriveImageStage({ imageId: 'shared-id', inflight: undefined, asset });
    const stageD = deriveImageStage({ imageId: 'shared-id', inflight: undefined, asset });
    expect(stageC).toBe('ready');
    expect(stageD).toBe('ready');
  });
});
