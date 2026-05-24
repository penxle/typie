import { uploadBlobAsFile, uploadBlobAsImage } from '$lib/utils/blob.svelte';
import type { FileAsset, ImageAsset } from '../types';

export const getImageDimensions = (src: string): Promise<{ width: number; height: number }> => {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.addEventListener('load', () => resolve({ width: img.naturalWidth, height: img.naturalHeight }));
    img.addEventListener('error', () => reject(new Error('Failed to load image')));
    img.src = src;
  });
};

export const uploadFileAsFile = async (file: File): Promise<FileAsset> => {
  const uploaded = await uploadBlobAsFile(file);
  return {
    id: uploaded.id,
    name: uploaded.name,
    size: uploaded.size,
    url: uploaded.url,
  };
};

export const uploadImageFile = async (file: File): Promise<ImageAsset> => {
  const uploaded = await uploadBlobAsImage(file);
  return {
    id: uploaded.id,
    url: uploaded.url,
    originalUrl: uploaded.originalUrl,
    width: uploaded.width,
    height: uploaded.height,
    placeholder: uploaded.placeholder,
  };
};
