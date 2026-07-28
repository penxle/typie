import type { Message } from '@typie/editor-ffi/browser';
import type { EmbedAsset } from '../types';

export const createSetEmbedAttrsMessage = (nodeId: string, embedId: string): Message => ({
  type: 'node',
  op: {
    type: 'set_attrs',
    id: nodeId,
    attrs: { type: 'embed', id: embedId },
  },
});

export const createDeleteEmbedNodeMessage = (nodeId: string): Message => ({
  type: 'node',
  op: { type: 'delete', id: nodeId },
});

export const normalizeEmbedUrl = (raw: string): string => (/^[^:]+:\/\//.test(raw) ? raw : `https://${raw}`);

export const processEmbedUpload = async ({
  url,
  nodeId,
  setPending,
  clearPending,
  isCurrent,
  unfurl,
  setEmbedAsset,
  commit,
}: {
  url: string;
  nodeId: string;
  setPending: () => void;
  clearPending: () => void;
  isCurrent: () => boolean;
  unfurl: (url: string) => Promise<EmbedAsset>;
  setEmbedAsset: (asset: EmbedAsset) => void;
  commit: (message: Message) => boolean;
}): Promise<'uploaded' | 'failed' | 'cancelled'> => {
  setPending();

  try {
    const asset = await unfurl(normalizeEmbedUrl(url));
    if (!isCurrent()) {
      clearPending();
      return 'cancelled';
    }
    setEmbedAsset(asset);
    if (!commit(createSetEmbedAttrsMessage(nodeId, asset.id))) {
      throw new Error('Embed update was rejected');
    }
    clearPending();
    return 'uploaded';
  } catch {
    clearPending();
    return 'failed';
  }
};
