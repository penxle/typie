import { describe, expect, it, vi } from 'vitest';
import { createDeleteNodeMessage, createSetFileAttrsMessage, processFileUpload } from './file-flow';
import type { Message } from '@typie/editor-ffi/browser';
import type { FileAsset } from '../types';

const createFile = (name: string, type: string, size = 1024) => new File([new ArrayBuffer(size)], name, { type });

const createAsset = (id = 'file-1'): FileAsset => ({
  id,
  name: 'document.pdf',
  size: '204800',
  url: 'https://example.com/document.pdf',
});

const createDeps = () => {
  const messages: Message[] = [];
  const inflight = new Map<string, { name: string; size: number }>();
  const assets = new Map<string, FileAsset>();
  const focus = vi.fn();

  return {
    deps: {
      setInflightFile: (nodeId: string, data: { name: string; size: number }) => inflight.set(nodeId, data),
      deleteInflightFile: (nodeId: string) => inflight.delete(nodeId),
      setFileAsset: (asset: FileAsset) => assets.set(asset.id, asset),
      enqueue: (message: Message) => {
        messages.push(message);
      },
      focus,
    },
    messages,
    inflight,
    assets,
    focus,
  };
};

describe('v2 file flow messages', () => {
  it('creates a set_attrs message with file type and id', () => {
    expect(createSetFileAttrsMessage('node-1', 'file-1')).toEqual({
      type: 'node',
      op: {
        type: 'set_attrs',
        id: 'node-1',
        attrs: {
          type: 'file',
          id: 'file-1',
        },
      },
    });
  });

  it('creates a delete message for node cleanup', () => {
    expect(createDeleteNodeMessage('node-1')).toEqual({
      type: 'node',
      op: {
        type: 'delete',
        id: 'node-1',
      },
    });
  });
});

describe('v2 file upload processing', () => {
  it('stores inflight state, persists uploaded file id, and calls focus', async () => {
    const { deps, messages, inflight, assets, focus } = createDeps();
    const file = createFile('document.pdf', 'application/pdf', 204_800);
    const asset = createAsset('file-1');

    const result = await processFileUpload({
      file,
      nodeId: 'node-1',
      ...deps,
      uploadFileAsFile: async () => asset,
    });

    expect(result).toBe('uploaded');
    expect(inflight.has('node-1')).toBe(false);
    expect(assets.get('file-1')).toEqual(asset);
    expect(messages).toEqual([createSetFileAttrsMessage('node-1', 'file-1')]);
    expect(focus).toHaveBeenCalledOnce();
  });

  it('stores inflight with original file name and size before upload resolves', async () => {
    const { deps, inflight } = createDeps();
    const { promise: uploadResult, resolve: resolveUpload } = Promise.withResolvers<FileAsset>();

    const uploadPromise = processFileUpload({
      file: createFile('report.xlsx', 'application/octet-stream', 51_200),
      nodeId: 'node-2',
      ...deps,
      uploadFileAsFile: () => uploadResult,
    });

    expect(inflight.get('node-2')).toEqual({ name: 'report.xlsx', size: 51_200 });

    resolveUpload(createAsset('file-2'));
    await uploadPromise;
  });

  it('cleans up inflight state and leaves node as empty placeholder when upload fails', async () => {
    const { deps, messages, inflight, focus } = createDeps();
    const file = createFile('document.pdf', 'application/pdf');

    const result = await processFileUpload({
      file,
      nodeId: 'node-1',
      ...deps,
      uploadFileAsFile: async () => {
        throw new Error('upload failed');
      },
    });

    expect(result).toBe('failed');
    expect(inflight.has('node-1')).toBe(false);
    expect(messages).toEqual([]);
    expect(focus).toHaveBeenCalledOnce();
  });

  it('does not set file asset when upload fails', async () => {
    const { deps, assets } = createDeps();

    await processFileUpload({
      file: createFile('document.pdf', 'application/pdf'),
      nodeId: 'node-1',
      ...deps,
      uploadFileAsFile: async () => {
        throw new Error('network error');
      },
    });

    expect(assets.size).toBe(0);
  });

  it('handles multiple concurrent uploads independently', async () => {
    const { deps, messages, inflight, assets } = createDeps();

    const [result1, result2] = await Promise.all([
      processFileUpload({
        file: createFile('a.pdf', 'application/pdf'),
        nodeId: 'node-a',
        ...deps,
        uploadFileAsFile: async () => ({ id: 'file-a', name: 'a.pdf', size: '1024', url: 'https://example.com/a.pdf' }),
      }),
      processFileUpload({
        file: createFile('b.pdf', 'application/pdf'),
        nodeId: 'node-b',
        ...deps,
        uploadFileAsFile: async () => ({ id: 'file-b', name: 'b.pdf', size: '2048', url: 'https://example.com/b.pdf' }),
      }),
    ]);

    expect(result1).toBe('uploaded');
    expect(result2).toBe('uploaded');
    expect(assets.has('file-a')).toBe(true);
    expect(assets.has('file-b')).toBe(true);
    expect(inflight.has('node-a')).toBe(false);
    expect(inflight.has('node-b')).toBe(false);
    expect(messages).toHaveLength(2);
  });

  it('first upload success does not affect second upload failure', async () => {
    const { deps, messages } = createDeps();

    const [result1, result2] = await Promise.all([
      processFileUpload({
        file: createFile('a.pdf', 'application/pdf'),
        nodeId: 'node-a',
        ...deps,
        uploadFileAsFile: async () => createAsset('file-a'),
      }),
      processFileUpload({
        file: createFile('b.pdf', 'application/pdf'),
        nodeId: 'node-b',
        ...deps,
        uploadFileAsFile: async () => {
          throw new Error('failed');
        },
      }),
    ]);

    expect(result1).toBe('uploaded');
    expect(result2).toBe('failed');
    expect(messages).toContainEqual(createSetFileAttrsMessage('node-a', 'file-a'));
    expect(messages).not.toContainEqual(createDeleteNodeMessage('node-b'));
  });
});
