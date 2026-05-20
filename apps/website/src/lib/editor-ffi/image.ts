import type { ExternalElementData, Message } from '@typie/editor-ffi/browser';

export const IMAGE_PROPORTION_MAX = 100;
export const IMAGE_PROPORTION_MIN = 1;

export const getExternalElementPlaceholderLabel = (data: ExternalElementData): string | null => {
  switch (data.type) {
    case 'image': {
      return null;
    }
    case 'file': {
      return '파일';
    }
    case 'embed': {
      return '임베드';
    }
    case 'archived': {
      return '보관된 블록';
    }
  }
};

export const proportionToScale = (proportion: number): number => {
  if (!Number.isFinite(proportion) || proportion <= 0) {
    return 1;
  }
  if (proportion >= IMAGE_PROPORTION_MAX) {
    return 1;
  }
  return proportion / IMAGE_PROPORTION_MAX;
};

const clampProportion = (proportion: number): number => {
  if (!Number.isFinite(proportion)) {
    return IMAGE_PROPORTION_MAX;
  }
  const rounded = Math.round(proportion);
  return Math.max(IMAGE_PROPORTION_MIN, Math.min(IMAGE_PROPORTION_MAX, rounded));
};

type SetImageAttrsArgs = {
  nodeId: string;
  currentId?: string;
  currentProportion: number;
  nextId?: string;
  nextProportion?: number;
};

export const createSetImageAttrsMessage = ({
  nodeId,
  currentId,
  currentProportion,
  nextId,
  nextProportion,
}: SetImageAttrsArgs): Message => ({
  type: 'node',
  op: {
    type: 'set_attrs',
    id: nodeId,
    attrs: {
      type: 'image',
      id: nextId ?? currentId,
      proportion: clampProportion(nextProportion ?? currentProportion),
    },
  },
});

export const createDeleteImageMessage = (nodeId: string): Message => ({
  type: 'node',
  op: { type: 'delete', id: nodeId },
});

export const createDropImageSelectionMessages = (local: { page: number; x: number; y: number } | null): Message[] => {
  if (!local) {
    return [];
  }

  return [
    { type: 'pointer', event: { type: 'down', page: local.page, x: local.x, y: local.y, count: 1 } },
    { type: 'pointer', event: { type: 'up' } },
  ];
};

const getImageWidthBounds = (originalWidth: number, boundsWidth: number) => {
  const maxWidth = Math.min(originalWidth > 0 ? originalWidth : boundsWidth, boundsWidth);
  const minWidth = Math.max(boundsWidth * 0.1, 100);
  return { minWidth, maxWidth };
};

export const clampImageWidth = (width: number, originalWidth: number, boundsWidth: number): number => {
  const { minWidth, maxWidth } = getImageWidthBounds(originalWidth, boundsWidth);
  return Math.max(minWidth, Math.min(maxWidth, width));
};

type ImagePresentationInput = {
  proportion: number;
  boundsWidth: number;
  imageId: string | undefined;
  asset: { id?: string; url: string; originalUrl?: string; width: number; height: number; placeholder?: string | null } | undefined;
  inflight: { url: string; width: number; height: number } | undefined;
};

export type ImagePresentation = {
  hasImage: boolean;
  isUploading: boolean;
  isResolvingAsset: boolean;
  url: string | undefined;
  placeholder: string | null | undefined;
  width: number;
  height: number;
  originalWidth: number;
  originalHeight: number;
};

export const computeImagePresentation = ({
  proportion,
  boundsWidth,
  imageId,
  asset,
  inflight,
}: ImagePresentationInput): ImagePresentation => {
  const url = asset?.url ?? inflight?.url;
  const hasImage = !!url;
  const isUploading = !!inflight && !asset;
  const isResolvingAsset = !!imageId && !asset && !inflight;

  const originalWidth = asset?.width ?? inflight?.width ?? 0;
  const originalHeight = asset?.height ?? inflight?.height ?? 0;
  const scale = proportionToScale(proportion);

  let width = 0;
  let height = 0;
  if (hasImage && boundsWidth > 0) {
    const targetWidth = boundsWidth * scale;
    width = originalWidth > 0 ? Math.min(originalWidth, targetWidth) : targetWidth;
    height = originalWidth > 0 ? (width * originalHeight) / originalWidth : 0;
  }

  return {
    hasImage,
    isUploading,
    isResolvingAsset,
    url,
    placeholder: asset && 'placeholder' in asset ? (asset.placeholder ?? null) : undefined,
    width,
    height,
    originalWidth,
    originalHeight,
  };
};

type ResolveResizeArgs = {
  boundsWidth: number;
  originalWidth: number;
  initialWidth: number;
  initialClientX: number;
  nextClientX: number;
  reverse: boolean;
};

type UploadImageResult = {
  id: string;
  url: string;
  originalUrl: string;
  width: number;
  height: number;
  placeholder?: string | null;
};

type ProcessImageUploadEditor = {
  inflightImages: Map<string, { url: string; width: number; height: number }>;
  imageAssets: Map<string, UploadImageResult>;
  enqueue: (message: Message) => void;
  focus: () => void;
};

type ProcessImageUploadArgs = {
  file: File;
  nodeId: string;
  currentId?: string;
  currentProportion: number;
  editor: ProcessImageUploadEditor;
  getImageDimensions: (src: string) => Promise<{ width: number; height: number }>;
  uploadImage: (file: File) => Promise<UploadImageResult>;
  createObjectUrl: (file: File) => string;
  revokeObjectUrl: (url: string) => void;
  onFailure?: (error: unknown) => void;
};

export const processImageUpload = async ({
  file,
  nodeId,
  currentId,
  currentProportion,
  editor,
  getImageDimensions,
  uploadImage,
  createObjectUrl,
  revokeObjectUrl,
  onFailure,
}: ProcessImageUploadArgs): Promise<{ ok: true; uploadedImage: UploadImageResult } | { ok: false; error: unknown }> => {
  const objectUrl = createObjectUrl(file);

  try {
    const { width, height } = await getImageDimensions(objectUrl);
    editor.inflightImages.set(nodeId, { url: objectUrl, width, height });

    const uploadedImage = await uploadImage(file);
    if (!editor.inflightImages.has(nodeId)) {
      return { ok: false, error: new Error('Upload cancelled') };
    }
    editor.imageAssets.set(uploadedImage.id, uploadedImage);
    editor.enqueue(
      createSetImageAttrsMessage({
        nodeId,
        currentId,
        currentProportion,
        nextId: uploadedImage.id,
      }),
    );
    editor.focus();

    return { ok: true, uploadedImage };
  } catch (err) {
    editor.enqueue(createDeleteImageMessage(nodeId));
    onFailure?.(err);
    return { ok: false, error: err };
  } finally {
    editor.inflightImages.delete(nodeId);
    revokeObjectUrl(objectUrl);
  }
};

export const resolveResizedImageProportion = ({
  boundsWidth,
  originalWidth,
  initialWidth,
  initialClientX,
  nextClientX,
  reverse,
}: ResolveResizeArgs): { width: number; proportion: number } => {
  if (boundsWidth <= 0) {
    return { width: initialWidth, proportion: 0 };
  }

  const dx = (nextClientX - initialClientX) * (reverse ? -1 : 1);
  const width = clampImageWidth(initialWidth + dx * 2, originalWidth, boundsWidth);
  return { width, proportion: (width / boundsWidth) * IMAGE_PROPORTION_MAX };
};
