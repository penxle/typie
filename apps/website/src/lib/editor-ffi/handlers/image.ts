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
