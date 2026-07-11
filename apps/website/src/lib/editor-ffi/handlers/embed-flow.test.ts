import { describe, expect, it } from 'vitest';
import { createDeleteEmbedNodeMessage, createSetEmbedAttrsMessage, normalizeEmbedUrl, processEmbedUpload } from './embed-flow';
import type { EmbedAsset } from '../types';

describe('embed flow messages', () => {
  it('creates a set_attrs message with embed type and id', () => {
    expect(createSetEmbedAttrsMessage('node-1', 'embed-1')).toEqual({
      type: 'node',
      op: {
        type: 'set_attrs',
        id: 'node-1',
        attrs: { type: 'embed', id: 'embed-1' },
      },
    });
  });

  it('creates a delete message for a node', () => {
    expect(createDeleteEmbedNodeMessage('node-1')).toEqual({
      type: 'node',
      op: {
        type: 'delete',
        id: 'node-1',
      },
    });
  });
});

describe('URL normalization', () => {
  it('adds https:// prefix when scheme is missing', () => {
    expect(normalizeEmbedUrl('example.com')).toBe('https://example.com');
  });

  it('preserves https:// URLs as-is', () => {
    expect(normalizeEmbedUrl('https://example.com')).toBe('https://example.com');
  });

  it('preserves http:// URLs as-is', () => {
    expect(normalizeEmbedUrl('http://example.com')).toBe('http://example.com');
  });

  it('preserves URLs with other schemes as-is', () => {
    expect(normalizeEmbedUrl('ftp://example.com')).toBe('ftp://example.com');
  });
});

const asset: EmbedAsset = {
  id: 'embed-1',
  url: 'https://example.com',
  title: 'Example',
  description: null,
  thumbnailUrl: null,
  html: null,
};

describe('embed upload processing', () => {
  it('keeps pending state through local commit and clears it afterwards', async () => {
    let pending = false;
    let commitSawPending = false;
    const assets = new Map<string, EmbedAsset>();

    const result = await processEmbedUpload({
      url: 'example.com',
      nodeId: 'node-1',
      setPending: () => (pending = true),
      clearPending: () => (pending = false),
      isCurrent: () => pending,
      unfurl: async () => asset,
      setEmbedAsset: (value) => assets.set(value.id, value),
      commit: (message) => {
        commitSawPending = pending;
        expect(message).toEqual(createSetEmbedAttrsMessage('node-1', asset.id));
      },
    });

    expect(result).toBe('uploaded');
    expect(commitSawPending).toBe(true);
    expect(pending).toBe(false);
    expect(assets.get(asset.id)).toEqual(asset);
  });

  it('does not cache or commit an unfurl that is no longer current', async () => {
    const { promise, resolve } = Promise.withResolvers<EmbedAsset>();
    let pending = false;
    let current = true;
    let committed = false;
    const assets = new Map<string, EmbedAsset>();

    const upload = processEmbedUpload({
      url: 'example.com',
      nodeId: 'node-1',
      setPending: () => (pending = true),
      clearPending: () => (pending = false),
      isCurrent: () => current,
      unfurl: () => promise,
      setEmbedAsset: (value) => assets.set(value.id, value),
      commit: () => (committed = true),
    });

    current = false;
    resolve(asset);

    await expect(upload).resolves.toBe('cancelled');
    expect(pending).toBe(false);
    expect(assets.size).toBe(0);
    expect(committed).toBe(false);
  });
});
