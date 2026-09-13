import { fitImageSize } from '@typie/lib/image';

export const calculateImageWidth = (
  boundsWidth: number,
  proportion: number,
  originalWidth: number,
  originalHeight: number,
  maxHeight?: number,
): number => {
  const proportionalWidth = (boundsWidth * proportion) / 100;
  if (originalWidth <= 0) return proportionalWidth;
  return fitImageSize({ width: originalWidth, height: originalHeight, maxWidth: boundsWidth, maxHeight, proportion: proportion / 100 })
    .width;
};

export const calculateImageHeight = (width: number, originalWidth: number, originalHeight: number): number => {
  if (originalWidth <= 0) return 0;
  return width * (originalHeight / originalWidth);
};

export const calculateImageSize = ({
  boundsWidth,
  proportion,
  originalWidth,
  originalHeight,
  maxHeight,
}: {
  boundsWidth: number;
  proportion: number;
  originalWidth: number;
  originalHeight: number;
  maxHeight?: number;
}): { width: number; height: number } | undefined => {
  if (originalWidth <= 0 || originalHeight <= 0) {
    return undefined;
  }

  const width = calculateImageWidth(boundsWidth, proportion, originalWidth, originalHeight, maxHeight);
  const height = calculateImageHeight(width, originalWidth, originalHeight);
  return { width, height };
};
