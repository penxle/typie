import { unwrapError } from '$lib/graphql';
import { getSyncConnection } from '$lib/sync';
import {
  abandonBlobUpload,
  finalizeBlobUploadAsFile,
  finalizeBlobUploadAsImage,
  reserveBlobUploads,
  uploadToReservation,
} from '$lib/utils/blob.svelte';
import { SubscribeModal } from '../../../routes/website/(dashboard)/@subscription/subscribe-modal.svelte';
import type { AttachmentImporterDeps } from '../attachment-importer';

export const getImageDimensions = (src: string): Promise<{ width: number; height: number }> => {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.addEventListener('load', () => resolve({ width: img.naturalWidth, height: img.naturalHeight }));
    img.addEventListener('error', () => reject(new Error('Failed to load image')));
    img.src = src;
  });
};

// mearie는 GraphQL 오류를 aggregate로 감싼다. 호출측은 `code` 문자열로 분기하므로 여기서 한 겹 벗겨 준다.
async function unwrapped<T>(run: () => Promise<T>): Promise<T> {
  try {
    return await run();
  } catch (err) {
    const inner = unwrapError(err);
    throw inner instanceof Error ? inner : new Error(String(inner));
  }
}

export const createAttachmentImporterDeps = (): AttachmentImporterDeps => ({
  getImageDimensions,

  // 구독 게이트는 예약(=문서 기록 이전) 진입점에서만 판정한다.
  reserveUploads: (documentId, items) =>
    unwrapped(async () => {
      if (!SubscribeModal.gate('editor_upload')) {
        throw new Error('subscription_required');
      }
      return await reserveBlobUploads(documentId, items);
    }),

  transferToReservation: (reservation, file, signal) => uploadToReservation(reservation, file, signal),

  finalizeImage: (assetId, nonce) =>
    unwrapped(async () => {
      const asset = await finalizeBlobUploadAsImage(assetId, nonce);
      return {
        id: asset.id,
        url: asset.url,
        originalUrl: asset.originalUrl,
        width: asset.width,
        height: asset.height,
        placeholder: asset.placeholder,
      };
    }),

  finalizeFile: (assetId, nonce) =>
    unwrapped(async () => {
      const asset = await finalizeBlobUploadAsFile(assetId, nonce);
      return {
        id: asset.id,
        name: asset.name,
        size: asset.size,
        url: asset.url,
      };
    }),

  abandonUpload: (assetId, nonce) => unwrapped(() => abandonBlobUpload(assetId, nonce)),

  sendAssetFailed: (documentId, items) => getSyncConnection().sendAssetFailed(documentId, items),
  sendAssetHeartbeat: (documentId, items) => getSyncConnection().sendAssetHeartbeat(documentId, items),

  startInterval: (ms, tick) => {
    const id = setInterval(tick, ms);
    return () => clearInterval(id);
  },
  startTimeout: (ms, tick) => {
    const id = setTimeout(tick, ms);
    return () => clearTimeout(id);
  },
  delay: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
});
