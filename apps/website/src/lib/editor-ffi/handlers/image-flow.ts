import type { Message } from '@typie/editor-ffi/browser';
import type { EditorContext } from '../editor.svelte';
import type { ImageAsset, ImageStage } from '../types';

export const deriveImageStage = ({
  imageId,
  inflight,
  asset,
}: {
  imageId: string | undefined;
  inflight: { url: string; width: number; height: number } | undefined;
  asset: ImageAsset | undefined;
}): ImageStage => {
  if (asset) return 'ready';
  if (inflight) return 'uploading';
  if (imageId != null && imageId !== '') return 'resolving';
  return 'empty';
};

type UploadImageFile = (file: File) => Promise<ImageAsset>;
type ReadImageDimensions = (src: string) => Promise<{ width: number; height: number }>;

export const setImageAttrsMessage = (nodeId: string, imageId: string | undefined, proportion: number): Message => ({
  type: 'node',
  op: {
    type: 'set_attrs',
    id: nodeId,
    attrs: {
      type: 'image',
      id: imageId,
      proportion: Math.round(proportion),
    },
  },
});

export const deleteNodeMessage = (nodeId: string): Message => ({
  type: 'node',
  op: {
    type: 'delete',
    id: nodeId,
  },
});

const insertEmptyImageMessage = (): Message => ({
  type: 'insertion',
  op: { type: 'fragment', fragment: { node: { type: 'image', id: undefined } } },
});

export const queuePendingImages = (ctx: EditorContext, files: File[]): void => {
  ctx.pendingImageDrops.push(...files);
  let remaining = files.length;
  while (remaining-- > 0) {
    ctx.editor?.enqueue(insertEmptyImageMessage());
  }
};

const imagePicker = (ctx: EditorContext, processFile: (file: File) => void): HTMLInputElement => {
  const picker = document.createElement('input');
  picker.type = 'file';
  picker.accept = 'image/*';
  picker.multiple = true;

  picker.addEventListener('change', () => uploadImages(ctx, processFile, picker.files));

  return picker;
};

const uploadImages = (ctx: EditorContext, processFile: (file: File) => void, files: FileList | null): void => {
  const picked = [...(files ?? [])];
  const first = picked.shift();
  if (first !== undefined) {
    processFile(first);
  }
  queuePendingImages(ctx, picked);
};

export const openImagePicker = (ctx: EditorContext, processFile: (file: File) => void): HTMLInputElement => {
  const picker = imagePicker(ctx, processFile);
  picker.click();

  return picker;
};

export const getFirstImageFile = (files: Iterable<File>): File | undefined => {
  return [...files].find((file) => file.type.startsWith('image/'));
};

export const resolveImageSrc = (asset?: ImageAsset, inflight?: { url: string; width: number; height: number }): string | undefined =>
  asset?.url ?? inflight?.url;

export const calculateImageWidth = (boundsWidth: number, proportion: number, originalWidth: number): number => {
  const proportionalWidth = (boundsWidth * proportion) / 100;
  return originalWidth <= 0 ? proportionalWidth : Math.min(originalWidth, proportionalWidth);
};

export const calculateImageHeight = (width: number, originalWidth: number, originalHeight: number): number => {
  if (originalWidth <= 0) return 0;
  return width * (originalHeight / originalWidth);
};

export const calculateImageContainerSize = ({
  boundsWidth,
  proportion,
  originalWidth,
  originalHeight,
}: {
  boundsWidth: number;
  proportion: number;
  originalWidth: number;
  originalHeight: number;
}): { width: string; height: string | undefined } => {
  if (originalWidth <= 0 || originalHeight <= 0) {
    return { width: '100%', height: undefined };
  }

  const width = calculateImageWidth(boundsWidth, proportion, originalWidth);
  const height = calculateImageHeight(width, originalWidth, originalHeight);
  return { width: `${width}px`, height: `${height}px` };
};

export const processImageUpload = async ({
  file,
  nodeId,
  getProportion,
  setInflightImage,
  deleteInflightImage,
  setImageAsset,
  isCurrent,
  commit,
  focus,
  createObjectUrl,
  revokeObjectUrl,
  readImageDimensions,
  uploadImageFile,
}: {
  file: File;
  nodeId: string;
  getProportion: () => number;
  setInflightImage: (nodeId: string, image: { url: string; width: number; height: number }) => void;
  deleteInflightImage: (nodeId: string) => void;
  setImageAsset: (asset: ImageAsset) => void;
  isCurrent: () => boolean;
  commit: (message: Message) => void;
  focus: () => void;
  createObjectUrl: (file: File) => string;
  revokeObjectUrl: (url: string) => void;
  readImageDimensions: ReadImageDimensions;
  uploadImageFile: UploadImageFile;
}): Promise<'uploaded' | 'failed' | 'cancelled'> => {
  const objectUrl = createObjectUrl(file);
  setInflightImage(nodeId, { url: objectUrl, width: 0, height: 0 });

  const cancel = (): 'cancelled' => {
    deleteInflightImage(nodeId);
    revokeObjectUrl(objectUrl);
    return 'cancelled';
  };

  try {
    const { width, height } = await readImageDimensions(objectUrl);
    if (!isCurrent()) return cancel();
    setInflightImage(nodeId, { url: objectUrl, width, height });

    const uploaded = await uploadImageFile(file);
    if (!isCurrent()) return cancel();
    setImageAsset(uploaded);
    commit(setImageAttrsMessage(nodeId, uploaded.id, getProportion()));
    deleteInflightImage(nodeId);
    revokeObjectUrl(objectUrl);
    focus();

    return 'uploaded';
  } catch {
    if (isCurrent()) {
      try {
        commit(deleteNodeMessage(nodeId));
      } catch {
        // The original commit failure is already represented by the failed result.
      }
    }
    deleteInflightImage(nodeId);
    revokeObjectUrl(objectUrl);
    focus();
    return 'failed';
  }
};
